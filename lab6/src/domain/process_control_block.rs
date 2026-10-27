use super::{Command, CommandType, ProcessState};

/// Слово состояния процесса (PSW) модели ОС.
///
/// Хранит сохраняемую часть контекста процесса: счётчик команд,
/// состояние, параметры задания и контекст выполняемой команды.
#[derive(Debug, Clone)]
pub struct ProcessControlBlock {
    pub slot: i32,
    pub task_id: i32,
    pub size: i32,
    pub command_counter: u64,
    pub state: ProcessState,

    // Параметры задания (лабораторная работа 4).
    /// N_cmnd — число команд задания.
    pub planned_commands: u64,
    /// D_InOut — доля команд ввода-вывода, %.
    pub io_percent: u32,
    /// N_InOut — длительность команды ввода-вывода, тактов.
    pub io_duration: u32,
    /// Остаток тактов ввода-вывода.
    pub remaining_io: u32,

    // Контекст выполнения команды.
    pub current_command: Option<Command>,
    pub last_command: Option<Command>,
    pub op1: i32,
    pub op2: i32,
    pub result: i32,

    // Временные параметры (лабораторная работа 6).
    /// Такт загрузки задания в систему.
    pub arrival_tick: u64,
    /// Время в состоянии «Готов», тактов.
    pub ready_ticks: u64,
    /// Время в состоянии «Активен», тактов.
    pub active_ticks: u64,
    /// Время в состоянии «Инициализация IO»/«Блокирован IO», тактов.
    pub io_ticks: u64,
}

impl ProcessControlBlock {
    pub fn new(task_id: i32, size: i32) -> Self {
        Self {
            slot: -1,
            task_id,
            size,
            command_counter: 0,
            state: ProcessState::Absent,
            planned_commands: 0,
            io_percent: 0,
            io_duration: 0,
            remaining_io: 0,
            current_command: None,
            last_command: None,
            op1: 0,
            op2: 0,
            result: 0,
            arrival_tick: 0,
            ready_ticks: 0,
            active_ticks: 0,
            io_ticks: 0,
        }
    }

    /// Описание выполняемой команды для индикации.
    pub fn command_name(&self) -> String {
        match self.current_command.or(self.last_command) {
            None => String::from("—"),
            Some(cmd) => match cmd.kind {
                CommandType::Compute => format!("вычисл. {}", cmd.opcode_name()),
                other => other.name().to_string(),
            },
        }
    }
}
