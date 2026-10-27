/// Процессор ввода-вывода модели.
///
/// Каждый процессор обслуживает не более одной операции ввода-вывода.
/// По окончании обслуживания (остаток тактов равен нулю) генерируется
/// сигнал прерывания, который обрабатывает регулировщик.
#[derive(Debug)]
pub struct IoProcessor {
    pub id: u32,
    /// Слот таблицы PSW обслуживаемого процесса (None — процессор свободен).
    pub slot: Option<usize>,
    pub task_id: i32,
    /// Остаток тактов обслуживания.
    pub remaining: u32,
    /// Полная длительность операции.
    pub total: u32,
}

impl IoProcessor {
    pub fn new(id: u32) -> Self {
        Self {
            id,
            slot: None,
            task_id: 0,
            remaining: 0,
            total: 0,
        }
    }

    /// Сброс в исходное состояние.
    pub fn reset(&mut self) {
        self.slot = None;
        self.task_id = 0;
        self.remaining = 0;
        self.total = 0;
    }

    pub fn is_free(&self) -> bool {
        self.slot.is_none()
    }

    /// Начать обслуживание процесса.
    pub fn start(&mut self, slot: usize, task_id: i32, duration: u32) {
        self.slot = Some(slot);
        self.task_id = task_id;
        self.remaining = duration;
        self.total = duration;
    }

    /// Один такт обслуживания.
    pub fn tick(&mut self) {
        if self.remaining > 0 {
            self.remaining -= 1;
        }
    }

    /// Обслуживание завершено (сигнал прерывания).
    pub fn is_finished(&self) -> bool {
        self.slot.is_some() && self.remaining == 0
    }

    /// Освободить процессор.
    pub fn release(&mut self) {
        self.reset();
    }
}
