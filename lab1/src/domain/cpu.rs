/// Модель центрального процессора.
///
/// Активный процесс хранится в виде индекса в таблице PSW — это
/// идиоматично для Rust и позволяет обойтись без `Rc<RefCell<_>>`.
#[derive(Debug)]
pub struct Cpu {
    pub id: u32,
    pub active_slot: Option<usize>,
    pub hardware_command_counter: u64,
}

impl Cpu {
    pub fn new() -> Self {
        Self {
            id: 1,
            active_slot: None,
            hardware_command_counter: 0,
        }
    }

    pub fn is_waiting(&self) -> bool {
        self.active_slot.is_none()
    }
}

impl Default for Cpu {
    fn default() -> Self {
        Self::new()
    }
}
