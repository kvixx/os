use crate::domain::{Command, CommandType, Cpu, IoProcessor, ProcessControlBlock, ProcessState};
use crate::services::{Params, Scheduler};
use rand::rngs::StdRng;
use rand::{Rng, SeedableRng};
use std::collections::VecDeque;

/// Сколько последних событий регулировщика хранить для индикации.
const EVENT_LOG_LIMIT: usize = 6;

/// Ядро учебной модели ОС.
///
/// Лабораторная работа 8: входные параметры читаются из текстового файла
/// параметров (диапазон, дискретность, комментарий) один раз при запуске.
pub struct SimulationModel {
    rng: StdRng,
    process_table: Vec<Option<ProcessControlBlock>>,
    next_task_id: i32,
    total_memory: i32,
    model_tick: u64,
    speed_ticks_per_second: f64,
    pub cpus: Vec<Cpu>,
    pub io_processors: Vec<IoProcessor>,
    pub scheduler: Scheduler,
    pub new_task: Option<ProcessControlBlock>,

    /// Параметры модели, прочитанные из файла и сгенерированные.
    pub params: Params,

    // Аппаратная (операндная) память.
    operand_memory: Vec<i32>,

    // Журналы и статистика.
    event_log: VecDeque<String>,
    pub commands_executed: u64,
    pub io_initiated: u64,
    pub io_completed: u64,
    pub completed_tasks: u64,
    pub aborted_tasks: u64,

    // Накопленные параметры для расчёта средних значений (ЛР6).
    pub sum_turnaround: u64,
    pub sum_ready: u64,
    pub sum_active: u64,
    pub sum_io: u64,
    pub sum_commands_done: u64,
    pub busy_cpu_ticks: u64,
    pub queue_len_sum: u64,

    /// Сумма расчётных времён T_mono завершённых заданий.
    pub sum_mono: u64,

    // Состояние директив оператора (лабораторная работа 9).
    /// Автоматическая загрузка новых заданий.
    autoload: bool,
    /// Режим «выполнить загруженные задания и выключить питание».
    finish_requested: bool,
    /// Выбранный оператором процесс (номер слота таблицы PSW).
    selected_slot: usize,
    /// Заданий уничтожено оператором.
    pub destroyed_tasks: u64,
}

impl SimulationModel {
    /// Модель по параметрам из файла; размер ОЗУ и содержимое
    /// аппаратной памяти генерируются случайно при запуске.
    pub fn from_params(params: &Params) -> Self {
        let rng = StdRng::seed_from_u64(params.random_seed);
        let cpus = (1..=params.cpu_count)
            .map(|id| Cpu::new(id as u32))
            .collect();
        let io_processors = (1..=params.io_processors)
            .map(|id| IoProcessor::new(id as u32))
            .collect();

        Self {
            rng,
            process_table: vec![None; params.process_table_size],
            next_task_id: 1,
            total_memory: params.total_memory,
            model_tick: 0,
            speed_ticks_per_second: 2.0,
            cpus,
            io_processors,
            scheduler: Scheduler::new("FIFO"),
            new_task: None,
            params: params.clone(),
            operand_memory: vec![0; params.memory_words],
            event_log: VecDeque::new(),
            commands_executed: 0,
            io_initiated: 0,
            io_completed: 0,
            completed_tasks: 0,
            aborted_tasks: 0,
            sum_turnaround: 0,
            sum_ready: 0,
            sum_active: 0,
            sum_io: 0,
            sum_commands_done: 0,
            busy_cpu_ticks: 0,
            queue_len_sum: 0,
            sum_mono: 0,
            autoload: true,
            finish_requested: false,
            selected_slot: 0,
            destroyed_tasks: 0,
        }
    }

