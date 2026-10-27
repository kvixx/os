use rand::rngs::StdRng;
use rand::{Rng, SeedableRng};
use std::fs;

/// Описание одного параметра: диапазон, дискретность, комментарий.
#[derive(Debug, Clone)]
pub struct ParamSpec {
    pub min: i64,
    pub max: i64,
    pub step: i64,
    pub comment: String,
}

impl ParamSpec {
    pub fn new(min: i64, max: i64, step: i64, comment: &str) -> Self {
        Self {
            min,
            max,
            step,
            comment: comment.to_string(),
        }
    }

    /// Генерация значения параметра в диапазоне [min; max] с заданной
    /// дискретностью step. Последовательность значений определяется
    /// состоянием генератора (зерном), что обеспечивает повторяемость.
    pub fn generate(&self, rng: &mut StdRng) -> i64 {
        if self.max <= self.min || self.step <= 1 {
            self.min
        } else {
            let steps = (self.max - self.min) / self.step;
            self.min + rng.gen_range(0..=steps) * self.step
        }
    }
}

/// Прочитанные из файла и сгенерированные параметры модели.
#[derive(Debug, Clone)]
pub struct Params {
    /// Откуда прочитаны параметры (для индикации).
    pub source: String,
    /// Все описания параметров (в порядке следования в файле).
    pub specs: Vec<ParamSpec>,

    // Спецификации параметров заданий (генерируются для каждого задания).
    pub task_size: ParamSpec,
    pub commands: ParamSpec,
    pub io_percent: ParamSpec,
    pub io_duration: ParamSpec,
    pub error_percent: ParamSpec,

    // Параметры системы (сгенерированы один раз при запуске).
    pub random_seed: u64,
    pub total_memory: i32,
    pub process_table_size: usize,
    pub cpu_count: usize,
    pub io_processors: usize,
    pub memory_words: usize,
    pub load_ticks: u64,
    pub init_io_ticks: u64,
    pub intr_io_ticks: u64,
    pub switch_ticks: u64,
}

/// Разбор содержимого файла параметров.
fn parse_params(text: &str) -> Result<Vec<ParamSpec>, String> {
    let mut specs = Vec::new();
    for (number, line) in text.lines().enumerate() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let mut parts = line.split_whitespace();
        let min = parts.next().and_then(|s| s.parse::<i64>().ok());
        let max = parts.next().and_then(|s| s.parse::<i64>().ok());
        let step = parts.next().and_then(|s| s.parse::<i64>().ok());
        let comment = parts.collect::<Vec<_>>().join(" ");
        match (min, max, step) {
            (Some(min), Some(max), Some(step)) => {
                specs.push(ParamSpec::new(min, max, step, &comment));
            }
            _ => {
                return Err(format!(
                    "строка {number}: неверный формат (нужны мин, макс, дискретность и комментарий): {line}"
                ));
            }
        }
    }
    Ok(specs)
}

impl Params {
    /// Чтение файла параметров (один раз при запуске модели) и
    /// генерация параметров системы.
    pub fn load(path: &str) -> Result<Self, String> {
        let text = fs::read_to_string(path).map_err(|e| format!("{path}: {e}"))?;
        let specs = parse_params(&text)?;
        if specs.len() < 15 {
            return Err(format!(
                "{path}: ожидается 15 параметров, найдено {}",
                specs.len()
            ));
        }
        let mut params = Self::default_params();
        params.source = path.to_string();
        params.specs = specs;
        params.regenerate(params.specs[2].min.max(1) as u64);
        Ok(params)
    }

    /// Сгенерировать параметры системы заново с заданным зерном
    /// (используется также директивой запуска ГПСЧ из командной строки).
    pub fn regenerate(&mut self, seed: u64) {
        self.random_seed = seed.max(1);
        let mut rng = StdRng::seed_from_u64(self.random_seed);
        let specs = self.specs.clone();
        self.task_size = specs[3].clone();
        self.commands = specs[4].clone();
        self.io_percent = specs[5].clone();
        self.io_duration = specs[6].clone();
        self.error_percent = specs[7].clone();
        self.total_memory = specs[0].generate(&mut rng) as i32;
        self.process_table_size = specs[1].generate(&mut rng).max(1) as usize;
        self.memory_words = specs[8].generate(&mut rng).max(1) as usize;
        self.io_processors = specs[9].generate(&mut rng).max(1) as usize;
        self.load_ticks = specs[10].generate(&mut rng).max(0) as u64;
        self.init_io_ticks = specs[11].generate(&mut rng).max(0) as u64;
        self.intr_io_ticks = specs[12].generate(&mut rng).max(0) as u64;
        self.switch_ticks = specs[13].generate(&mut rng).max(0) as u64;
        self.cpu_count = specs[14].generate(&mut rng).max(1) as usize;
    }

    /// Значения по умолчанию, если файл параметров недоступен.
    pub fn default_params() -> Self {
        let specs = vec![
            ParamSpec::new(1024, 4096, 128, "Размер ОЗУ модели"),
            ParamSpec::new(8, 8, 1, "Число слотов таблицы PSW"),
            ParamSpec::new(14, 14, 1, "Зерно генератора"),
            ParamSpec::new(96, 384, 16, "Размер задания"),
            ParamSpec::new(16, 48, 8, "Число команд задания"),
            ParamSpec::new(10, 30, 5, "Доля команд ввода-вывода"),
            ParamSpec::new(6, 6, 1, "Длительность ввода-вывода"),
            ParamSpec::new(2, 2, 1, "Доля ошибочных команд"),
            ParamSpec::new(64, 64, 1, "Число слов памяти"),
            ParamSpec::new(8, 8, 1, "Число процессоров ввода-вывода"),
            ParamSpec::new(2, 2, 1, "T_Load"),
            ParamSpec::new(1, 1, 1, "T_InitIO"),
            ParamSpec::new(1, 1, 1, "T_IntrIO"),
            ParamSpec::new(1, 1, 1, "T_next"),
            ParamSpec::new(3, 3, 1, "Число центральных процессоров"),
        ];
        Self {
            source: String::from("значения по умолчанию"),
            task_size: specs[3].clone(),
            commands: specs[4].clone(),
            io_percent: specs[5].clone(),
            io_duration: specs[6].clone(),
            error_percent: specs[7].clone(),
            specs,
            random_seed: 14,
            total_memory: 2048,
            process_table_size: 8,
            cpu_count: 3,
            io_processors: 8,
            memory_words: 64,
            load_ticks: 2,
            init_io_ticks: 1,
            intr_io_ticks: 1,
            switch_ticks: 1,
        }
    }
}
