use crate::domain::{Command, CommandType, Cpu, ProcessControlBlock, ProcessState};
use crate::services::{Config, Scheduler};
use rand::rngs::StdRng;
use rand::{Rng, SeedableRng};
/// Ядро учебной модели ОС.
///
/// Лабораторная работа 4: реализовано выполнение команд процесса по
/// фон-Неймановской схеме (чтение команды, разборка, чтение операндов,
/// выполнение операции, запись результата), инициализация ввода-вывода
/// с блокировкой процесса и завершение задания с освобождением памяти
/// и загрузкой новых заданий.
pub struct SimulationModel {
    rng: StdRng,
    process_table: Vec<Option<ProcessControlBlock>>,
    next_task_id: i32,
    total_memory: i32,
    model_tick: u64,
    speed_ticks_per_second: f64,
    pub cpus: Vec<Cpu>,
    pub scheduler: Scheduler,
    pub new_task: Option<ProcessControlBlock>,

    // Параметры заданий из конфигурации.
    task_size_min: i32,
    task_size_max: i32,
    commands_min: u64,
    commands_max: u64,
    io_percent_min: u32,
    io_percent_max: u32,
    io_duration: u32,
    error_percent: u32,

    // Аппаратная (операндная) память.
    operand_memory: Vec<i32>,

    // Статистика.
    pub commands_executed: u64,
    pub io_initiated: u64,
    pub completed_tasks: u64,
    pub aborted_tasks: u64,
}

impl SimulationModel {
    /// Модель из файла конфигурации. Размер ОЗУ и содержимое аппаратной
    /// памяти генерируются случайно при запуске.
    pub fn from_config(cfg: &Config) -> Self {
        let mut rng = StdRng::seed_from_u64(cfg.model.random_seed);
        let total_memory = rng.gen_range(cfg.model.memory_min..=cfg.model.memory_max);
        let cpus = (1..=cfg.scheduler.cpu_count)
            .map(|id| Cpu::new(id as u32))
            .collect();

        Self {
            rng,
            process_table: vec![None; cfg.model.process_table_size],
            next_task_id: 1,
            total_memory,
            model_tick: 0,
            speed_ticks_per_second: 2.0,
            cpus,
            scheduler: Scheduler::new(&cfg.scheduler.algorithm),
            new_task: None,
            task_size_min: cfg.task.size_min,
            task_size_max: cfg.task.size_max,
            commands_min: cfg.task.commands_min,
            commands_max: cfg.task.commands_max,
            io_percent_min: cfg.task.io_percent_min,
            io_percent_max: cfg.task.io_percent_max,
            io_duration: cfg.task.io_duration,
            error_percent: cfg.task.error_percent,
            operand_memory: vec![0; cfg.memory.words],
            commands_executed: 0,
            io_initiated: 0,
            completed_tasks: 0,
            aborted_tasks: 0,
        }
    }

    /// Генерация нового задания: размер, число команд, доля ввода-вывода.
    pub fn generate_new_task(&mut self) {
        let size = self.rng.gen_range(self.task_size_min..=self.task_size_max);
        let planned_commands = self.rng.gen_range(self.commands_min..=self.commands_max);
        let io_percent = if self.io_percent_max > self.io_percent_min {
            self.rng.gen_range(self.io_percent_min..=self.io_percent_max)
        } else {
            self.io_percent_min
        };
        let id = self.next_task_id;
        self.next_task_id += 1;

        let mut task = ProcessControlBlock::new(id, size);
        task.planned_commands = planned_commands;
        task.io_percent = io_percent;
        task.io_duration = self.io_duration;
        self.new_task = Some(task);
    }

    // ---- Чтение состояния ----

    pub fn total_memory(&self) -> i32 {
        self.total_memory
    }

    pub fn used_memory(&self) -> i32 {
        self.process_table
            .iter()
            .filter_map(|p| p.as_ref())
            .map(|p| p.size)
            .sum()
    }

    pub fn free_memory(&self) -> i32 {
        self.total_memory - self.used_memory()
    }

