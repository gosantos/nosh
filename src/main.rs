mod app;
mod markdown;
mod storage;
mod ui;

use std::io;
use std::path::PathBuf;
use std::time::{Duration, Instant};

use app::{App, InputMode, NoteMode, Panel, View};
use chrono::Local;
use clap::Parser;
use crossterm::{
    event::{self, Event, KeyCode, KeyEventKind},
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
    ExecutableCommand,
};
use ratatui::backend::CrosstermBackend;
use ratatui::Terminal;

use storage::{Note, Todo};

#[derive(Parser)]
#[command(name = "tui-todo")]
struct Cli {
    #[command(subcommand)]
    command: Option<Command>,
}

#[derive(clap::Subcommand)]
enum Command {
    #[command(visible_alias = "ls")]
    List {
        #[arg(short, long)]
        done: bool,
        #[arg(short, long)]
        pending: bool,
        #[arg(long)]
        ids: bool,
        #[arg(short, long)]
        archived: bool,
    },
    #[command(visible_alias = "a")]
    Add { description: String },
    #[command(visible_alias = "e")]
    Edit { id: u64, description: String },
    #[command(visible_alias = "do")]
    Done { id: u64 },
    #[command(visible_alias = "un")]
    Undone { id: u64 },
    #[command(visible_alias = "rm")]
    Delete { id: u64 },
    Archive { id: u64 },
    #[command(visible_alias = "ua")]
    Unarchive { id: u64 },
    #[command(visible_alias = "grep")]
    Search {
        query: String,
        #[arg(short, long)]
        archived: bool,
    },
    #[command(subcommand)]
    Note(NoteCommand),
}

#[derive(clap::Subcommand)]
enum NoteCommand {
    #[command(visible_alias = "ls")]
    List,
    #[command(visible_alias = "a")]
    New { title: String },
    #[command(visible_alias = "e")]
    Edit { id: u64 },
    #[command(alias = "show")]
    View { id: u64 },
    #[command(visible_alias = "rm")]
    Delete { id: u64 },
}

fn storage_path() -> PathBuf {
    PathBuf::from(std::env::var("HOME").unwrap_or_default()).join(".tui-todo.json")
}

fn notes_path() -> PathBuf {
    PathBuf::from(std::env::var("HOME").unwrap_or_default()).join(".tui-todo-notes.json")
}

fn open_editor(content: &str) -> io::Result<String> {
    let editor = std::env::var("EDITOR").unwrap_or_else(|_| "vim".to_string());
    let tmp = std::env::temp_dir().join(format!("tui-todo-note-{}.md", std::process::id()));
    std::fs::write(&tmp, content)?;
    let status = std::process::Command::new(&editor).arg(&tmp).status()?;
    let new_content = std::fs::read_to_string(&tmp)?;
    let _ = std::fs::remove_file(&tmp);
    if !status.success() {
        return Err(io::Error::new(io::ErrorKind::Other, "editor exited with error"));
    }
    Ok(new_content)
}

fn extract_title(content: &str) -> String {
    for line in content.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with("# ") {
            return trimmed[2..].trim().to_string();
        }
        if !trimmed.is_empty() {
            return trimmed.chars().take(60).collect();
        }
    }
    "Untitled".to_string()
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
    match cmd {
        Command::List { done, pending, ids, archived } => list_todos(done, pending, ids, archived),
        Command::Add { description } => add_todo(&description),
        Command::Edit { id, description } => edit_todo(id, &description),
        Command::Done { id } => mark_done(id),
        Command::Undone { id } => mark_undone(id),
        Command::Delete { id } => delete_todo(id),
        Command::Archive { id } => archive_todo(id),
        Command::Unarchive { id } => unarchive_todo(id),
        Command::Search { query, archived } => search_todos(&query, archived),
        Command::Note(note_cmd) => run_note_cmd(note_cmd),
    }
}

