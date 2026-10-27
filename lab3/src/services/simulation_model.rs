use crate::domain::{Cpu, ProcessControlBlock, ProcessState};
use crate::services::{Config, Scheduler};
use rand::rngs::StdRng;
use rand::{Rng, SeedableRng};

/// Ядро учебной модели ОС.
///
/// В третьей лабораторной работе модель расширена супервизором времени:
/// добавлены три центральных процессора, список готовности FIFO,
/// процедуры сохранения/восстановления слова состояния процесса и
/// подпрограмма планировщика GetNextProcessForCPU.
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
    task_size_min: i32,
    task_size_max: i32,
}

impl SimulationModel {
    /// Размер ОЗУ генерируется случайно в диапазоне [memory_min; memory_max],
    /// число ЦПр и дисциплина диспетчеризации берутся из файла конфигурации.
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
        }
    }

    /// Генерация нового задания со случайным размером.
    pub fn generate_new_task(&mut self) {
        let size = self.rng.gen_range(self.task_size_min..=self.task_size_max);
        let id = self.next_task_id;
        self.next_task_id += 1;
        self.new_task = Some(ProcessControlBlock::new(id, size));
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

    /// Процесс по номеру слота таблицы PSW.
    pub fn process_by_slot(&self, slot: usize) -> Option<&ProcessControlBlock> {
        self.process_table[slot].as_ref()
    }

    /// Активный процесс на заданном ЦП.
    pub fn active_process_on(&self, cpu_idx: usize) -> Option<&ProcessControlBlock> {
        self.cpus[cpu_idx]
            .active_slot
            .and_then(|slot| self.process_table[slot].as_ref())
    }

    /// Число процессов, находящихся в состоянии «Активен».
    pub fn active_process_count(&self) -> usize {
        self.cpus.iter().filter(|c| !c.is_waiting()).count()
    }

    // ---- Процедуры модели ----

    /// Инициализация модели: очистка PSW и списка готовности, генерация
    /// и загрузка заданий, первичная диспетчеризация на три ЦПр.
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

        self.generate_new_task();
        while self.load_new_task() {
            self.generate_new_task();
        }
        self.dispatch_idle_cpus();
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
            task.slot = slot as i32;
            task.state = ProcessState::Ready;
            self.process_table[slot] = Some(task);
            self.scheduler.enqueue(slot);
            true
        } else {
            false
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
    pub fn get_next_process_for_cpu(&mut self) -> Option<usize> {
        self.scheduler.get_next_process_for_cpu()
    }

    /// Проверка состояния системы: каждый простаивающий ЦП получает
    /// следующий процесс из списка готовности.
    pub fn dispatch_idle_cpus(&mut self) {
        for cpu_idx in 0..self.cpus.len() {
            if self.cpus[cpu_idx].is_waiting() {
                if let Some(slot) = self.get_next_process_for_cpu() {
                    self.restore_process_state(cpu_idx, slot);
                }
            }
        }
    }

    /// Выполнение одного такта моделирования.
    pub fn execute_one_tick(&mut self) {
        // 1. Моделирование ЦПр: все активные процессы выполняют
        //    очередную команду (одна команда — один такт).
        for cpu_idx in 0..self.cpus.len() {
            if self.cpus[cpu_idx].active_slot.is_some() {
                self.cpus[cpu_idx].hardware_command_counter += 1;
                self.save_process_state(cpu_idx);
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