    /// Генерация нового задания по параметрам файла: размер, число
    /// команд, доля команд ввода-вывода (с дискретностью).
    pub fn generate_new_task(&mut self) {
        let size = self.params.task_size.generate(&mut self.rng) as i32;
        let planned_commands = self.params.commands.generate(&mut self.rng) as u64;
        let io_percent = self.params.io_percent.generate(&mut self.rng) as u32;
        let id = self.next_task_id;
        self.next_task_id += 1;

        let mut task = ProcessControlBlock::new(id, size);
        task.planned_commands = planned_commands;
        task.io_percent = io_percent;
        task.io_duration = self.params.io_duration.min.max(1) as u32;
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

    pub fn event_log(&self) -> &VecDeque<String> {
        &self.event_log
    }

    pub fn active_process_on(&self, cpu_idx: usize) -> Option<&ProcessControlBlock> {
        self.cpus[cpu_idx]
            .active_slot
            .and_then(|slot| self.process_table[slot].as_ref())
    }

    pub fn active_process_count(&self) -> usize {
        self.cpus.iter().filter(|c| !c.is_waiting()).count()
    }

    // ---- Журналы ----

    fn log_event(&mut self, message: String) {
        self.event_log.push_back(message);
        if self.event_log.len() > EVENT_LOG_LIMIT {
            self.event_log.pop_front();
        }
    }

    // ---- Регулировщик ----

    /// Регулировщик — единственная подпрограмма, изменяющая состояния
    /// процессов. Переводит процесс в новое состояние и выполняет
    /// побочные действия (постановка в очередь готовности и т.п.).
    pub fn regulator(&mut self, slot: usize, new_state: ProcessState) {
        let (task_id, old_state) = match self.process_table[slot].as_ref() {
            Some(process) => (process.task_id, process.state),
            None => return,
        };
        if old_state == new_state {
            return;
        }
        if let Some(process) = self.process_table[slot].as_mut() {
            process.state = new_state;
        }
        if new_state == ProcessState::Ready {
            self.scheduler.enqueue(slot, task_id);
        }
        self.log_event(format!(
            "ID {}: {} -> {}",
            task_id,
            old_state.name(),
            new_state.name()
        ));
    }

    // ---- Процедуры модели ----

    /// Инициализация модели.
    pub fn initialize(&mut self) {
        for slot in self.process_table.iter_mut() {
            *slot = None;
        }
        for cpu in self.cpus.iter_mut() {
            cpu.reset();
        }
        for io in self.io_processors.iter_mut() {
            io.reset();
        }
        self.scheduler.clear();
        self.model_tick = 0;
        self.next_task_id = 1;
        self.new_task = None;
        self.event_log.clear();
        self.commands_executed = 0;
        self.io_initiated = 0;
        self.io_completed = 0;
        self.completed_tasks = 0;
        self.aborted_tasks = 0;
        self.sum_turnaround = 0;
        self.sum_ready = 0;
        self.sum_active = 0;
        self.sum_io = 0;
        self.sum_commands_done = 0;
        self.busy_cpu_ticks = 0;
        self.queue_len_sum = 0;
        self.sum_mono = 0;
        self.autoload = true;
        self.finish_requested = false;
        self.selected_slot = 0;
        self.destroyed_tasks = 0;

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
            task.arrival_tick = self.model_tick;
            self.process_table[slot] = Some(task);
            self.scheduler.enqueue(slot, task_id);
            true
        } else {
            false
        }
    }

