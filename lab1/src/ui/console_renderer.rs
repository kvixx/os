use crate::services::SimulationModel;
use std::io::Write;

/// Отрисовка текущего состояния модели на экране.
pub fn draw(model: &SimulationModel, show_help: bool) {
    let mut buf = String::new();

    // ANSI: очистить экран и вернуть курсор в левый верхний угол.
    buf.push_str("\x1B[2J\x1B[1;1H");

    buf.push_str(&format!(
        "Такт модели: {:<8} Скорость: {:>7.1} такт/с\r\n",
        model.model_tick(),
        model.speed_ticks_per_second()
    ));

    let cpu_status = if model.cpu.is_waiting() {
        "ОЖИДАНИЕ"
    } else {
        "РАБОТА"
    };
    let active_id = model
        .active_process()
        .map(|p| p.task_id.to_string())
        .unwrap_or_else(|| "-".to_string());

    buf.push_str(&format!(
        "ЦП-{}: {:<10} Активный процесс: {:<3} Счётчик команд: {}\r\n",
        model.cpu.id, cpu_status, active_id, model.cpu.hardware_command_counter
    ));
    buf.push_str(&format!(
        "Память: занято {}/{}, свободно {}; PSW: {}/{}\r\n",
        model.used_memory(),
        model.total_memory(),
        model.free_memory(),
        model.resident_process_count(),
        model.process_table_capacity()
    ));

    buf.push_str("\r\n");
    buf.push_str(" №  ID    Размер   Счётчик команд   Состояние\r\n");
    buf.push_str(&format!("{}\r\n", "-".repeat(54)));

    for process in model.process_table() {
        if let Some(p) = process {
            buf.push_str(&format!(
                "{:>2}  {:>2}    {:>5}   {:>14}   {}\r\n",
                p.slot + 1,
                p.task_id,
                p.size,
                p.command_counter,
                p.state.name()
            ));
        }
    }

    buf.push_str("\r\n");
    match &model.new_task {
        None => buf.push_str("Новое задание: нет\r\n"),
        Some(task) => buf.push_str(&format!(
            "Новое задание: ID {}, {} ед. памяти\r\n",
            task.task_id, task.size
        )),
    }
    buf.push_str(
        "Управление: [+] быстрее  [-] медленнее  [R] перезапуск  [Q] выход  [H] справка\r\n",
    );

    if show_help {
        buf.push_str("\r\n");
        buf.push_str("Справка: +/- изменяют скорость на 10%; R инициализирует модель; Q завершает моделирование.\r\n");
        buf.push_str("Параметр запуска /? выводит эту справку и завершает программу.\r\n");
    }

    let mut stdout = std::io::stdout();
    let _ = stdout.write_all(buf.as_bytes());
    let _ = stdout.flush();
}

/// Очистка экрана и восстановление курсора.
pub fn cleanup() {
    print!("\x1B[2J\x1B[1;1H\x1B[?25h");
    let _ = std::io::stdout().flush();
}
