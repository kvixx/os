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
    /// Очередь готовности: (номер слота таблицы PSW, идентификатор задания).
    ready: VecDeque<(usize, i32)>,
    /// Идентификаторы процессов в порядке выборов (последние 16).
    history: VecDeque<i32>,
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
    pub fn enqueue(&mut self, slot: usize, task_id: i32) {
        if !self.ready.iter().any(|(s, _)| *s == slot) {
            self.ready.push_back((slot, task_id));
        }
    }

    /// GetNextProcessForCPU — планировщик.
    ///
    /// Формула выбора для варианта 1:
    ///     p* = first(Q_ready), если Q_ready ≠ ∅.
    pub fn get_next_process_for_cpu(&mut self) -> Option<(usize, i32)> {
        let (slot, task_id) = self.ready.pop_front()?;
        self.selections += 1;
        self.history.push_back(task_id);
        if self.history.len() > HISTORY_LIMIT {
            self.history.pop_front();
        }
        Some((slot, task_id))
    }

    pub fn len(&self) -> usize {
        self.ready.len()
    }

    /// Текущий список готовности (голова — следующий на обслуживание).
    pub fn ready(&self) -> &VecDeque<(usize, i32)> {
        &self.ready
    }

    /// Идентификаторы последних выбранных процессов (не более 16).
    pub fn history(&self) -> &VecDeque<i32> {
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