    /// Загрузка заданий, ожидающих свободной памяти или слота.
    /// При выключенной автозагрузке не выполняется.
    fn load_pending_tasks(&mut self) {
        if !self.autoload {
            return;
        }
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
    /// загружается в аппаратный счётчик ЦП; процесс становится активным
    /// (перевод состояния выполняет регулировщик).
    pub fn restore_process_state(&mut self, cpu_idx: usize, slot: usize) -> bool {
        if self.process_table[slot].is_none() {
            return false;
        }
        self.regulator(slot, ProcessState::Active);
        if let Some(process) = self.process_table[slot].as_ref() {
            self.cpus[cpu_idx].hardware_command_counter = process.command_counter;
            self.cpus[cpu_idx].active_slot = Some(slot);
            true
        } else {
            false
        }
    }

    /// Подпрограмма планировщика GetNextProcessForCPU.
    /// Возвращает (номер слота PSW, идентификатор задания).
    pub fn get_next_process_for_cpu(&mut self) -> Option<(usize, i32)> {
        self.scheduler.get_next_process_for_cpu()
    }

    /// Проверка состояния системы: диспетчеризация процессов на
    /// простаивающие ЦПр и загрузка новых заданий, если есть ресурсы.
    pub fn check_system_state(&mut self) {
        self.load_pending_tasks();
        self.dispatch_idle_cpus();
    }

    /// Раздача процессов простаивающим ЦПр.
    pub fn dispatch_idle_cpus(&mut self) {
        for cpu_idx in 0..self.cpus.len() {
            if self.cpus[cpu_idx].is_waiting() {
                if let Some((slot, _task_id)) = self.get_next_process_for_cpu() {
                    self.restore_process_state(cpu_idx, slot);
                    // Системные затраты планировщика и переключения
                    // контекста увеличивают время готовых процессов.
                    self.add_ready_time(self.params.switch_ticks);
                }
            }
        }
    }

    /// Подпрограмма «Увеличить время готовых процессов» на заданное
    /// число тактов (системные затраты ОС).
    pub fn add_ready_time(&mut self, ticks: u64) {
        if ticks == 0 {
            return;
        }
        for slot in 0..self.process_table.len() {
            if let Some(process) = self.process_table[slot].as_mut() {
                if process.state == ProcessState::Ready {
                    process.ready_ticks += ticks;
                }
            }
        }
    }

    // ---- Выполнение команд процесса ----

    /// Генерация команды процесса (чтение команды из памяти).
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
        } else if roll < io_percent + self.params.error_percent.min as u32 {
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

    /// Выполнение операции (АЛУ).
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

    /// Выполнение одной команды активного процесса на заданном ЦП.
    pub fn execute_command_on_cpu(&mut self, cpu_idx: usize) {
        let slot = match self.cpus[cpu_idx].active_slot {
            Some(slot) => slot,
            None => return,
        };

        // 1. Чтение команды из памяти (генерация).
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

        // 2. Разборка команды; счётчик команд увеличивается на длину
        //    команды (длина команды = 1).
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

        // 3. Чтение операндов.
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

    // ---- Ввод-вывод и прерывания ----

    /// Инициализация ввода-вывода: слово состояния процесса сохраняется,
    /// процесс переводится в состояние «Инициализация IO», получает
    /// процессор ввода-вывода и переводится регулировщиком в состояние
    /// «Блокирован IO»; ЦПр освобождается.
    pub fn init_io(&mut self, cpu_idx: usize, slot: usize) {
        let (task_id, io_duration) = {
            let process = self.process_table[slot]
                .as_mut()
                .expect("процесс существует");
            process.remaining_io = process.io_duration;
            process.current_command = None;
            (process.task_id, process.io_duration)
        };
        self.regulator(slot, ProcessState::InitIo);
        self.save_process_state(cpu_idx);
        self.io_initiated += 1;
        self.add_ready_time(self.params.init_io_ticks);
        if let Some(process) = self.process_table[slot].as_mut() {
            process.io_commands += 1;
        }
        self.cpus[cpu_idx].active_slot = None;

        // Системный процесс инициализации ищет свободный процессор IO.
        if let Some(io_idx) = self.io_processors.iter().position(|p| p.is_free()) {
            self.io_processors[io_idx].start(slot, task_id, io_duration);
            self.regulator(slot, ProcessState::BlockedIo);
            self.log_event(format!(
                "ID {}: IO-{} начал ввод-вывод ({} такт.)",
                task_id,
                self.io_processors[io_idx].id,
                io_duration
            ));
        } else {
            self.log_event(format!(
                "ID {}: ожидание свободного процессора IO",
                task_id
            ));
        }
    }

    /// Назначение процессоров ввода-вывода процессам, ожидающим
    /// в состоянии «Инициализация IO».
    fn assign_pending_io(&mut self) {
        for slot in 0..self.process_table.len() {
            let pending = self.process_table[slot]
                .as_ref()
                .map(|p| p.state == ProcessState::InitIo)
                .unwrap_or(false);
            if !pending {
                continue;
            }
            let (task_id, duration) = {
                let process = self.process_table[slot].as_ref().unwrap();
                (process.task_id, process.io_duration)
            };
            if let Some(io_idx) = self.io_processors.iter().position(|p| p.is_free()) {
                self.io_processors[io_idx].start(slot, task_id, duration);
                self.regulator(slot, ProcessState::BlockedIo);
                self.log_event(format!(
                    "ID {}: IO-{} начал ввод-вывод ({} такт.)",
                    task_id,
                    self.io_processors[io_idx].id,
                    duration
                ));
            }
        }
    }

    /// Моделирование процессоров ввода-вывода: один такт обслуживания и
    /// обработка прерываний по окончании операций.
    fn tick_io_processors(&mut self) {
        for idx in 0..self.io_processors.len() {
            if self.io_processors[idx].is_free() {
                continue;
            }
            self.io_processors[idx].tick();
            if self.io_processors[idx].is_finished() {
                let slot = self.io_processors[idx].slot.unwrap_or(0);
                let task_id = self.io_processors[idx].task_id;
                let io_id = self.io_processors[idx].id;
                self.io_processors[idx].release();
                self.io_completed += 1;
                self.log_event(format!(
                    "IO-{}: прерывание по окончании ввода-вывода ID {}",
                    io_id, task_id
                ));
                // Регулировщик возвращает процесс в состояние «Готов».
                self.regulator(slot, ProcessState::Ready);
                self.add_ready_time(self.params.intr_io_ticks);
            }
        }
    }

    /// Завершение процесса: освобождение памяти и слота, загрузка новых.
    pub fn complete_process(&mut self, cpu_idx: usize, slot: usize) {
        let (task_id, turnaround, ready, active, io, commands, io_commands, io_duration) = {
            let process = self.process_table[slot]
                .as_ref()
                .expect("процесс существует");
            (
                process.task_id,
                self.model_tick.saturating_sub(process.arrival_tick),
                process.ready_ticks,
                process.active_ticks,
                process.io_ticks,
                process.command_counter,
                process.io_commands,
                process.io_duration as u64,
            )
        };
        // Накопление параметров завершённого задания для средних значений.
        self.sum_turnaround += turnaround;
        self.sum_ready += ready;
        self.sum_active += active;
        self.sum_io += io;
        self.sum_commands_done += commands;

        // Расчётное время выполнения задания в однопрограммной ОС:
        // загрузка + вычислительные команды + команды ввода-вывода.
        let mono_ticks = self.params.load_ticks
            + commands.saturating_sub(io_commands)
            + io_commands * io_duration;
        self.sum_mono += mono_ticks;
        if let Some(process) = self.process_table[slot].as_mut() {
            process.mono_ticks = mono_ticks;
        }

        self.save_process_state(cpu_idx);
        self.regulator(slot, ProcessState::Completed);
        self.process_table[slot] = None;
        self.cpus[cpu_idx].active_slot = None;
        self.completed_tasks += 1;
        self.log_event(format!(
            "ID {}: задание завершено, память освобождена",
            task_id
        ));
        self.load_pending_tasks();
        self.add_ready_time(self.params.load_ticks);
    }

    /// Удаление процесса операционной системой по ошибке.
    pub fn abort_process(&mut self, cpu_idx: usize, slot: usize, reason: &str) {
        let task_id = self.process_table[slot]
            .as_ref()
            .map(|p| p.task_id)
            .unwrap_or(0);
        self.process_table[slot] = None;
        self.cpus[cpu_idx].active_slot = None;
        self.aborted_tasks += 1;
        self.log_event(format!("ID {}: удалён по ошибке ({})", task_id, reason));
        self.load_pending_tasks();
    }

    // ---- Учёт временных параметров и средние значения (ЛР6) ----

    /// Учёт времени нахождения процессов в состояниях (выполняется
    /// каждый такт моделирования).
    fn account_state_times(&mut self) {
        for slot in 0..self.process_table.len() {
            if let Some(process) = self.process_table[slot].as_mut() {
                match process.state {
                    ProcessState::Ready => process.ready_ticks += 1,
                    ProcessState::Active => process.active_ticks += 1,
                    ProcessState::InitIo | ProcessState::BlockedIo => process.io_ticks += 1,
                    _ => {}
                }
            }
        }
        let busy = self.cpus.iter().filter(|c| !c.is_waiting()).count() as u64;
        self.busy_cpu_ticks += busy;
        self.queue_len_sum += self.scheduler.len() as u64;
    }

    fn average(sum: u64, count: u64) -> f64 {
        if count == 0 {
            0.0
        } else {
            sum as f64 / count as f64
        }
    }

    /// Среднее время нахождения задания в системе (оборотное время).
    pub fn average_turnaround(&self) -> f64 {
        Self::average(self.sum_turnaround, self.completed_tasks)
    }

    /// Среднее время ожидания в состоянии «Готов».
    pub fn average_ready(&self) -> f64 {
        Self::average(self.sum_ready, self.completed_tasks)
    }

    /// Среднее время работы на ЦПр.
    pub fn average_active(&self) -> f64 {
        Self::average(self.sum_active, self.completed_tasks)
    }

    /// Среднее время блокировки по вводу-выводу.
    pub fn average_io(&self) -> f64 {
        Self::average(self.sum_io, self.completed_tasks)
    }

    /// Среднее число выполненных команд задания.
    pub fn average_commands(&self) -> f64 {
        Self::average(self.sum_commands_done, self.completed_tasks)
    }

    /// Среднее расчётное время выполнения задания в однопрограммной ОС.
    pub fn average_mono(&self) -> f64 {
        Self::average(self.sum_mono, self.completed_tasks)
    }

    /// Производительность относительно однопрограммного режима, %:
    /// отношение числа выполненных заданий к числу заданий, которые
    /// могли бы быть выполнены за это же время в однопрограммной ОС.
    pub fn productivity_relative(&self) -> f64 {
        if self.model_tick == 0 || self.completed_tasks == 0 {
            return 0.0;
        }
        let mono_average = self.average_mono();
        if mono_average <= 0.0 {
            return 0.0;
        }
        let mono_possible = self.model_tick as f64 / mono_average;
        self.completed_tasks as f64 / mono_possible * 100.0
    }

    /// Системные затраты модели: (T_Load, T_InitIO, T_IntrIO, T_next).
    pub fn system_costs(&self) -> (u64, u64, u64, u64) {
        (
            self.params.load_ticks,
            self.params.init_io_ticks,
            self.params.intr_io_ticks,
            self.params.switch_ticks,
        )
    }

    /// Полезная загрузка центральных процессоров, %.
    pub fn cpu_utilization(&self) -> f64 {
        if self.model_tick == 0 || self.cpus.is_empty() {
            0.0
        } else {
            self.busy_cpu_ticks as f64 * 100.0 / (self.cpus.len() as f64 * self.model_tick as f64)
        }
    }

    /// Средняя длина очереди готовности, процессов.
    pub fn average_queue_len(&self) -> f64 {
        if self.model_tick == 0 {
            0.0
        } else {
            self.queue_len_sum as f64 / self.model_tick as f64
        }
    }

    /// Производительность: завершённых заданий за такт.
    pub fn productivity(&self) -> f64 {
        if self.model_tick == 0 {
            0.0
        } else {
            self.completed_tasks as f64 / self.model_tick as f64
        }
    }

    // ---- Такт моделирования ----

    /// Выполнение одного такта моделирования по алгоритму лабораторной
    /// работы 5.
    pub fn execute_one_tick(&mut self) {
        // Б.1. Моделирование ЦПр: каждый активный процесс выполняет
        //      очередную команду.
        for cpu_idx in 0..self.cpus.len() {
            if self.cpus[cpu_idx].active_slot.is_some() {
                self.execute_command_on_cpu(cpu_idx);
            }
        }
        // Б.2. Моделирование процессоров ввода-вывода и обработка
        //      прерываний по окончании операций.
        self.assign_pending_io();
        self.tick_io_processors();
        // Б.3. Прерывания по времени для варианта 1 (FIFO) не используются.
        // Б.4. Проверка достигнутого состояния системы.
        self.check_system_state();
        // Б.5. Обновление баз данных модели (временные параметры).
        self.account_state_times();
        // Б.6. Индикация выполняется извне.
        self.model_tick += 1;
    }

    // ---- Директивы оператора (ЛР9) ----

    pub fn is_autoload(&self) -> bool {
        self.autoload
    }

    /// Директива «приостановить/продолжить автоматическую загрузку».
    pub fn set_autoload(&mut self, value: bool) {
        if self.autoload != value {
            self.autoload = value;
            self.log_event(format!(
                "Оператор: автозагрузка {}",
                if value { "включена" } else { "приостановлена" }
            ));
        }
    }

    /// Директива «загрузить новое задание».
    pub fn load_one_task(&mut self) -> bool {
        if self.load_new_task() {
            self.generate_new_task();
            self.add_ready_time(self.params.load_ticks);
            self.log_event(String::from("Оператор: загружено новое задание"));
            true
        } else {
            self.log_event(String::from("Оператор: нет ресурсов для загрузки"));
            false
        }
    }

    pub fn selected_slot(&self) -> usize {
        self.selected_slot
    }

    /// Перемещение выбора по занятым слотам таблицы PSW.
    pub fn select_next(&mut self) {
        for step in 1..=self.process_table.len() {
            let slot = (self.selected_slot + step) % self.process_table.len();
            if self.process_table[slot].is_some() {
                self.selected_slot = slot;
                return;
            }
        }
    }

    pub fn select_prev(&mut self) {
        for step in 1..=self.process_table.len() {
            let slot = (self.selected_slot + self.process_table.len() - step)
                % self.process_table.len();
            if self.process_table[slot].is_some() {
                self.selected_slot = slot;
                return;
            }
        }
    }

    /// Директива «приостановить/активизировать выбранный процесс».
    pub fn suspend_or_resume_selected(&mut self) -> bool {
        let slot = self.selected_slot;
        let (task_id, state) = match self.process_table[slot].as_ref() {
            Some(process) => (process.task_id, process.state),
            None => return false,
        };
        match state {
            ProcessState::Suspended => {
                self.regulator(slot, ProcessState::Ready);
                self.log_event(format!("Оператор: ID {} активизирован", task_id));
                true
            }
            ProcessState::Ready => {
                self.scheduler.remove(slot);
                self.regulator(slot, ProcessState::Suspended);
                self.log_event(format!("Оператор: ID {} приостановлен", task_id));
                true
            }
            ProcessState::Active => {
                if let Some(cpu_idx) = self.cpus.iter().position(|c| c.active_slot == Some(slot)) {
                    self.save_process_state(cpu_idx);
                    self.cpus[cpu_idx].active_slot = None;
                }
                self.regulator(slot, ProcessState::Suspended);
                self.log_event(format!("Оператор: ID {} приостановлен (снят с ЦП)", task_id));
                true
            }
            ProcessState::InitIo | ProcessState::BlockedIo => {
                for io in self.io_processors.iter_mut() {
                    if io.slot == Some(slot) {
                        io.release();
                    }
                }
                self.regulator(slot, ProcessState::Suspended);
                self.log_event(format!("Оператор: ID {} приостановлен (IO отменён)", task_id));
                true
            }
            _ => false,
        }
    }

    /// Директива «уничтожить выбранный процесс».
    pub fn destroy_selected(&mut self) -> bool {
        let slot = self.selected_slot;
        let task_id = match self.process_table[slot].as_ref() {
            Some(process) => process.task_id,
            None => return false,
        };
        if let Some(cpu_idx) = self.cpus.iter().position(|c| c.active_slot == Some(slot)) {
            self.save_process_state(cpu_idx);
            self.cpus[cpu_idx].active_slot = None;
        }
        for io in self.io_processors.iter_mut() {
            if io.slot == Some(slot) {
                io.release();
            }
        }
        self.scheduler.remove(slot);
        self.process_table[slot] = None;
        self.destroyed_tasks += 1;
        self.log_event(format!("Оператор: ID {} уничтожен", task_id));
        true
    }

    /// Директива «изменить параметры нового задания»: изменение верхней
    /// границы числа команд для новых заданий.
    pub fn change_task_commands(&mut self, delta: i64) -> bool {
        let min = self.params.commands.min.max(1);
        let new_max = (self.params.commands.max + delta).clamp(min, 64);
        if new_max == self.params.commands.max {
            return false;
        }
        self.params.commands.max = new_max;
        self.generate_new_task();
        self.log_event(format!(
            "Оператор: число команд новых заданий до {}",
            new_max
        ));
        true
    }

    /// Директива «изменить размер памяти модели».
    pub fn change_memory(&mut self, delta: i32) -> bool {
        let new_total = (self.total_memory + delta).max(256);
        if (new_total as i64) < self.used_memory() as i64 {
            self.log_event(String::from(
                "Оператор: недостаточно свободной памяти для уменьшения",
            ));
            return false;
        }
        self.total_memory = new_total;
        self.log_event(format!("Оператор: размер ОЗУ = {}", new_total));
        true
    }

    /// Директива «выполнить загруженные задания и выключить питание».
    pub fn request_finish(&mut self) {
        self.autoload = false;
        self.finish_requested = true;
        self.log_event(String::from(
            "Оператор: догрузка остановлена, выполнение до конца",
        ));
    }

    pub fn finish_requested(&self) -> bool {
        self.finish_requested
    }

    /// Все загруженные задания выполнены (для директивы F).
    pub fn is_finish_complete(&self) -> bool {
        self.finish_requested && self.resident_process_count() == 0
    }

    /// Текущая верхняя граница числа команд новых заданий.
    pub fn task_commands_max(&self) -> i64 {
        self.params.commands.max
    }

    /// Изменение скорости с ограничением [0.1; 1000.0]
    pub fn change_speed(&mut self, multiplier: f64) {
        self.speed_ticks_per_second = (self.speed_ticks_per_second * multiplier).clamp(0.1, 1000.0);
    }
}