    pub fn process_table_capacity(&self) -> usize {
        self.process_table.len()
    }

    pub fn resident_process_count(&self) -> usize {
        self.process_table.iter().filter(|p| p.is_some()).count()
    }

    pub fn model_tick(&self) -> u64 {
        self.model_tick
    }

    pub fn speed_ticks_per_second(&self) -> f64 {
        self.speed_ticks_per_second
    }

    pub fn process_table(&self) -> &[Option<ProcessControlBlock>] {
        &self.process_table
    }

    pub fn cpu_count(&self) -> usize {
        self.cpus.len()
    }

    pub fn operand_memory(&self) -> &[i32] {
        &self.operand_memory
    }

    pub fn active_process_on(&self, cpu_idx: usize) -> Option<&ProcessControlBlock> {
        self.cpus[cpu_idx]
            .active_slot
            .and_then(|slot| self.process_table[slot].as_ref())
    }

    pub fn active_process_count(&self) -> usize {
        self.cpus.iter().filter(|c| !c.is_waiting()).count()
    }

    // ---- Процедуры модели ----

    /// Инициализация модели: очистка PSW и списка готовности, генерация
    /// и загрузка заданий, инициализация памяти и первичная диспетчеризация.
    pub fn initialize(&mut self) {
        for slot in self.process_table.iter_mut() {
            *slot = None;
        }
        for cpu in self.cpus.iter_mut() {
            cpu.reset();
        }
        self.scheduler.clear();
        self.model_tick = 0;
        self.next_task_id = 1;
        self.new_task = None;
        self.commands_executed = 0;
        self.io_initiated = 0;
        self.completed_tasks = 0;
        self.aborted_tasks = 0;

        self.generate_new_task();
        while self.load_new_task() {
            self.generate_new_task();
        }
        self.fill_operand_memory();
        self.dispatch_idle_cpus();
    }

    /// Генерация содержимого аппаратной памяти.
    fn fill_operand_memory(&mut self) {
        for word in self.operand_memory.iter_mut() {
            *word = self.rng.gen_range(-50..=50);
        }
    }

    /// Проверка наличия свободной памяти.
    pub fn has_available_memory(&self) -> bool {
        match &self.new_task {
            Some(t) => t.size <= self.free_memory(),
            None => false,
        }
    }

    /// Загрузка нового задания в первую свободную запись таблицы PSW.
    /// Процесс получает состояние «Готов» и ставится в хвост очереди FIFO.
    pub fn load_new_task(&mut self) -> bool {
        if !self.has_available_memory() {
            return false;
        }
        let slot = match self.process_table.iter().position(|p| p.is_none()) {
            Some(s) => s,
            None => return false,
        };
        if let Some(mut task) = self.new_task.take() {
            let task_id = task.task_id;
            task.slot = slot as i32;
            task.state = ProcessState::Ready;
            self.process_table[slot] = Some(task);
            self.scheduler.enqueue(slot, task_id);
            true
        } else {
            false
        }
    }

    /// Загрузка заданий, ожидающих свободной памяти или слота.
    fn load_pending_tasks(&mut self) {
        while self.load_new_task() {
            self.generate_new_task();
        }
    }

    /// Сохранить слово состояния процесса: аппаратный счётчик команд
    /// ЦП переносится в PSW.
    pub fn save_process_state(&mut self, cpu_idx: usize) -> Option<usize> {
        let slot = self.cpus[cpu_idx].active_slot?;
        if let Some(process) = self.process_table[slot].as_mut() {
            process.command_counter = self.cpus[cpu_idx].hardware_command_counter;
        }
        Some(slot)
    }

    /// Восстановить слово состояния процесса: счётчик команд из PSW
    /// загружается в аппаратный счётчик ЦП, процесс становится активным.
    pub fn restore_process_state(&mut self, cpu_idx: usize, slot: usize) -> bool {
        match self.process_table[slot].as_mut() {
            Some(process) => {
                process.state = ProcessState::Active;
                self.cpus[cpu_idx].hardware_command_counter = process.command_counter;
                self.cpus[cpu_idx].active_slot = Some(slot);
                true
            }
            None => false,
        }
    }

