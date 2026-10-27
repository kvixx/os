mod domain;
mod services;
mod ui;

use crossterm::event::{self, Event, KeyCode, KeyEventKind};
use crossterm::terminal::{disable_raw_mode, enable_raw_mode};
use services::SimulationModel;
use std::env;
use std::time::{Duration, Instant};

use crate::services::Config;

fn print_help() {
    println!("Модель ОС — лабораторная работа 5 (вариант 1: FIFO, три ЦПр)");
    println!("Запуск: cargo run");
    println!(
        "Клавиши: + увеличить скорость на 10%, - уменьшить на 10%, R перезапуск, Q выход, H справка."
    );
    println!("Режим отчёта: cargo run -- --snapshot [число тактов].");
}

fn main() -> std::io::Result<()> {
    let args: Vec<String> = env::args().collect();

    if args.iter().any(|a| a == "/?" || a == "--help" || a == "-h") {
        print_help();
        return Ok(());
    }

    let cfg = Config::load("config.toml");
    let mut model = SimulationModel::from_config(&cfg);
    model.initialize();

    // Режим снимка экрана для отчёта: выполнить такты и вывести экранную
    // форму без ANSI-управления (используется генератором отчёта).
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
                    match key.code {
                        KeyCode::Char('+') | KeyCode::Char('=') => model.change_speed(1.10),
                        KeyCode::Char('-') | KeyCode::Char('_') => model.change_speed(0.90),
                        KeyCode::Char('r') | KeyCode::Char('R') => model.initialize(),
                        KeyCode::Char('h') | KeyCode::Char('H') => show_help = !show_help,
                        KeyCode::Char('q') | KeyCode::Char('Q') | KeyCode::Esc => running = false,
                        _ => {}
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

        if now.duration_since(last_draw_at) >= Duration::from_millis(100) {
            ui::draw(&model, show_help);
            last_draw_at = now;
        }

        std::thread::sleep(Duration::from_millis(10));
    }

    disable_raw_mode()?;
    ui::cleanup();
    println!("Моделирование завершено.");
    Ok(())
}
