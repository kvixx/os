use crate::services::SimulationModel;
use std::io::Write;

// --------------------------------------------------------------------------
// Подпрограмма «Вывод постоянной информации»
// --------------------------------------------------------------------------

fn draw_constant_info() -> String {
    let mut buf = String::new();
    buf.push_str("МОДЕЛЬ ОПЕРАЦИОННОЙ СИСТЕМЫ | Лабораторная работа 9\n");
    buf.push_str("Вариант 1: FIFO, три ЦПр; директивы оператора\n");
    buf.push_str(&format!("{}\n", "=".repeat(74)));
    buf
}

// --------------------------------------------------------------------------
// Подпрограмма «Вывод параметров системы»
// --------------------------------------------------------------------------

fn draw_system_params(model: &SimulationModel) -> String {
    let mut buf = String::new();
    buf.push_str(&format!(
        "Такт: {:<6} Скорость: {:>6.1} такт/с   Дисциплина: {}\n",
        model.model_tick(),
        model.speed_ticks_per_second(),
        model.scheduler.name()
    ));
    buf.push_str(&format!(
        "Заданий: завершено {:<3} ошибок {:<3} уничтожено {}\n",
        model.completed_tasks,
        model.aborted_tasks,
        model.destroyed_tasks
    ));
    buf.push_str(&format!(
        "Резидентных: {}/{}   Память: {}/{}\n",
        model.resident_process_count(),
        model.process_table_capacity(),
        model.used_memory(),
        model.total_memory()
    ));
    buf.push_str(&format!(
        "Команд: {:<6} Иниц. IO: {:<5} Завершено IO: {:<5} Загрузка ЦП: {:>5.1}%\n",
        model.commands_executed,
        model.io_initiated,
        model.io_completed,
        model.cpu_utilization()
    ));
    buf.push_str(&format!(
        "Активных ЦПр: {}/{}   Выборов планировщика: {:<5} Слов памяти: {}\n",
        model.active_process_count(),
        model.cpu_count(),
        model.scheduler.selections(),
        model.operand_memory().len()
    ));
    buf.push_str(&format!(
        "Очередь готовности: {:<3} (средняя {:.2})   Производительность: {:.4} зад/такт\n",
        model.scheduler.len(),
        model.average_queue_len(),
        model.productivity()
    ));
    let (load, init_io, intr_io, switch) = model.system_costs();
    buf.push_str(&format!(
        "Параметры: {} (зерно {})\n",
        model.params.source,
        model.params.random_seed
    ));
    buf.push_str(&format!(
        "T_mono_ср: {:>5.1} такт   Производительность: {:>6.1}% от однопрограммной\n",
        model.average_mono(),
        model.productivity_relative()
    ));
    buf.push_str(&format!(
        "Системные затраты: T_Load={} T_InitIO={} T_IntrIO={} T_next={} (тактов)\n",
        load, init_io, intr_io, switch
    ));
    buf.push_str(&format!(
        "Автозагрузка: {:<4}  Выполнить-и-выключить: {:<3}  N_cmnd до {}\n",
        if model.is_autoload() { "ВКЛ" } else { "ВЫКЛ" },
        if model.finish_requested() { "ДА" } else { "НЕТ" },
        model.task_commands_max()
    ));
    buf
}

// --------------------------------------------------------------------------
// Подпрограмма «Вывод параметров из файла»
// --------------------------------------------------------------------------

fn draw_params_info(model: &SimulationModel) -> String {
    let params = &model.params;
    let mut buf = String::new();
    buf.push_str("Параметры из файла (диапазон / шаг / комментарий):\n");
    for (name, spec) in [
        ("Задание", &params.task_size),
        ("Команды", &params.commands),
        ("IO, %", &params.io_percent),
        ("IO, такт", &params.io_duration),
        ("Ошибки, %", &params.error_percent),
    ] {
        buf.push_str(&format!(
            "  {:<9} {:>5}..{:<5} шаг {:<4}\n      {}\n",
            name, spec.min, spec.max, spec.step, spec.comment
        ));
    }
    buf.push_str(&format!(
        "  Сгенерировано: ОЗУ {} ед.; слотов PSW {}; ЦПр {}; IO-процессоров {}\n",
        params.total_memory,
        params.process_table_size,
        params.cpu_count,
        params.io_processors
    ));
    buf
}

// --------------------------------------------------------------------------
// Подпрограмма «Вывод параметров нового задания»
// --------------------------------------------------------------------------

fn draw_new_task_params(model: &SimulationModel) -> String {
    match &model.new_task {
        None => String::from("Новое задание: нет\n"),
        Some(task) => format!(
            "Новое задание: ID {}, {} ед. памяти, {} команд, IO {}%\n",
            task.task_id, task.size, task.planned_commands, task.io_percent
        ),
    }
}

// --------------------------------------------------------------------------
// Подпрограмма «Вывод параметров центральных процессоров»
// --------------------------------------------------------------------------

fn draw_cpu_params(model: &SimulationModel) -> String {
    let mut buf = String::new();
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
    buf
}

// --------------------------------------------------------------------------
// Подпрограмма «Вывод параметров процессов»
// --------------------------------------------------------------------------

