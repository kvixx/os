use crate::services::SimulationModel;
use std::io::Write;

/// Формирование экранной формы в виде строки.
pub fn render(model: &SimulationModel, show_help: bool) -> String {
    let mut buf = String::new();

    buf.push_str("МОДЕЛЬ ОПЕРАЦИОННОЙ СИСТЕМЫ | Лабораторная работа 5\n");
    buf.push_str("Вариант 1: FIFO, три ЦПр; прерывания и регулировщик\n");
    buf.push_str(&format!("{}\n", "=".repeat(74)));

    // ---- Параметры системы ----
    buf.push_str(&format!(
        "Такт модели: {:<7} Скорость: {:>7.1} такт/с   Дисциплина: {}\n",
        model.model_tick(),
        model.speed_ticks_per_second(),
        model.scheduler.name()
    ));
    buf.push_str(&format!(
        "Команд: {:<5} Иниц.IO: {:<4} Завершено IO: {:<4} Завершено заданий: {:<3} Ошибок: {}\n",
        model.commands_executed,
        model.io_initiated,
        model.io_completed,
        model.completed_tasks,
        model.aborted_tasks
    ));
    let history: Vec<String> = model
        .scheduler
        .history()
        .iter()
        .map(|task_id| task_id.to_string())
        .collect();
    buf.push_str(&format!(
        "Выборов: {:<4} Активных ЦПр: {}/{}\n",
        model.scheduler.selections(),
        model.active_process_count(),
        model.cpu_count()
    ));
    buf.push_str("История выборов:\n");
    if history.is_empty() {
        buf.push_str("  —\n");
    } else {
        for chunk in history.chunks(8) {
            buf.push_str(&format!("  {}\n", chunk.join(" -> ")));
        }
    }

    // ---- Параметры центральных процессоров и выполняемые команды ----
    for (idx, cpu) in model.cpus.iter().enumerate() {
        let status = if cpu.is_waiting() { "ОЖИДАНИЕ" } else { "РАБОТА" };
        let (task, command) = match model.active_process_on(idx) {
            Some(p) => (format!("ID {:<3}", p.task_id), p.command_name()),
            None => (String::from("—   "), String::from("—")),
        };
        buf.push_str(&format!(
            "ЦП-{}: {:<9} {:<7} команда: {:<14} счётчик: {}\n",
            cpu.id, status, task, command, cpu.hardware_command_counter
        ));
    }

    // ---- Процессоры ввода-вывода ----
    let io_busy: Vec<String> = model
        .io_processors
        .iter()
        .filter(|io| !io.is_free())
        .map(|io| format!("ID {}({}/{})", io.task_id, io.remaining, io.total))
        .collect();
    buf.push_str(&format!(
        "IO-процессоры: занято {}/{}\n",
        io_busy.len(),
        model.io_processors.len()
    ));
    if io_busy.is_empty() {
        buf.push_str("  —\n");
    } else {
        for chunk in io_busy.chunks(6) {
            buf.push_str(&format!("  {}\n", chunk.join(", ")));
        }
    }

    // ---- Список готовности ----
    let queue: Vec<String> = model
        .scheduler
        .ready()
        .iter()
        .map(|(_, task_id)| format!("ID {}", task_id))
        .collect();
    let next = queue
        .first()
        .cloned()
        .unwrap_or_else(|| String::from("—"));
    buf.push_str(&format!(
        "Очередь FIFO (мощность {}):\n",
        model.scheduler.len()
    ));
    if queue.is_empty() {
        buf.push_str("  —\n");
    } else {
        for chunk in queue.chunks(8) {
            buf.push_str(&format!("  {}\n", chunk.join(", ")));
        }
    }
    buf.push_str(&format!(
        "Формула выбора: p* = first(Q_ready)  =>  p* = {}\n",
        next
    ));

    // ---- Память ----
    buf.push_str(&format!(
        "Память: занято {}/{}, свободно {}; PSW: {}/{}; слов памяти: {}\n",
        model.used_memory(),
        model.total_memory(),
        model.free_memory(),
        model.resident_process_count(),
        model.process_table_capacity(),
        model.operand_memory().len()
    ));

    // ---- Таблица слов состояния процессов ----
    buf.push_str("\n");
    buf.push_str(" №   ID   Размер   Счётчик   Команд   Состояние\n");
    buf.push_str(&format!("{}\n", "-".repeat(60)));
    for process in model.process_table() {
        if let Some(p) = process {
            buf.push_str(&format!(
                "{:>2}  {:>3}   {:>6}   {:>7}   {:>6}   {}\n",
                p.slot + 1,
                p.task_id,
                p.size,
                p.command_counter,
                p.planned_commands,
                p.state.name()
            ));
        }
    }

    // ---- События регулировщика ----
    buf.push_str("События регулировщика:\n");
    if model.event_log().is_empty() {
        buf.push_str("  —\n");
    } else {
        for line in model.event_log() {
            buf.push_str(&format!("  {}\n", line));
        }
    }

    // ---- Новое задание и управление ----
    buf.push_str("\n");
    match &model.new_task {
        None => buf.push_str("Новое задание: нет\n"),
        Some(task) => buf.push_str(&format!(
            "Новое задание: ID {}, {} ед. памяти, {} команд, IO {}%\n",
            task.task_id, task.size, task.planned_commands, task.io_percent
        )),
    }
    buf.push_str(
        "Управление: [+] быстрее  [-] медленнее  [R] перезапуск  [Q] выход  [H] справка\n",
    );

    if show_help {
        buf.push_str("\n");
        buf.push_str("Справка: +/- изменяют скорость на 10%; R инициализирует модель;\n");
        buf.push_str("Q завершает моделирование; H показывает эту справку.\n");
        buf.push_str("Команды: вычислительные, ввод-вывод, завершение, ошибочные.\n");
        buf.push_str("Вариант 1: планировщик FIFO, три ЦПр, процессоры ввода-вывода.\n");
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