    /// Подпрограмма планировщика GetNextProcessForCPU.
    /// Возвращает (номер слота PSW, идентификатор задания).
    pub fn get_next_process_for_cpu(&mut self) -> Option<(usize, i32)> {
        self.scheduler.get_next_process_for_cpu()
    }

    /// Проверка состояния системы: каждый простаивающий ЦП получает
    /// следующий процесс из списка готовности.
    pub fn dispatch_idle_cpus(&mut self) {
        for cpu_idx in 0..self.cpus.len() {
            if self.cpus[cpu_idx].is_waiting() {
                if let Some((slot, _task_id)) = self.get_next_process_for_cpu() {
                    self.restore_process_state(cpu_idx, slot);
                }
            }
        }
    }

    // ---- Выполнение команд процесса ----

    /// Генерация команды процесса (чтение команды из памяти).
    ///
    /// Если счётчик команд достиг числа команд задания, генерируется
    /// команда завершения. Иначе случайно выбирается вычислительная
    /// команда, команда ввода-вывода или ошибочная команда.
    pub fn generate_command(&mut self, slot: usize) -> Command {
        let (counter, planned, io_percent) = {
            let process = self.process_table[slot]
                .as_ref()
                .expect("команда генерируется только для процесса");
            (process.command_counter, process.planned_commands, process.io_percent)
        };

        if counter >= planned {
            return Command::new(CommandType::Exit, 0, 0, 0);
        }

        let roll = self.rng.gen_range(0..100u32);
        if roll < io_percent {
            let addr = self.rng.gen_range(0..self.operand_memory.len());
            Command::new(CommandType::Io, 0, addr, 0)
        } else if roll < io_percent + self.error_percent {
            Command::new(CommandType::Error, 0, 0, 0)
        } else {
            let opcodes = [b'+', b'-', b'*', b'/'];
            let opcode = opcodes[self.rng.gen_range(0..opcodes.len())];
            let addr1 = self.rng.gen_range(0..self.operand_memory.len());
            let addr2 = self.rng.gen_range(0..self.operand_memory.len());
            Command::new(CommandType::Compute, opcode, addr1, addr2)
        }
    }

    /// Чтение операндов из аппаратной памяти.
    fn read_operands(&self, command: &Command) -> (i32, i32) {
        (
            self.operand_memory[command.addr1],
            self.operand_memory[command.addr2],
        )
    }

    /// Выполнение операции (АЛУ). Возвращается None при ошибке
    /// (деление на ноль).
    fn do_operation(&self, opcode: u8, op1: i32, op2: i32) -> Option<i32> {
        match opcode {
            b'+' => Some(op1.wrapping_add(op2)),
            b'-' => Some(op1.wrapping_sub(op2)),
            b'*' => Some(op1.saturating_mul(op2)),
            b'/' => {
                if op2 == 0 {
                    None
                } else {
                    Some(op1.wrapping_div(op2))
                }
            }
            _ => None,
        }
    }

    /// Запись результата операции в аппаратную память.
    fn write_result(&mut self, address: usize, value: i32) {
        if address < self.operand_memory.len() {
            self.operand_memory[address] = value;
        }
    }

    /// Инициализация процессора ввода-вывода: слово состояния процесса
    /// сохраняется, процесс переводится в состояние «Инициализация IO»,
    /// затем — в состояние «Блокирован IO», ЦПр освобождается.
    pub fn init_io(&mut self, cpu_idx: usize, slot: usize) {
        if let Some(process) = self.process_table[slot].as_mut() {
            process.state = ProcessState::InitIo;
            process.remaining_io = process.io_duration;
            process.current_command = None;
        }
        self.save_process_state(cpu_idx);
        self.io_initiated += 1;

        // Регулировщик (лабораторная работа 5) переводит процесс в
        // состояние блокировки; в данной работе переход выполняется сразу.
        if let Some(process) = self.process_table[slot].as_mut() {
            process.state = ProcessState::BlockedIo;
        }
        self.cpus[cpu_idx].active_slot = None;
    }

