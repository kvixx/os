/// Тип команды процесса.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CommandType {
    /// Вычислительная команда (выполняется за один такт).
    Compute,
    /// Команда обращения к устройству ввода-вывода.
    Io,
    /// Команда завершения задания.
    Exit,
    /// Ошибочная команда (для контроля).
    Error,
}

impl CommandType {
    pub fn name(self) -> &'static str {
        match self {
            CommandType::Compute => "вычислительная",
            CommandType::Io => "ввод-вывод",
            CommandType::Exit => "завершение",
            CommandType::Error => "ошибочная",
        }
    }
}

/// Команда процесса: код операции и математические адреса операндов.
#[derive(Debug, Clone, Copy)]
pub struct Command {
    pub kind: CommandType,
    /// Код операции: b'+', b'-', b'*', b'/' (0 — для невычислительных команд).
    pub opcode: u8,
    /// Математический адрес первого операнда.
    pub addr1: usize,
    /// Математический адрес второго операнда (он же адрес результата).
    pub addr2: usize,
}

impl Command {
    pub fn new(kind: CommandType, opcode: u8, addr1: usize, addr2: usize) -> Self {
        Self {
            kind,
            opcode,
            addr1,
            addr2,
        }
    }

    /// Символьная запись кода операции.
    pub fn opcode_name(self) -> &'static str {
        match self.opcode {
            b'+' => "+",
            b'-' => "-",
            b'*' => "*",
            b'/' => "/",
            _ => "—",
        }
    }
}
