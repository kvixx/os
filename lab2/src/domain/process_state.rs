/// Состояния процесса, расширяемые в последующих лабораторных работах.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProcessState {
    Absent,
    Ready,
    Active,
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
            ProcessState::BlockedIo => "Блокирован IO",
            ProcessState::Suspended => "Приостановлен",
            ProcessState::Completed => "Завершён",
        }
    }
}
