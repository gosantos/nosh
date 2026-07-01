mod app;
mod storage;
mod ui;

use std::io;
use std::path::PathBuf;
use std::time::{Duration, Instant};

use app::{App, InputMode};
use crossterm::{
    event::{self, Event, KeyCode, KeyEventKind},
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
    ExecutableCommand,
};
use ratatui::backend::CrosstermBackend;
use ratatui::Terminal;

fn main() -> io::Result<()> {
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    stdout.execute(EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;
    terminal.clear()?;

    let storage_path = PathBuf::from(
        std::env::var("HOME").unwrap_or_default(),
    )
    .join(".tui-todo.json");

    let mut app = App::new(storage_path);
    let tick_rate = Duration::from_millis(250);
    let mut last_tick = Instant::now();

    while !app.should_quit {
        terminal.draw(|f| ui::draw(f, &mut app))?;

        let timeout = tick_rate.saturating_sub(last_tick.elapsed());
        if event::poll(timeout)? {
            handle_event(&mut app)?;
        }
        if last_tick.elapsed() >= tick_rate {
            last_tick = Instant::now();
        }
    }

    disable_raw_mode()?;
    terminal.backend_mut().execute(LeaveAlternateScreen)?;
    Ok(())
}

fn handle_event(app: &mut App) -> io::Result<()> {
    if let Event::Key(key) = event::read()? {
        if key.kind != KeyEventKind::Press {
            return Ok(());
        }

        match app.input_mode {
            InputMode::Normal => match key.code {
                KeyCode::Char('q') => app.should_quit = true,
                KeyCode::Char('n') => {
                    app.input_mode = InputMode::Adding;
                    app.input_buffer.clear();
                }
                KeyCode::Char('d') => app.delete_selected(),
                KeyCode::Char(' ') => app.toggle_done(),
                KeyCode::Up | KeyCode::Char('k') => app.move_up(),
                KeyCode::Down | KeyCode::Char('j') => app.move_down(),
                KeyCode::Esc => app.should_quit = true,
                _ => {}
            },
            InputMode::Adding => match key.code {
                KeyCode::Enter => {
                    app.add_todo();
                    app.input_mode = InputMode::Normal;
                }
                KeyCode::Esc => {
                    app.input_buffer.clear();
                    app.input_mode = InputMode::Normal;
                }
                KeyCode::Backspace => {
                    app.input_buffer.pop();
                }
                KeyCode::Char(c) => {
                    app.input_buffer.push(c);
                }
                _ => {}
            },
        }
    }
    Ok(())
}
