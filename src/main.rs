mod app;
mod storage;
mod ui;

use std::io;
use std::path::PathBuf;
use std::time::{Duration, Instant};

use app::{App, InputMode};
use chrono::Local;
use clap::Parser;
use crossterm::{
    event::{self, Event, KeyCode, KeyEventKind},
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
    ExecutableCommand,
};
use ratatui::backend::CrosstermBackend;
use ratatui::Terminal;

use storage::Todo;

#[derive(Parser)]
#[command(name = "tui-todo")]
struct Cli {
    #[command(subcommand)]
    command: Option<Command>,
}

#[derive(clap::Subcommand)]
enum Command {
    /// List all todos
    #[command(visible_alias = "ls")]
    List {
        /// Show only done todos
        #[arg(short, long)]
        done: bool,
        /// Show only pending todos
        #[arg(short, long)]
        pending: bool,
        /// Show internal IDs
        #[arg(long)]
        ids: bool,
    },
    /// Add a new todo
    #[command(visible_alias = "a")]
    Add {
        /// Todo description
        description: String,
    },
    /// Edit a todo description
    #[command(visible_alias = "e")]
    Edit {
        /// Todo ID
        id: u64,
        /// New description
        description: String,
    },
    /// Mark a todo as done
    #[command(visible_alias = "do")]
    Done {
        /// Todo ID
        id: u64,
    },
    /// Mark a todo as not done
    #[command(visible_alias = "un")]
    Undone {
        /// Todo ID
        id: u64,
    },
    /// Delete a todo
    #[command(visible_alias = "rm")]
    Delete {
        /// Todo ID
        id: u64,
    },
}

fn storage_path() -> PathBuf {
    PathBuf::from(std::env::var("HOME").unwrap_or_default()).join(".tui-todo.json")
}

fn main() -> io::Result<()> {
    let cli = Cli::parse();
    if let Some(cmd) = cli.command {
        run_cli(cmd);
        return Ok(());
    }
    run_tui()
}

fn run_cli(cmd: Command) {
    let path = storage_path();
    match cmd {
        Command::List { done, pending, ids } => {
            let mut todos = storage::load(&path);
            todos.sort_by_key(|t| t.id);
            let show_done = done || (!done && !pending);
            let show_pending = pending || (!done && !pending);
            let filtered: Vec<_> = todos
                .iter()
                .filter(|t| (show_done && t.done) || (show_pending && !t.done))
                .collect();
            if filtered.is_empty() {
                println!("No todos found.");
                return;
            }
            for t in filtered {
                let status = if t.done { "[x]" } else { "[ ]" };
                if ids {
                    println!("{}  {:>16}  {}  {}", status, t.id, t.created_at.format("%m-%d %H:%M"), t.description);
                } else {
                    println!("{}  {}  {}", status, t.created_at.format("%m-%d %H:%M"), t.description);
                }
            }
        }
        Command::Add { description } => {
            let mut todos = storage::load(&path);
            todos.push(Todo {
                id: storage::next_id(),
                description,
                done: false,
                created_at: Local::now().naive_local(),
            });
            storage::save(&path, &todos);
            println!("Added todo");
        }
        Command::Edit { id, description } => {
            let mut todos = storage::load(&path);
            match todos.iter_mut().find(|t| t.id == id) {
                Some(todo) => {
                    todo.description = description;
                    storage::save(&path, &todos);
                    println!("Updated todo #{}", id);
                }
                None => {
                    eprintln!("Todo #{} not found", id);
                    std::process::exit(1);
                }
            }
        }
        Command::Done { id } => {
            let mut todos = storage::load(&path);
            match todos.iter_mut().find(|t| t.id == id) {
                Some(todo) => {
                    todo.done = true;
                    storage::save(&path, &todos);
                    println!("Marked todo #{} as done", id);
                }
                None => {
                    eprintln!("Todo #{} not found", id);
                    std::process::exit(1);
                }
            }
        }
        Command::Undone { id } => {
            let mut todos = storage::load(&path);
            match todos.iter_mut().find(|t| t.id == id) {
                Some(todo) => {
                    todo.done = false;
                    storage::save(&path, &todos);
                    println!("Marked todo #{} as not done", id);
                }
                None => {
                    eprintln!("Todo #{} not found", id);
                    std::process::exit(1);
                }
            }
        }
        Command::Delete { id } => {
            let mut todos = storage::load(&path);
            let len_before = todos.len();
            todos.retain(|t| t.id != id);
            if todos.len() == len_before {
                eprintln!("Todo #{} not found", id);
                std::process::exit(1);
            }
            storage::save(&path, &todos);
            println!("Deleted todo #{}", id);
        }
    }
}

fn run_tui() -> io::Result<()> {
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    stdout.execute(EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;
    terminal.clear()?;

    let mut app = App::new(storage_path());
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
                KeyCode::Char('e') => {
                    if let Some(todo) = app.todos.get(app.selected_index) {
                        app.input_mode = InputMode::Editing;
                        app.input_buffer = todo.description.clone();
                    }
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
            InputMode::Editing => match key.code {
                KeyCode::Enter => {
                    app.edit_todo();
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
