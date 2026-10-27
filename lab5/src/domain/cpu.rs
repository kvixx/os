/// Модель центрального процессора.
///
/// Активный процесс хранится в виде индекса в таблице PSW — это
/// идиоматично для Rust и позволяет обойтись без `Rc<RefCell<_>>`.
/// В варианте 1 моделируется три центральных процессора.
#[derive(Debug)]
pub struct Cpu {
    pub id: u32,
    pub active_slot: Option<usize>,
    pub hardware_command_counter: u64,
}

impl Cpu {
    pub fn new(id: u32) -> Self {
        Self {
            id,
            active_slot: None,
            hardware_command_counter: 0,
        }
    }

    /// Сброс процессора в исходное состояние («Ожидание»).
    pub fn reset(&mut self) {
        self.active_slot = None;
        self.hardware_command_counter = 0;
    }

    /// Процессор простаивает (нет активного процесса).
    pub fn is_waiting(&self) -> bool {
        self.active_slot.is_none()
    }
}

impl Default for Cpu {
    fn default() -> Self {
        Self::new(1)
    }
}