    /// Завершение процесса: освобождение памяти и слота таблицы PSW,
    /// счётчик выполненных заданий, загрузка новых заданий.
    pub fn complete_process(&mut self, cpu_idx: usize, slot: usize) {
        self.save_process_state(cpu_idx);
        self.process_table[slot] = None;
        self.cpus[cpu_idx].active_slot = None;
        self.completed_tasks += 1;
        self.load_pending_tasks();
    }

    /// Удаление процесса операционной системой по ошибке.
    pub fn abort_process(&mut self, cpu_idx: usize, slot: usize, reason: &str) {
        let _ = reason;
        self.process_table[slot] = None;
        self.cpus[cpu_idx].active_slot = None;
        self.aborted_tasks += 1;
        self.load_pending_tasks();
    }

    /// Выполнение одной команды активного процесса на заданном ЦП.
    pub fn execute_command_on_cpu(&mut self, cpu_idx: usize) {
        let slot = match self.cpus[cpu_idx].active_slot {
            Some(slot) => slot,
            None => return,
        };

        // 1. Чтение команды из памяти по адресу на счётчике команд
        //    (в модели — генерация команды).
        let need_command = self.process_table[slot]
            .as_ref()
            .map(|p| p.current_command.is_none())
            .unwrap_or(false);
        if need_command {
            let command = self.generate_command(slot);
            if let Some(process) = self.process_table[slot].as_mut() {
                process.current_command = Some(command);
            }
        }

        // 2. Разборка команды; увеличение счётчика команд на длину
        //    команды (для модели длина команды равна 1).
        let command = match self.process_table[slot]
            .as_ref()
            .and_then(|p| p.current_command)
        {
            Some(command) => command,
            None => return,
        };
        if let Some(process) = self.process_table[slot].as_mut() {
            process.command_counter += 1;
            process.last_command = Some(command);
        }
        self.cpus[cpu_idx].hardware_command_counter += 1;
        self.save_process_state(cpu_idx);

        // 3. Чтение операндов из аппаратной памяти.
        let (op1, op2) = self.read_operands(&command);

        // 4-6. Выполнение операции и запись результата.
        match command.kind {
            CommandType::Compute => match self.do_operation(command.opcode, op1, op2) {
                Some(result) => {
                    self.write_result(command.addr2, result);
                    if let Some(process) = self.process_table[slot].as_mut() {
                        process.op1 = op1;
                        process.op2 = op2;
                        process.result = result;
                        process.current_command = None;
                    }
                    self.commands_executed += 1;
                }
                None => {
                    self.commands_executed += 1;
                    self.abort_process(cpu_idx, slot, "деление на ноль");
                }
            },
            CommandType::Io => {
                self.commands_executed += 1;
                self.init_io(cpu_idx, slot);
            }
            CommandType::Exit => {
                self.commands_executed += 1;
                self.complete_process(cpu_idx, slot);
            }
            CommandType::Error => {
                self.commands_executed += 1;
                self.abort_process(cpu_idx, slot, "ошибочная команда");
            }
        }
    }

    /// Выполнение одного такта моделирования.
    pub fn execute_one_tick(&mut self) {
        // 1. Моделирование ЦПр: каждый активный процесс выполняет
        //    очередную команду.
        for cpu_idx in 0..self.cpus.len() {
            if self.cpus[cpu_idx].active_slot.is_some() {
                self.execute_command_on_cpu(cpu_idx);
            }
        }
        // 2. Проверка состояния системы и диспетчеризация.
        self.dispatch_idle_cpus();
        // 3. Служебный счётчик тактов модели.
        self.model_tick += 1;
    }

    /// Изменение скорости с ограничением [0.1; 1000.0]
    pub fn change_speed(&mut self, multiplier: f64) {
        self.speed_ticks_per_second = (self.speed_ticks_per_second * multiplier).clamp(0.1, 1000.0);
    }
}
