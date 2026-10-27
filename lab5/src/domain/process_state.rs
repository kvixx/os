/// Состояния процесса.
/// Часть состояний (приостановка, завершение) будет задействована
/// в последующих лабораторных работах.
#[allow(dead_code)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProcessState {
    Absent,
    Ready,
    Active,
    /// Инициализация процессора ввода-вывода.
    InitIo,
    BlockedIo,
    Suspended,
    Completed,
}

impl ProcessState {
    /// Русское имя состояния для вывода на экран.
    pub fn name(self) -> &'static str {
        match self {
            ProcessState::Absent => "Отсутствует",
            ProcessState::Ready => "Готов",
            ProcessState::Active => "Активен",
            ProcessState::InitIo => "Инициализация IO",
            ProcessState::BlockedIo => "Блокирован IO",
            ProcessState::Suspended => "Приостановлен",
            ProcessState::Completed => "Завершён",
        }
    }
}
