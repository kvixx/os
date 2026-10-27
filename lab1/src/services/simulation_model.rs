use crate::domain::{Cpu, ProcessControlBlock, ProcessState};
use rand::rngs::StdRng;
use rand::{Rng, SeedableRng};

/// Ядро учебной модели ОС.

pub struct SimulationModel {
    rng: StdRng,
    process_table: Vec<Option<ProcessControlBlock>>,
    next_task_id: i32,
    total_memory: i32,
    model_tick: u64,
    speed_ticks_per_second: f64,
    pub cpu: Cpu,
    pub new_task: Option<ProcessControlBlock>,
}

impl SimulationModel {
    pub fn new(memory_size: i32, process_table_size: usize, random_seed: u64) -> Self {
        Self {
            rng: StdRng::seed_from_u64(random_seed),
            process_table: vec![None; process_table_size],
            next_task_id: 1,
            total_memory: memory_size,
            model_tick: 0,
            speed_ticks_per_second: 2.0,
            cpu: Cpu::new(),
            new_task: None,
        }
    }

    /// Значения по умолчанию: 2048 ед. памяти, 8 слотов PSW, seed = 14.
    pub fn default_new() -> Self {
        Self::new(2048, 8, 14)
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

    pub fn active_process(&self) -> Option<&ProcessControlBlock> {
        self.cpu
            .active_slot
            .and_then(|s| self.process_table[s].as_ref())
    }

    // ---- Процедуры модели ----

    /// Инициализация модели
    pub fn initialize(&mut self) {
        for slot in self.process_table.iter_mut() {
            *slot = None;
        }
        self.cpu.active_slot = None;
        self.cpu.hardware_command_counter = 0;
        self.model_tick = 0;
        self.next_task_id = 1;
        self.new_task = None;

        self.generate_new_task();
        while self.load_new_task() {
            self.generate_new_task();
        }
        self.dispatch_first_ready_process();
    }

    /// Генерация нового задания со случайным размером 96..384.
    pub fn generate_new_task(&mut self) {
        let size = self.rng.gen_range(96..385);
        let id = self.next_task_id;
        self.next_task_id += 1;
        self.new_task = Some(ProcessControlBlock::new(id, size));
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
            task.slot = slot as i32;
            task.state = ProcessState::Ready;
            self.process_table[slot] = Some(task);
            true
        } else {
            false
        }
    }

    /// Выполнение одного такта активного процесса.
    pub fn execute_one_tick(&mut self) {
        if self.cpu.active_slot.is_none() {
            self.dispatch_first_ready_process();
        }
        if let Some(slot) = self.cpu.active_slot {
            if let Some(process) = self.process_table[slot].as_mut() {
                process.command_counter += 1;
                self.cpu.hardware_command_counter += 1;
            }
        }
        self.model_tick += 1;
    }

    /// Изменение скорости с ограничением [0.1; 1000.0]
    pub fn change_speed(&mut self, multiplier: f64) {
        self.speed_ticks_per_second = (self.speed_ticks_per_second * multiplier).clamp(0.1, 1000.0);
    }

    /// Выбор первого готового процесса и передача его на ЦП.
    fn dispatch_first_ready_process(&mut self) {
        let slot = self
            .process_table
            .iter()
            .position(|p| matches!(p.as_ref().map(|x| x.state), Some(ProcessState::Ready)));
        if let Some(slot) = slot {
            if let Some(process) = self.process_table[slot].as_mut() {
                process.state = ProcessState::Active;
                self.cpu.active_slot = Some(slot);
                self.cpu.hardware_command_counter = process.command_counter;
            }
        }
    }
}
