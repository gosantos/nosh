mod app;
mod markdown;
mod storage;
mod ui;

#[cfg(test)]
mod e2e;

use std::io;
use std::path::PathBuf;
use std::sync::mpsc;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use notify::{Config, EventKind, RecommendedWatcher, RecursiveMode, Watcher};

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
#[command(name = "nosh", version = env!("CARGO_PKG_VERSION"))]
struct Cli {
    #[command(subcommand)]
    command: Option<Command>,
}

#[derive(clap::Subcommand)]
enum Command {
    #[command(subcommand)]
    Todos(TodosCommand),
    #[command(subcommand)]
    Notes(NotesCommand),
}

#[derive(clap::Subcommand)]
enum TodosCommand {
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
    Create {
        description: String,
    },
    Edit {
        id: u64,
        description: String,
    },
    Do {
        id: u64,
    },
    Undo {
        id: u64,
    },
    Delete {
        id: u64,
    },
    Archive {
        id: u64,
    },
    Unarchive {
        id: u64,
    },
}

#[derive(clap::Subcommand)]
enum NotesCommand {
    List,
    Create { title: String },
    Edit { id: u64 },
    View { id: u64 },
    Delete { id: u64 },
}

fn data_dir() -> PathBuf {
    if let Ok(dir) = std::env::var("NOSH_DATA_DIR") {
        let dir = PathBuf::from(dir);
        let _ = std::fs::create_dir_all(&dir);
        return dir;
    }
    let cwd = std::env::current_dir().unwrap_or_default();
    let dir = cwd.join(".nosh");
    let _ = std::fs::create_dir_all(&dir);
    dir
}

fn storage_path() -> PathBuf {
    data_dir().join("todos.json")
}

fn notes_path() -> PathBuf {
    data_dir().join("notes.json")
}

fn open_editor(content: &str) -> io::Result<String> {
    let editor = std::env::var("EDITOR").unwrap_or_else(|_| "vim".to_string());
    let tmp = std::env::temp_dir().join(format!("nosh-note-{}.md", std::process::id()));
    std::fs::write(&tmp, content)?;
    let status = std::process::Command::new(&editor).arg(&tmp).status()?;
    let new_content = std::fs::read_to_string(&tmp)?;
    let _ = std::fs::remove_file(&tmp);
    if !status.success() {
        return Err(io::Error::other("editor exited with error"));
    }
    Ok(new_content)
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
        Command::Todos(todos_cmd) => run_todos_cmd(todos_cmd),
        Command::Notes(notes_cmd) => run_notes_cmd(notes_cmd),
    }
}

fn run_todos_cmd(cmd: TodosCommand) {
    match cmd {
        TodosCommand::List {
            done,
            pending,
            ids,
            archived,
        } => list_todos(done, pending, ids, archived),
        TodosCommand::Create { description } => add_todo(&description),
        TodosCommand::Edit { id, description } => {
            update_todo(id, format!("Updated todo #{id}"), |t| {
                t.description = description
            })
        }
        TodosCommand::Do { id } => update_todo(id, format!("Marked todo #{id} as done"), |t| {
            t.set_done(true)
        }),
        TodosCommand::Undo { id } => {
            update_todo(id, format!("Marked todo #{id} as not done"), |t| {
                t.set_done(false)
            })
        }
        TodosCommand::Delete { id } => delete_todo(id),
        TodosCommand::Archive { id } => update_todo(id, format!("Archived todo #{id}"), |t| {
            t.archived = true;
            t.set_done(true);
        }),
        TodosCommand::Unarchive { id } => {
            update_todo(id, format!("Unarchived todo #{id}"), |t| t.archived = false)
        }
    }
}

/// True when the todo passes the `list` command's flags.
fn matches_list_filter(t: &Todo, done: bool, pending: bool, archived: bool) -> bool {
    let show_done = done || !pending;
    let show_pending = pending || !done;
    t.archived == archived && ((show_done && t.done) || (show_pending && !t.done))
}

fn list_todos(done: bool, pending: bool, ids: bool, archived: bool) {
    let mut todos = storage::load(&storage_path());
    todos.sort_by_key(|t| t.id);
    let filtered: Vec<_> = todos
        .iter()
        .filter(|t| matches_list_filter(t, done, pending, archived))
        .collect();
    if filtered.is_empty() {
        println!("No todos found.");
        return;
    }
    for t in filtered {
        let status = if t.done { "[x]" } else { "[ ]" };
        let id_col = if ids {
            format!("  {:>16}", t.id)
        } else {
            String::new()
        };
        println!(
            "{}{}  {}  {}",
            status,
            id_col,
            t.created_at.format("%m-%d %H:%M"),
            t.description,
        );
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
        completed_at: None,
    });
    storage::save(&path, &todos);
    println!("Added todo");
}

