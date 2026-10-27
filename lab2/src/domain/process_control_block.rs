use super::ProcessState;

/// Слово состояния процесса (PSW) модели ОС.
#[derive(Debug, Clone)]
pub struct ProcessControlBlock {
    pub slot: i32,
    pub task_id: i32,
    pub size: i32,
    pub command_counter: u64,
    pub state: ProcessState,

    // Поля зарезервированы для лабораторных 3–5.
    pub base_priority: i32,
    pub dynamic_priority: i32,
    pub remaining_quantum: i32,
}

impl ProcessControlBlock {
    pub fn new(task_id: i32, size: i32) -> Self {
        Self {
            slot: -1,
            task_id,
            size,
            command_counter: 0,
            state: ProcessState::Absent,
            base_priority: 0,
            dynamic_priority: 0,
            remaining_quantum: 0,
        }
    }
}
