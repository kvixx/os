use crate::services::SimulationModel;
use std::io::Write;

/// Формирование экранной формы в виде строки.
/// В отдельных подпрограммах выводятся: параметры системы, параметры ЦПр,
/// список готовности, формула выбора, таблица процессов и справка.
pub fn render(model: &SimulationModel, show_help: bool) -> String {
    let mut buf = String::new();

    buf.push_str("МОДЕЛЬ ОПЕРАЦИОННОЙ СИСТЕМЫ | Лабораторная работа 3\n");
    buf.push_str("Вариант 1: FIFO (без приоритетов), три центральных процессора\n");
    buf.push_str(&format!("{}\n", "=".repeat(74)));

    // ---- Параметры системы ----
    buf.push_str(&format!(
        "Такт модели: {:<8} Скорость: {:>7.1} такт/с\n",
        model.model_tick(),
        model.speed_ticks_per_second()
    ));
    buf.push_str(&format!(
        "Дисциплина: {:<6} Выборов планировщика: {:<4} Активных ЦПр: {}/{}\n",
        model.scheduler.name(),
        model.scheduler.selections(),
        model.active_process_count(),
        model.cpu_count()
    ));

    // ---- Параметры центральных процессоров ----
    for (idx, cpu) in model.cpus.iter().enumerate() {
        let status = if cpu.is_waiting() { "ОЖИДАНИЕ" } else { "РАБОТА" };
        let active = match model.active_process_on(idx) {
            Some(p) => format!("ID {}", p.task_id),
            None => String::from("—"),
        };
        buf.push_str(&format!(
            "ЦП-{}: {:<9} Активный процесс: {:<6} Счётчик команд: {}\n",
            cpu.id, status, active, cpu.hardware_command_counter
        ));
    }

    // ---- Список готовности ----
    let queue: Vec<String> = model
        .scheduler
        .ready()
        .iter()
        .map(|slot| match model.process_by_slot(*slot) {
            Some(p) => format!("ID {}", p.task_id),
            None => String::from("?"),
        })
        .collect();
    let next = queue
        .first()
        .cloned()
        .unwrap_or_else(|| String::from("—"));
    buf.push_str(&format!(
        "Список готовности (FIFO): [{}]  мощность: {}\n",
        queue.join(", "),
        model.scheduler.len()
    ));

    // ---- Формула выбора следующего процесса ----
    buf.push_str(&format!(
        "Формула выбора: p* = first(Q_ready)  =>  p* = {}\n",
        next
    ));

    let history: Vec<String> = model
        .scheduler
        .history()
        .iter()
        .map(|slot| match model.process_by_slot(*slot) {
            Some(p) => p.task_id.to_string(),
            None => String::from("?"),
        })
        .collect();
    buf.push_str(&format!(
        "Последние выборы: {}\n",
        if history.is_empty() {
            String::from("—")
        } else {
            history.join(" -> ")
        }
    ));

    // ---- Память ----
    buf.push_str(&format!(
        "Память: занято {}/{}, свободно {}; PSW: {}/{}\n",
        model.used_memory(),
        model.total_memory(),
        model.free_memory(),
        model.resident_process_count(),
        model.process_table_capacity()
    ));

    // ---- Таблица слов состояния процессов ----
    buf.push_str("\n");
    buf.push_str(" №   ID   Размер   Счётчик команд   Состояние\n");
    buf.push_str(&format!("{}\n", "-".repeat(56)));
    for process in model.process_table() {
        if let Some(p) = process {
            buf.push_str(&format!(
                "{:>2}  {:>3}   {:>6}   {:>14}   {}\n",
                p.slot + 1,
                p.task_id,
                p.size,
                p.command_counter,
                p.state.name()
            ));
        }
    }

    // ---- Новое задание и управление ----
    buf.push_str("\n");
    match &model.new_task {
        None => buf.push_str("Новое задание: нет\n"),
        Some(task) => buf.push_str(&format!(
            "Новое задание: ID {}, {} ед. памяти\n",
            task.task_id, task.size
        )),
    }
    buf.push_str(
        "Управление: [+] быстрее  [-] медленнее  [R] перезапуск  [Q] выход  [H] справка\n",
    );

    if show_help {
        buf.push_str("\n");
        buf.push_str("Справка: +/- изменяют скорость на 10%; R инициализирует модель;\n");
        buf.push_str("Q завершает моделирование; H показывает эту справку.\n");
        buf.push_str("Вариант 1: планировщик FIFO без приоритетов, три ЦПр.\n");
    }

    buf
}

/// Отрисовка текущего состояния модели на экране (с очисткой экрана).
pub fn draw(model: &SimulationModel, show_help: bool) {
    let mut buf = String::from("\x1B[2J\x1B[1;1H");
    buf.push_str(&render(model, show_help).replace('\n', "\r\n"));

    let mut stdout = std::io::stdout();
    let _ = stdout.write_all(buf.as_bytes());
    let _ = stdout.flush();
}

/// Очистка экрана и восстановление курсора.
pub fn cleanup() {
    print!("\x1B[2J\x1B[1;1H\x1B[?25h");
    let _ = std::io::stdout().flush();
}
