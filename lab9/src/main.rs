mod domain;
mod services;
mod ui;

use crossterm::event::{self, Event, KeyCode, KeyEventKind};
use crossterm::terminal::{disable_raw_mode, enable_raw_mode};
use services::SimulationModel;
use std::env;
use std::time::{Duration, Instant};

use crate::services::Params;

fn print_help() {
    println!("Модель ОС — лабораторная работа 9 (вариант 1: FIFO, три ЦПр)");
    println!("Запуск: cargo run (параметры читаются из params.txt)");
    println!("Ключи командной строки:");
    println!("  --seed N          запустить ГПСЧ с заданного значения");
    println!("  --snapshot [N]    вывести экранную форму после N тактов");
    println!("  --keys \"...\"       выполнить директивы оператора, например \"N G [ S\"");
    println!("  --until-finish    выполнять задания до полного завершения");
    println!("Директивы: +,- скорость; N/A пауза/продолжить автозагрузку;");
    println!("G загрузить задание; [/] выбор процесса; S приостановить/активизировать;");
    println!("D уничтожить; F выполнить всё и выключить питание; M/B изменить ОЗУ;");
    println!("T/Y изменить число команд новых заданий; Q выход; H справка.");
}

/// Обработка одной директивы оператора.
/// Возвращает false, если запрошено выключение питания (выход).
fn handle_directive(model: &mut SimulationModel, ch: char, show_help: &mut bool) -> bool {
    match ch {
        '+' | '=' => model.change_speed(1.10),
        '-' | '_' => model.change_speed(0.90),
        'r' | 'R' => model.initialize(),
        'h' | 'H' => *show_help = !*show_help,
        'n' | 'N' => model.set_autoload(false),
        'a' | 'A' => model.set_autoload(true),
        'g' | 'G' => {
            model.load_one_task();
        }
        '[' => model.select_prev(),
        ']' => model.select_next(),
        's' | 'S' => {
            model.suspend_or_resume_selected();
        }
        'd' | 'D' => {
            model.destroy_selected();
        }
        'f' | 'F' => model.request_finish(),
        'm' | 'M' => {
            model.change_memory(256);
        }
        'b' | 'B' => {
            model.change_memory(-256);
        }
        't' | 'T' => {
            model.change_task_commands(8);
        }
        'y' | 'Y' => {
            model.change_task_commands(-8);
        }
        'q' | 'Q' | '\x1b' => return false,
        _ => {}
    }
    true
}

/// Выполнение директив, заданных строкой ключа --keys.
fn apply_key_script(model: &mut SimulationModel, script: &str) {
    let mut show_help = false;
    for ch in script.chars() {
        if ch.is_whitespace() {
            continue;
        }
        handle_directive(model, ch, &mut show_help);
    }
}

fn main() -> std::io::Result<()> {
    let args: Vec<String> = env::args().collect();

    if args.iter().any(|a| a == "/?" || a == "--help" || a == "-h") {
        print_help();
        return Ok(());
    }

    let mut params = match Params::load("params.txt") {
        Ok(params) => params,
        Err(error) => {
            eprintln!("Ошибка чтения параметров: {error}. Беру значения по умолчанию.");
            Params::default_params()
        }
    };

    // Директива запуска ГПСЧ с заданного значения по ключу из командной строки.
    if let Some(pos) = args.iter().position(|a| a == "--seed") {
        if let Some(seed) = args.get(pos + 1).and_then(|value| value.parse::<u64>().ok()) {
            params.regenerate(seed);
            println!("ГПСЧ запущен с зерном {seed}.");
        }
    }

    let mut model = SimulationModel::from_params(&params);
    model.initialize();

    // Выполнение директив оператора, переданных ключом --keys.
    if let Some(pos) = args.iter().position(|a| a == "--keys") {
        if let Some(script) = args.get(pos + 1) {
            apply_key_script(&mut model, script);
        }
    }

    if let Some(pos) = args.iter().position(|a| a == "--snapshot") {
        let ticks = args
            .get(pos + 1)
            .and_then(|value| value.parse::<u32>().ok())
            .unwrap_or(12);
        for _ in 0..ticks {
            model.execute_one_tick();
        }
        let show_help = args.iter().any(|a| a == "--help-screen");
        print!("{}", ui::render(&model, show_help));
        return Ok(());
    }

    // Режим «выполнить всё и выключить питание»: используется для отчёта
    // (--keys F --until-finish) и в интерактивном режиме.
    if args.iter().any(|a| a == "--until-finish") {
        let mut ticks = 0u32;
        while !model.is_finish_complete() && ticks < 100_000 {
            model.execute_one_tick();
            ticks += 1;
        }
        print!("{}", ui::render(&model, false));
        println!("Все загруженные задания выполнены за {ticks} тактов. Питание выключено.");
        return Ok(());
    }

    enable_raw_mode()?;
    print!("\x1B[?25l");

    let mut last_tick_at = Instant::now();
    let mut last_draw_at = Instant::now() - Duration::from_secs(1);
    let mut show_help = false;
    let mut running = true;

    while running {
        while event::poll(Duration::from_millis(0))? {
            if let Event::Key(key) = event::read()? {
                if key.kind == KeyEventKind::Press {
                    let ch = match key.code {
                        KeyCode::Char(c) => Some(c),
                        KeyCode::Esc => Some('\x1b'),
                        KeyCode::Up => Some('['),
                        KeyCode::Down => Some(']'),
                        _ => None,
                    };
                    if let Some(ch) = ch {
                        running = handle_directive(&mut model, ch, &mut show_help);
                    }
                }
            }
        }

        let now = Instant::now();
        let tick_period = Duration::from_secs_f64(1.0 / model.speed_ticks_per_second());
        while now.duration_since(last_tick_at) >= tick_period {
            model.execute_one_tick();
            last_tick_at += tick_period;
        }

        // Директива «выполнить загруженные задания и выключить питание».
        if model.is_finish_complete() {
            running = false;
        }

        if now.duration_since(last_draw_at) >= Duration::from_millis(100) {
            ui::draw(&model, show_help);
            last_draw_at = now;
        }

        std::thread::sleep(Duration::from_millis(10));
    }

    disable_raw_mode()?;
    ui::cleanup();
    if model.finish_requested() {
        println!("Все загруженные задания выполнены. Питание выключено.");
    } else {
        println!("Моделирование завершено.");
    }
    Ok(())
}