/// Applies `update` to the todo with `id` and saves; exits with an error
/// message when the id is unknown.
fn update_todo(id: u64, success: String, update: impl FnOnce(&mut Todo)) {
    let path = storage_path();
    let mut todos = storage::load(&path);
    match todos.iter_mut().find(|t| t.id == id) {
        Some(todo) => {
            update(todo);
            storage::save(&path, &todos);
            println!("{success}");
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

fn run_notes_cmd(cmd: NotesCommand) {
    let path = notes_path();
    match cmd {
        NotesCommand::List => {
            let notes = storage::load_notes(&path);
            if notes.is_empty() {
                println!("No notes found.");
                return;
            }
            for n in &notes {
                println!(
                    "{}  {}  {}",
                    n.id,
                    n.created_at.format("%m-%d %H:%M"),
                    n.title
                );
            }
        }
        NotesCommand::Create { title } => {
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
                folder: None,
                created_at: now,
                updated_at: now,
            });
            notes.sort_by_key(|n| n.id);
            storage::save_notes(&path, &notes);
            println!("Created note");
        }
        NotesCommand::Edit { id } => {
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
                    note.title = storage::extract_title(&note.content);
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
        NotesCommand::View { id } => {
            let notes = storage::load_notes(&path);
            match notes.iter().find(|n| n.id == id) {
                Some(note) => print!("{}", note.content),
                None => {
                    eprintln!("Note #{} not found", id);
                    std::process::exit(1);
                }
            }
        }
        NotesCommand::Delete { id } => {
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

fn handle_move_folder_event(app: &mut App, code: KeyCode) -> io::Result<()> {
    if app.new_folder_buffer.is_some() {
        match code {
            KeyCode::Esc => app.cancel_move(),
            KeyCode::Enter => app.move_new_confirm(),
            KeyCode::Backspace => app.move_new_backspace(),
            KeyCode::Char(c) => app.move_new_char(c),
            _ => {}
        }
    } else {
        match code {
            KeyCode::Esc => app.cancel_move(),
            KeyCode::Enter => app.move_picker_select(),
            KeyCode::Up | KeyCode::Char('k') => app.move_picker_up(),
            KeyCode::Down | KeyCode::Char('j') => app.move_picker_down(),
            KeyCode::Char('d') => app.delete_selected_folder(),
            _ => {}
        }
    }
    Ok(())
}

fn handle_confirm_delete_event(app: &mut App, code: KeyCode) -> io::Result<()> {
    match code {
        KeyCode::Esc => app.cancel_confirm(),
        KeyCode::Enter => app.confirm_delete(),
        KeyCode::Left | KeyCode::Char('h') => app.confirm_move_left(),
        KeyCode::Right | KeyCode::Char('l') => app.confirm_move_right(),
        _ => {}
    }
    Ok(())
}

fn run_tui() -> io::Result<()> {
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    stdout.execute(EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;
    terminal.clear()?;

    let storage_path = storage_path();
    let app = Arc::new(Mutex::new(App::new(storage_path.clone())));
    let tick_rate = Duration::from_millis(250);
    let mut last_tick = Instant::now();

    let (file_tx, file_rx) = mpsc::channel::<()>();
    let mut watcher = RecommendedWatcher::new(
        move |res: notify::Result<notify::Event>| {
            if let Ok(event) = res {
                if matches!(event.kind, EventKind::Modify(_) | EventKind::Create(_)) {
                    let _ = file_tx.send(());
                }
            }
        },
        Config::default(),
    )
    .map_err(io::Error::other)?;

    let _ = watcher.watch(&storage_path, RecursiveMode::NonRecursive);
    let notes_path = storage_path.with_file_name("notes.json");
    let _ = watcher.watch(&notes_path, RecursiveMode::NonRecursive);

    {
        let app = Arc::clone(&app);
        std::thread::spawn(move || loop {
            std::thread::sleep(Duration::from_secs(60));
            let mut app = app.lock().unwrap();
            app.archive_old();
        });
    }

    loop {
        if file_rx.try_recv().is_ok() {
            while file_rx.try_recv().is_ok() {}
            let mut app = app.lock().unwrap();
            app.reload();
        }

        let mut app = app.lock().unwrap();
        terminal.draw(|f| ui::draw(f, &mut app))?;

        let timeout = tick_rate
            .saturating_sub(last_tick.elapsed())
            .max(Duration::from_millis(1));
        if event::poll(timeout)? {
            handle_event(&mut app)?;
        }
        if last_tick.elapsed() >= tick_rate {
            last_tick = Instant::now();
        }

        if app.should_quit {
            break;
        }
    }

    disable_raw_mode()?;
    terminal.backend_mut().execute(LeaveAlternateScreen)?;
    Ok(())
}

fn handle_event(app: &mut App) -> io::Result<()> {
    if let Event::Key(key) = event::read()? {
        if key.kind == KeyEventKind::Press {
            dispatch_key(app, key.code)?;
        }
    }
    Ok(())
}

/// Applies a single key press to the app. Split out from terminal reading so
/// tests can drive the app with synthetic key codes.
fn dispatch_key(app: &mut App, code: KeyCode) -> io::Result<()> {
    {
        if matches!(app.input_mode, InputMode::ConfirmDelete) {
            return handle_confirm_delete_event(app, code);
        }

        if matches!(app.input_mode, InputMode::MoveToFolder) {
            return handle_move_folder_event(app, code);
        }

        if matches!(app.input_mode, InputMode::Search) {
            return handle_search_event(app, code);
        }

        if matches!(app.input_mode, InputMode::Creating) {
            return handle_creating_event(app, code);
        }

        if matches!(app.input_mode, InputMode::Editing) {
            return handle_editing_event(app, code);
        }

        if app.undo_state.is_active() && matches!(app.input_mode, InputMode::Normal) {
            match code {
                KeyCode::Char('u') => {
                    app.undo_delete();
                    return Ok(());
                }
                _ => app.clear_undo(),
            }
        }

        match (&app.input_mode, &app.note_mode, &app.panel, &app.view) {
            (InputMode::Normal, NoteMode::Editing, Panel::Main, View::Note) => match code {
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

            (InputMode::Normal, NoteMode::Viewing, Panel::Main, View::Note) => match code {
                KeyCode::Char('q') => app.should_quit = true,
                KeyCode::Char('e') | KeyCode::Char('i') => app.start_edit_note(),
                KeyCode::Esc | KeyCode::Char('n') => app.back_to_notes_list(),
                KeyCode::Char('t') => {
                    app.view = View::Todos;
                    app.show_archived = false;
                    app.selected_index = 0;
                }
                KeyCode::Char('a') => {
                    app.view = View::Todos;
                    app.show_archived = true;
                    app.selected_index = 0;
                }
                KeyCode::Char('s') => app.start_search(),
                KeyCode::Char('c') => {
                    app.start_creating_note();
                }
                KeyCode::Char('d') => app.start_deletion(),
                KeyCode::Char('m') => app.start_move_current_note(),
                KeyCode::Down | KeyCode::Char('j') => app.note_scroll_down(),
                KeyCode::Up | KeyCode::Char('k') => app.note_scroll_up(),
                KeyCode::PageDown => app.note_scroll_page_down(),
                KeyCode::PageUp => app.note_scroll_page_up(),
                KeyCode::Home => app.note_scroll_top(),
                KeyCode::End => app.note_scroll_bottom(),
                KeyCode::Tab => app.panel = Panel::Sidebar,
                _ => {}
            },

            (InputMode::Normal, _, Panel::Main, View::Todos) => match code {
                KeyCode::Char('q') => app.should_quit = true,
                KeyCode::Char('t') => {
                    app.show_archived = false;
                    app.selected_index = 0;
                }
                KeyCode::Char('a') => {
                    app.show_archived = true;
                    app.selected_index = 0;
                }
                KeyCode::Char('n') => {
                    app.view = View::Notes;
                    app.panel = Panel::Main;
                    app.selected_index = 0;
                }
                KeyCode::Char('s') => app.start_search(),
                KeyCode::Char('c') => {
                    app.start_creating();
                }
                KeyCode::Char('e') => {
                    app.start_editing();
                }
                KeyCode::Char('/') => app.start_search(),
                KeyCode::Char('A') => app.archive_selected(),
                KeyCode::Char('d') => app.start_deletion(),
                KeyCode::Char(' ') => app.toggle_done(),
                KeyCode::Up | KeyCode::Char('k') => app.move_up(),
                KeyCode::Down | KeyCode::Char('j') => app.move_down(),
                KeyCode::Tab => app.panel = Panel::Sidebar,
                KeyCode::Esc => {
                    app.search_filter = None;
                    app.search_buffer.clear();
                }
                _ => {}
            },

            (InputMode::Normal, _, Panel::Main, View::Notes) => match code {
                KeyCode::Char('q') => app.should_quit = true,
                KeyCode::Char('t') => {
                    app.view = View::Todos;
                    app.show_archived = false;
                    app.selected_index = 0;
                }
                KeyCode::Char('a') => {
                    app.view = View::Todos;
                    app.show_archived = true;
                    app.selected_index = 0;
                }
                KeyCode::Char('n') => {
                    app.selected_index = 0;
                }
                KeyCode::Char('s') => app.start_search(),
                KeyCode::Char('/') => app.start_search(),
                KeyCode::Char('c') => {
                    app.start_creating_note();
                }
                KeyCode::Char('d') => app.start_deletion(),
                KeyCode::Char('m') => app.start_move_note(),
                KeyCode::Enter => {
                    if let Some(idx) = app.selected_note_index() {
                        app.current_note_index = Some(idx);
                        app.view = View::Note;
                        app.note_mode = NoteMode::Viewing;
                        app.note_scroll = 0;
                        app.note_view_max_scroll = 0;
                    }
                }
                KeyCode::Up | KeyCode::Char('k') => app.note_list_up(),
                KeyCode::Down | KeyCode::Char('j') => app.note_list_down(),
                KeyCode::Tab => app.panel = Panel::Sidebar,
                KeyCode::Esc => {
                    app.search_filter = None;
                    app.search_buffer.clear();
                }
                _ => {}
            },

            (InputMode::Normal, _, Panel::Sidebar, _) => match code {
                KeyCode::Char('q') => app.should_quit = true,
                KeyCode::Up | KeyCode::Char('k') => app.side_up(),
                KeyCode::Down | KeyCode::Char('j') => app.side_down(),
                KeyCode::Enter => app.select_sidebar(),
                KeyCode::Char('t') => {
                    app.view = View::Todos;
                    app.show_archived = false;
                    app.selected_index = 0;
                    app.panel = Panel::Main;
                }
                KeyCode::Char('a') => {
                    app.view = View::Todos;
                    app.show_archived = true;
                    app.selected_index = 0;
                    app.panel = Panel::Main;
                }
                KeyCode::Char('n') => {
                    app.view = View::Notes;
                    app.panel = Panel::Main;
                    app.selected_index = 0;
                }
                KeyCode::Char('s') => app.start_search(),
                KeyCode::Char('c') => {
                    if app.side_index == 2 {
                        app.start_creating_note();
                    } else {
                        app.start_creating();
                    }
                }
                KeyCode::Char('d') => app.start_deletion(),
                KeyCode::Tab | KeyCode::Esc => app.panel = Panel::Main,
                _ => {}
            },

            _ => {}
        }
    }
    Ok(())
}

fn handle_search_event(app: &mut App, code: KeyCode) -> io::Result<()> {
    match code {
        KeyCode::Esc => app.cancel_search(),
        KeyCode::Enter => app.apply_search(),
        KeyCode::Backspace => app.search_buffer_pop(),
        KeyCode::Char(c) => app.search_buffer_push(c),
        _ => {}
    }
    Ok(())
}

fn handle_editing_event(app: &mut App, code: KeyCode) -> io::Result<()> {
    match code {
        KeyCode::Esc => app.cancel_editing(),
        KeyCode::Enter => app.confirm_editing(),
        KeyCode::Backspace => app.edit_backspace(),
        KeyCode::Char(c) => app.edit_type_char(c),
        _ => {}
    }
    Ok(())
}

fn handle_creating_event(app: &mut App, code: KeyCode) -> io::Result<()> {
    match code {
        KeyCode::Esc => app.cancel_creating(),
        KeyCode::Enter => app.confirm_creating(),
        KeyCode::Backspace => app.create_backspace(),
        KeyCode::Char(c) => app.create_type_char(c),
        _ => {}
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn todo(done: bool, archived: bool) -> Todo {
        Todo {
            id: 1,
            description: "t".to_string(),
            done,
            archived,
            created_at: Local::now().naive_local(),
            completed_at: None,
        }
    }

    #[test]
    fn list_filter_defaults_to_all_unarchived() {
        assert!(matches_list_filter(
            &todo(false, false),
            false,
            false,
            false
        ));
        assert!(matches_list_filter(&todo(true, false), false, false, false));
        assert!(!matches_list_filter(
            &todo(false, true),
            false,
            false,
            false
        ));
    }

    #[test]
    fn list_filter_done_and_pending_flags() {
        assert!(matches_list_filter(&todo(true, false), true, false, false));
        assert!(!matches_list_filter(
            &todo(false, false),
            true,
            false,
            false
        ));

        assert!(matches_list_filter(&todo(false, false), false, true, false));
        assert!(!matches_list_filter(&todo(true, false), false, true, false));

        // Both flags together show everything again.
        assert!(matches_list_filter(&todo(true, false), true, true, false));
        assert!(matches_list_filter(&todo(false, false), true, true, false));
    }

    #[test]
    fn list_filter_archived_flag() {
        assert!(matches_list_filter(&todo(true, true), false, false, true));
        assert!(!matches_list_filter(
            &todo(false, false),
            false,
            false,
            true
        ));
    }
}
