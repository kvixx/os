use std::collections::VecDeque;

/// Сколько последних выборов планировщика хранить для индикации.
const HISTORY_LIMIT: usize = 16;

/// Планировщик (супервизор времени).
///
/// Вариант 1 — FIFO без приоритетов: единый список готовности
/// организован как очередь; процесс, ставший готовым, помещается в
/// конец очереди, а свободный центральный процессор получает
/// процесс из головы очереди (First In — First Out).
#[derive(Debug)]
pub struct Scheduler {
    name: String,
    ready: VecDeque<usize>,
    history: VecDeque<usize>,
    selections: u64,
}

impl Scheduler {
    pub fn new(name: &str) -> Self {
        Self {
            name: name.to_string(),
            ready: VecDeque::new(),
            history: VecDeque::new(),
            selections: 0,
        }
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    /// Сброс списка готовности (при инициализации модели).
    pub fn clear(&mut self) {
        self.ready.clear();
        self.history.clear();
        self.selections = 0;
    }

    /// Поместить процесс в конец списка готовности.
    /// Дисциплина FIFO: новые процессы не могут обогнать уже стоящие.
    pub fn enqueue(&mut self, slot: usize) {
        if !self.ready.contains(&slot) {
            self.ready.push_back(slot);
        }
    }

    /// GetNextProcessForCPU — планировщик.
    ///
    /// Формула выбора для варианта 1:
    ///     p* = first(Q_ready), если Q_ready ≠ ∅.
    pub fn get_next_process_for_cpu(&mut self) -> Option<usize> {
        let slot = self.ready.pop_front()?;
        self.selections += 1;
        self.history.push_back(slot);
        if self.history.len() > HISTORY_LIMIT {
            self.history.pop_front();
        }
        Some(slot)
    }

    pub fn len(&self) -> usize {
        self.ready.len()
    }

    /// Текущий список готовности (голова — следующий на обслуживание).
    pub fn ready(&self) -> &VecDeque<usize> {
        &self.ready
    }

    /// Последние выборы планировщика (не более 16).
    pub fn history(&self) -> &VecDeque<usize> {
        &self.history
    }

    pub fn selections(&self) -> u64 {
        self.selections
    }
}

impl Default for Scheduler {
    fn default() -> Self {
        Self::new("FIFO")
    }
}
