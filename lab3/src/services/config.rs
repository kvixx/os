use serde::Deserialize;
use std::fs;

#[derive(Debug, Deserialize)]
pub struct Config {
    pub model: ModelConfig,
    pub task: TaskConfig,
    pub scheduler: SchedulerConfig,
}

#[derive(Debug, Deserialize)]
pub struct ModelConfig {
    pub memory_min: i32,
    pub memory_max: i32,
    pub process_table_size: usize,
    pub random_seed: u64,
}

#[derive(Debug, Deserialize)]
pub struct TaskConfig {
    pub size_min: i32,
    pub size_max: i32,
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
            },
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
