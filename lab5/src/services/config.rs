use serde::Deserialize;
use std::fs;

#[derive(Debug, Deserialize)]
pub struct Config {
    pub model: ModelConfig,
    pub task: TaskConfig,
    pub memory: MemoryConfig,
    pub io: IoConfig,
    pub scheduler: SchedulerConfig,
}

#[derive(Debug, Deserialize)]
pub struct ModelConfig {
    pub memory_min: i32,
    pub memory_max: i32,
    pub process_table_size: usize,
    pub random_seed: u64,
}

/// Параметры заданий.
#[derive(Debug, Deserialize)]
pub struct TaskConfig {
    pub size_min: i32,
    pub size_max: i32,
    /// N_cmnd — диапазон числа команд задания.
    pub commands_min: u64,
    pub commands_max: u64,
    /// D_InOut — диапазон доли команд ввода-вывода, %.
    pub io_percent_min: u32,
    pub io_percent_max: u32,
    /// N_InOut — длительность команды ввода-вывода, тактов.
    pub io_duration: u32,
    /// Доля ошибочных команд, %.
    pub error_percent: u32,
}

/// Параметры аппаратной (операндной) памяти.
#[derive(Debug, Deserialize)]
pub struct MemoryConfig {
    pub words: usize,
}

/// Параметры процессоров ввода-вывода.
#[derive(Debug, Deserialize)]
pub struct IoConfig {
    pub processors: usize,
}

/// Параметры супервизора времени (вариант 1: FIFO, три ЦПр).
#[derive(Debug, Deserialize)]
pub struct SchedulerConfig {
    pub cpu_count: usize,
    pub algorithm: String,
}

impl Config {
    /// Значения по умолчанию, если файл конфигурации отсутствует.
    pub fn default_config() -> Self {
        Self {
            model: ModelConfig {
                memory_min: 1024,
                memory_max: 4096,
                process_table_size: 8,
                random_seed: 14,
            },
            task: TaskConfig {
                size_min: 96,
                size_max: 384,
                commands_min: 16,
                commands_max: 48,
                io_percent_min: 10,
                io_percent_max: 30,
                io_duration: 6,
                error_percent: 2,
            },
            memory: MemoryConfig { words: 64 },
            io: IoConfig { processors: 8 },
            scheduler: SchedulerConfig {
                cpu_count: 3,
                algorithm: String::from("FIFO"),
            },
        }
    }

    /// Загрузка из файла; при любой ошибке — значения по умолчанию.
    pub fn load(path: &str) -> Self {
        match fs::read_to_string(path) {
            Ok(text) => toml::from_str(&text).unwrap_or_else(|e| {
                eprintln!("Ошибка разбора {}: {e}. Беру значения по умолчанию.", path);
                Self::default_config()
            }),
            Err(_) => {
                eprintln!("Файл {} не найден. Беру значения по умолчанию.", path);
                Self::default_config()
            }
        }
    }
}
