use super::ProcessState;

/// Слово состояния процесса (PSW) модели ОС.
///
/// Хранит «сохраняемую часть» контекста процесса: счётчик команд и
/// состояние. Аппаратный счётчик команд находится в модели ЦП и
/// восстанавливается из PSW при передаче процесса на процессор.
#[derive(Debug, Clone)]
pub struct ProcessControlBlock {
    pub slot: i32,
    pub task_id: i32,
    pub size: i32,
    pub command_counter: u64,
    pub state: ProcessState,
}

impl ProcessControlBlock {
    pub fn new(task_id: i32, size: i32) -> Self {
        Self {
            slot: -1,
            task_id,
            size,
            command_counter: 0,
            state: ProcessState::Absent,
        }
    }
}