fn draw_process_params(model: &SimulationModel) -> String {
    let (load, _, _, _) = model.system_costs();
    let mut buf = String::new();
    buf.push_str("\nПараметры процессов:\n");
    buf.push_str(" №  ID  Разм  Счёт  Ком  Состояние        Tож  Tакт  Tio  Tсист  T_mono*\n");
    buf.push_str(&format!("{}\n", "-".repeat(72)));
    for process in model.process_table() {
        if let Some(p) = process {
            // Оценка T_mono по числу команд (N_cmnd + команда завершения).
            let mono_est = load + p.planned_commands + 1;
            let pointer = if p.slot as usize == model.selected_slot() {
                "►"
            } else {
                " "
            };
            buf.push_str(&format!(
                "{} {:>2}  {:>3}  {:>4}  {:>4}  {:>3}  {:<15}  {:>3}  {:>4}  {:>3}  {:>5}  {:>6}\n",
                pointer,
                p.slot + 1,
                p.task_id,
                p.size,
                p.command_counter,
                p.planned_commands,
                p.state.name(),
                p.ready_ticks,
                p.active_ticks,
                p.io_ticks,
                model.model_tick().saturating_sub(p.arrival_tick),
                mono_est
            ));
        }
    }
    buf
}

// --------------------------------------------------------------------------
// Подпрограмма «Вывод последовательности выполнения процессов»
// --------------------------------------------------------------------------

fn draw_dispatch_sequence(model: &SimulationModel) -> String {
    let mut buf = String::new();
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
    buf.push_str(&format!("Формула выбора: p* = first(Q_ready)  =>  p* = {}\n", next));
    let history: Vec<String> = model
        .scheduler
        .history()
        .iter()
        .map(|task_id| task_id.to_string())
        .collect();
    buf.push_str("Последовательность выполнения (до 16):\n");
    if history.is_empty() {
        buf.push_str("  —\n");
    } else {
        for chunk in history.chunks(8) {
            buf.push_str(&format!("  {}\n", chunk.join(" -> ")));
        }
    }
    buf
}

// --------------------------------------------------------------------------
// Подпрограмма «Вывод средних значений параметров процессов»
// --------------------------------------------------------------------------

fn draw_average_params(model: &SimulationModel) -> String {
    let mut buf = String::new();
    buf.push_str(&format!(
        "Средние: Tоб {:.1}, Tож {:.1}, Tакт {:.1}, Tio {:.1}, команд {:.1}\n",
        model.average_turnaround(),
        model.average_ready(),
        model.average_active(),
        model.average_io(),
        model.average_commands()
    ));
    buf.push_str(&format!(
        "T_mono_ср {:.1}; производительность {:.1}% от однопрограммной\n",
        model.average_mono(),
        model.productivity_relative()
    ));
    buf
}

// --------------------------------------------------------------------------
// Подпрограмма «Вывод журналов»
// --------------------------------------------------------------------------

fn draw_logs(model: &SimulationModel) -> String {
    let mut buf = String::new();
    buf.push_str("События регулировщика:\n");
    if model.event_log().is_empty() {
        buf.push_str("  —\n");
    } else {
        for line in model.event_log() {
            buf.push_str(&format!("  {}\n", line));
        }
    }
    buf
}

// --------------------------------------------------------------------------
// Подпрограмма «Вывод языка управления»
// --------------------------------------------------------------------------

fn draw_controls(show_help: bool) -> String {
    let mut buf = String::new();
    buf.push_str("Управление: [+] быстрее  [-] медленнее  [Q] выход  [H] справка\n");
    buf.push_str("Директивы оператора:\n");
    buf.push_str("  [N] пауза автозагрузки       [A] продолжить автозагрузку\n");
    buf.push_str("  [G] загрузить новое задание  [F] выполнить всё и выключить питание\n");
    buf.push_str("  [[]/[]] выбрать процесс       [S] приостановить/активизировать\n");
    buf.push_str("  [D] уничтожить выбранный      [M]/[B] изменить ОЗУ на +/-256\n");
    buf.push_str("  [T]/[Y] изменить число команд новых заданий (+/-8)\n");
    if show_help {
        buf.push_str("\nСправка: параметры читаются из params.txt; зерно ГПСЧ можно\n");
        buf.push_str("задать ключом командной строки: cargo run -- --seed 42\n");
        buf.push_str("Tож — время в состоянии «Готов», Tакт — время работы,\n");
        buf.push_str("Tio — время ввода-вывода, Tсист — время в системе,\n");
        buf.push_str("T_mono* — оценка времени выполнения в однопрограммной ОС\n");
        buf.push_str("(по числу команд задания).\n");
    }
    buf
}

// --------------------------------------------------------------------------
// Композиция экранной формы
// --------------------------------------------------------------------------

/// Формирование экранной формы из подпрограмм вывода.
pub fn render(model: &SimulationModel, show_help: bool) -> String {
    let mut buf = String::new();
    buf.push_str(&draw_constant_info());
    buf.push_str(&draw_system_params(model));
    buf.push_str(&draw_params_info(model));
    buf.push_str(&draw_new_task_params(model));
    buf.push_str(&draw_cpu_params(model));
    buf.push_str(&draw_process_params(model));
    buf.push_str(&draw_dispatch_sequence(model));
    buf.push_str(&draw_average_params(model));
    buf.push_str(&draw_logs(model));
    buf.push_str(&draw_controls(show_help));
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
