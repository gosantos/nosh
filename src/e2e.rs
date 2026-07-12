//! End-to-end rendering tests. These build a real `App`, drive it with
//! synthetic key presses through `dispatch_key`, and render frames to an
//! in-memory `TestBackend`. Rendering to text lets us assert on exactly what
//! the user sees and catch layout regressions.

use crossterm::event::KeyCode;
use ratatui::backend::TestBackend;
use ratatui::buffer::Buffer;
use ratatui::Terminal;
use tempfile::TempDir;

use crate::app::App;
use crate::dispatch_key;
use crate::storage::{Note, Todo};

const W: u16 = 76;
const H: u16 = 22;

struct Harness {
    _dir: TempDir,
    app: App,
}

impl Harness {
    fn new(todos: Vec<Todo>, notes: Vec<Note>, folders: Vec<String>) -> Self {
        let dir = TempDir::new().unwrap();
        let storage_path = dir.path().join("todos.json");
        crate::storage::save(&storage_path, &todos);
        crate::storage::save_notes(&dir.path().join("notes.json"), &notes);
        crate::storage::save_folders(&dir.path().join("folders.json"), &folders);
        let app = App::new(storage_path);
        Harness { _dir: dir, app }
    }

    fn key(&mut self, code: KeyCode) -> &mut Self {
        dispatch_key(&mut self.app, code).unwrap();
        self
    }

    fn ch(&mut self, c: char) -> &mut Self {
        self.key(KeyCode::Char(c))
    }

    fn typ(&mut self, s: &str) -> &mut Self {
        for c in s.chars() {
            self.ch(c);
        }
        self
    }

    fn render(&mut self) -> String {
        render_to_string(&mut self.app, W, H)
    }
}

fn render_to_string(app: &mut App, w: u16, h: u16) -> String {
    let mut terminal = Terminal::new(TestBackend::new(w, h)).unwrap();
    terminal.draw(|f| crate::ui::draw(f, app)).unwrap();
    buffer_to_string(terminal.backend().buffer())
}

fn buffer_to_string(buf: &Buffer) -> String {
    let area = *buf.area();
    let mut out = String::new();
    for y in 0..area.height {
        for x in 0..area.width {
            out.push_str(buf[(x, y)].symbol());
        }
        // Trim trailing spaces so snapshots are easier to read.
        while out.ends_with(' ') {
            out.pop();
        }
        out.push('\n');
    }
    out
}

fn dt(s: &str) -> chrono::NaiveDateTime {
    chrono::NaiveDateTime::parse_from_str(s, "%Y-%m-%dT%H:%M:%S").unwrap()
}

fn todo(id: u64, desc: &str, done: bool) -> Todo {
    Todo {
        id,
        description: desc.to_string(),
        done,
        archived: false,
        created_at: dt("2026-07-10T09:00:00"),
        completed_at: done.then(|| dt("2026-07-10T10:00:00")),
    }
}

fn note(id: u64, title: &str, folder: Option<&str>) -> Note {
    Note {
        id,
        title: title.to_string(),
        content: format!("# {title}\n\nSome body text for {title}."),
        folder: folder.map(String::from),
        created_at: dt("2026-07-10T09:00:00"),
        updated_at: dt("2026-07-10T09:00:00"),
    }
}

fn sample() -> Harness {
    Harness::new(
        vec![
            todo(1, "Write the design doc", false),
            todo(2, "Review the pull request", true),
            todo(3, "Ship nosh v0.4", false),
        ],
        vec![
            note(1, "Standup notes", Some("Work")),
            note(2, "Groceries", Some("Personal")),
            note(3, "Scratch pad", None),
        ],
        vec!["Work".into(), "Personal".into()],
    )
}

/// Prints every major screen. Run with:
///   cargo test --bin nosh gallery -- --nocapture
#[test]
fn gallery() {
    let mut h = sample();
    let banner = |name: &str| println!("\n\n╔══ {name} {}", "═".repeat(60 - name.len()));

    banner("TODOS");
    println!("{}", h.render());

    banner("TODOS · creating");
    h.ch('c').typ("Buy oat milk");
    println!("{}", h.render());
    h.key(KeyCode::Esc);

    banner("NOTES · grouped by folder");
    h.ch('n');
    println!("{}", h.render());

    banner("NOTE · open (breadcrumb)");
    h.key(KeyCode::Enter);
    println!("{}", h.render());

    banner("NOTE · move picker");
    h.ch('m');
    println!("{}", h.render());
    h.key(KeyCode::Esc);

    banner("NOTE · editor");
    h.ch('e');
    println!("{}", h.render());
    h.key(KeyCode::Esc);
    h.key(KeyCode::Esc);

    banner("NOTES · search");
    h.ch('s').typ("gro");
    println!("{}", h.render());
    h.key(KeyCode::Esc);

    banner("TODOS · confirm delete");
    h.ch('t').ch('d');
    println!("{}", h.render());
    h.key(KeyCode::Esc);

    banner("EMPTY · no todos");
    let mut empty = Harness::new(vec![], vec![], vec![]);
    println!("{}", empty.render());

    banner("EMPTY · no notes");
    empty.ch('n');
    println!("{}", empty.render());

    banner("NARROW · todos at 60x18");
    let mut narrow = sample();
    println!("{}", render_to_string(&mut narrow.app, 60, 18));

    banner("NARROW · notes at 60x18");
    narrow.ch('n');
    println!("{}", render_to_string(&mut narrow.app, 60, 18));
}

// --- assertion-style e2e (guard the important invariants) ---

#[test]
fn todos_screen_shows_title_and_items() {
    let mut h = sample();
    let screen = h.render();
    assert!(screen.contains("nosh"), "brand in title bar");
    assert!(screen.contains("Write the design doc"));
    assert!(screen.contains("Todos"));
}

#[test]
fn notes_screen_groups_by_folder() {
    let mut h = sample();
    h.ch('n');
    let screen = h.render();
    // Folders alphabetical, unfiled last.
    let personal = screen.find("Personal").unwrap();
    let work = screen.find("Work").unwrap();
    let unfiled = screen.find("No folder").unwrap();
    assert!(personal < work, "Personal before Work");
    assert!(work < unfiled, "No folder last");
    assert!(screen.contains("Groceries") && screen.contains("Scratch pad"));
}

#[test]
fn open_note_shows_folder_breadcrumb() {
    let mut h = sample();
    h.ch('n').key(KeyCode::Enter); // first note in display order = Personal/Groceries
    let screen = h.render();
    assert!(
        screen.contains("Personal / Groceries"),
        "breadcrumb missing:\n{screen}"
    );
}

#[test]
fn move_picker_lists_folders_and_actions() {
    let mut h = sample();
    h.ch('n').ch('m');
    let screen = h.render();
    assert!(screen.contains("Move to folder"));
    assert!(screen.contains("Work") && screen.contains("Personal"));
    assert!(screen.contains("New folder"));
}

#[test]
fn footer_shows_back_hint_in_note_view() {
    let mut h = sample();
    h.ch('n').key(KeyCode::Enter);
    let screen = h.render();
    assert!(screen.contains("back"), "note footer should show Esc back");
}