fn list_todos(done: bool, pending: bool, ids: bool, archived: bool) {
    let path = storage_path();
    let mut todos = storage::load(&path);
    todos.sort_by_key(|t| t.id);
    let show_done = done || (!done && !pending);
    let show_pending = pending || (!done && !pending);
    let filtered: Vec<_> = todos
        .iter()
        .filter(|t| t.archived == archived && ((show_done && t.done) || (show_pending && !t.done)))
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

fn add_todo(description: &str) {
    let path = storage_path();
    let mut todos = storage::load(&path);
    todos.push(Todo {
        id: storage::next_id(),
        description: description.to_string(),
        done: false,
        archived: false,
        created_at: Local::now().naive_local(),
    });
    storage::save(&path, &todos);
    println!("Added todo");
}

fn edit_todo(id: u64, description: &str) {
    let path = storage_path();
    let mut todos = storage::load(&path);
    match todos.iter_mut().find(|t| t.id == id) {
        Some(todo) => {
            todo.description = description.to_string();
            storage::save(&path, &todos);
            println!("Updated todo #{}", id);
        }
        None => {
            eprintln!("Todo #{} not found", id);
            std::process::exit(1);
        }
    }
}

fn mark_done(id: u64) {
    let path = storage_path();
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

fn mark_undone(id: u64) {
    let path = storage_path();
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

fn delete_todo(id: u64) {
    let path = storage_path();
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

fn archive_todo(id: u64) {
    let path = storage_path();
    let mut todos = storage::load(&path);
    match todos.iter_mut().find(|t| t.id == id) {
        Some(todo) => {
            todo.archived = true;
            storage::save(&path, &todos);
            println!("Archived todo #{}", id);
        }
        None => {
            eprintln!("Todo #{} not found", id);
            std::process::exit(1);
        }
    }
}

fn unarchive_todo(id: u64) {
    let path = storage_path();
    let mut todos = storage::load(&path);
    match todos.iter_mut().find(|t| t.id == id) {
        Some(todo) => {
            todo.archived = false;
            storage::save(&path, &todos);
            println!("Unarchived todo #{}", id);
        }
        None => {
            eprintln!("Todo #{} not found", id);
            std::process::exit(1);
        }
    }
}

fn search_todos(query: &str, archived: bool) {
    let path = storage_path();
    let todos = storage::load(&path);
    let q = query.to_lowercase();
    let mut results: Vec<_> = todos
        .iter()
        .filter(|t| t.archived == archived && t.description.to_lowercase().contains(&q))
        .collect();
    results.sort_by_key(|t| t.id);
    if results.is_empty() {
        println!("No matches for '{}'", query);
        return;
    }
    for t in results {
        let status = if t.done { "[x]" } else { "[ ]" };
        println!("{}  {}  {}", status, t.created_at.format("%m-%d %H:%M"), t.description);
    }
}

fn run_note_cmd(cmd: NoteCommand) {
    let path = notes_path();
    match cmd {
        NoteCommand::List => {
            let notes = storage::load_notes(&path);
            if notes.is_empty() {
                println!("No notes found.");
                return;
            }
            for n in &notes {
                println!("{}  {}  {}", n.id, n.created_at.format("%m-%d %H:%M"), n.title);
            }
        }
        NoteCommand::New { title } => {
            let content = match open_editor("") {
                Ok(c) => c,
                Err(e) => {
                    eprintln!("Editor error: {}", e);
                    std::process::exit(1);
                }
            };
            let mut notes = storage::load_notes(&path);
            let now = Local::now().naive_local();
            notes.push(Note {
                id: storage::next_id(),
                title,
                content,
                created_at: now,
                updated_at: now,
            });
            notes.sort_by_key(|n| n.id);
            storage::save_notes(&path, &notes);
            println!("Created note");
        }
        NoteCommand::Edit { id } => {
            let mut notes = storage::load_notes(&path);
            match notes.iter_mut().find(|n| n.id == id) {
                Some(note) => {
                    let new_content = match open_editor(&note.content) {
                        Ok(c) => c,
                        Err(e) => {
                            eprintln!("Editor error: {}", e);
                            std::process::exit(1);
                        }
                    };
                    note.content = new_content;
                    note.title = extract_title(&note.content);
                    note.updated_at = Local::now().naive_local();
                    storage::save_notes(&path, &notes);
                    println!("Updated note #{}", id);
                }
                None => {
                    eprintln!("Note #{} not found", id);
                    std::process::exit(1);
                }
            }
        }
        NoteCommand::View { id } => {
            let notes = storage::load_notes(&path);
            match notes.iter().find(|n| n.id == id) {
                Some(note) => print!("{}", note.content),
                None => {
                    eprintln!("Note #{} not found", id);
                    std::process::exit(1);
                }
            }
        }
        NoteCommand::Delete { id } => {
            let mut notes = storage::load_notes(&path);
            let len_before = notes.len();
            notes.retain(|n| n.id != id);
            if notes.len() == len_before {
                eprintln!("Note #{} not found", id);
                std::process::exit(1);
            }
            storage::save_notes(&path, &notes);
            println!("Deleted note #{}", id);
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

        match (&app.input_mode, &app.note_mode, &app.panel, &app.view) {
            (InputMode::Normal, NoteMode::Editing, Panel::Main, View::Note) => match key.code {
                KeyCode::Esc => {
                    app.save_current_note();
                    app.note_mode = NoteMode::Viewing;
                }
                KeyCode::Char(c) => app.note_cursor_insert(c),
                KeyCode::Backspace => app.note_cursor_backspace(),
                KeyCode::Delete => app.note_cursor_delete(),
                KeyCode::Enter => app.note_cursor_enter(),
                KeyCode::Up => app.note_cursor_up(),
                KeyCode::Down => app.note_cursor_down(),
                KeyCode::Left => app.note_cursor_left(),
                KeyCode::Right => app.note_cursor_right(),
                KeyCode::Home => app.note_cursor_home(),
                KeyCode::End => app.note_cursor_end(),
                KeyCode::Tab => {
                    app.save_current_note();
                    app.note_mode = NoteMode::Viewing;
                    app.panel = Panel::Sidebar;
                }
                _ => {}
            },

            (InputMode::Normal, NoteMode::Viewing, Panel::Main, View::Note) => match key.code {
                KeyCode::Char('q') => app.should_quit = true,
                KeyCode::Char('i') | KeyCode::Char('e') => app.start_edit_note(),
                KeyCode::Char('t') => { app.view = View::Todos; app.show_archived = false; app.selected_index = 0; }
                KeyCode::Char('a') => { app.view = View::Todos; app.show_archived = true; app.selected_index = 0; }
                KeyCode::Char('n') => app.new_note_inline(),
                KeyCode::Char('c') => {
                    app.view = View::Todos;
                    app.show_archived = false;
                    app.search_query.clear();
                    app.input_mode = InputMode::Adding;
                    app.input_buffer.clear();
                }
                KeyCode::Down | KeyCode::Char('j') => app.note_scroll_down(),
                KeyCode::Up | KeyCode::Char('k') => app.note_scroll_up(),
                KeyCode::Tab => app.panel = Panel::Sidebar,
                _ => {}
            },

            (InputMode::Normal, _, Panel::Main, View::Todos) => match key.code {
                KeyCode::Char('q') => app.should_quit = true,
                KeyCode::Char('t') => { app.show_archived = false; app.selected_index = 0; }
                KeyCode::Char('a') => { app.show_archived = true; app.selected_index = 0; }
                KeyCode::Char('n') => app.new_note_inline(),
                KeyCode::Char('c') => {
                    app.show_archived = false;
                    app.search_query.clear();
                    app.input_mode = InputMode::Adding;
                    app.input_buffer.clear();
                }
                KeyCode::Enter | KeyCode::Char('e') => {
                    if let Some(idx) = app.selected_todo_index() {
                        app.input_mode = InputMode::Editing;
                        app.input_buffer = app.todos[idx].description.clone();
                    }
                }
                KeyCode::Char('A') => app.archive_selected(),
                KeyCode::Char('d') => app.delete_selected(),
                KeyCode::Char(' ') => app.toggle_done(),
                KeyCode::Char('/') => {
                    app.input_mode = InputMode::Searching;
                    app.input_buffer.clear();
                    app.search_query.clear();
                }
                KeyCode::Up | KeyCode::Char('k') => app.move_up(),
                KeyCode::Down | KeyCode::Char('j') => app.move_down(),
                KeyCode::Tab => app.panel = Panel::Sidebar,
                KeyCode::Esc => app.should_quit = true,
                _ => {}
            },

            (InputMode::Normal, _, Panel::Sidebar, _) => match key.code {
                KeyCode::Char('q') => app.should_quit = true,
                KeyCode::Up | KeyCode::Char('k') => app.side_up(),
                KeyCode::Down | KeyCode::Char('j') => app.side_down(),
                KeyCode::Enter => app.select_sidebar(),
                KeyCode::Char('t') => { app.view = View::Todos; app.show_archived = false; app.selected_index = 0; app.panel = Panel::Main; }
                KeyCode::Char('a') => { app.view = View::Todos; app.show_archived = true; app.selected_index = 0; app.panel = Panel::Main; }
                KeyCode::Char('n') => app.new_note_inline(),
                KeyCode::Char('c') => {
                    app.view = View::Todos;
                    app.show_archived = false;
                    app.panel = Panel::Main;
                    app.input_mode = InputMode::Adding;
                    app.input_buffer.clear();
                    app.search_query.clear();
                }
                KeyCode::Char('d') => app.delete_note_by_side_index(),
                KeyCode::Tab | KeyCode::Esc => app.panel = Panel::Main,
                _ => {}
            },

            (InputMode::Adding, _, _, _) => match key.code {
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

            (InputMode::Editing, _, _, _) => match key.code {
                KeyCode::Enter | KeyCode::Esc => {
                    app.edit_todo();
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

            (InputMode::Searching, _, _, _) => match key.code {
                KeyCode::Enter | KeyCode::Esc => {
                    app.input_buffer.clear();
                    app.search_query.clear();
                    app.input_mode = InputMode::Normal;
                    app.selected_index = 0;
                }
                KeyCode::Backspace => {
                    app.input_buffer.pop();
                    app.search_query = app.input_buffer.clone();
                    app.selected_index = 0;
                }
                KeyCode::Char(c) => {
                    app.input_buffer.push(c);
                    app.search_query = app.input_buffer.clone();
                    app.selected_index = 0;
                }
                _ => {}
            },
        }
    }
    Ok(())
}
