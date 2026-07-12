use chrono::NaiveDateTime;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::Path;
use std::sync::atomic::{AtomicU16, Ordering};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Todo {
    pub id: u64,
    pub description: String,
    pub done: bool,
    #[serde(default)]
    pub archived: bool,
    pub created_at: NaiveDateTime,
    #[serde(default)]
    pub completed_at: Option<NaiveDateTime>,
}

impl Todo {
    /// Marks the todo done/undone, keeping `completed_at` in sync.
    pub fn set_done(&mut self, done: bool) {
        self.done = done;
        self.completed_at = done.then(|| chrono::Local::now().naive_local());
    }
}

static SEQUENCE: AtomicU16 = AtomicU16::new(0);

pub fn next_id() -> u64 {
    let ts = chrono::Utc::now().timestamp_millis() as u64;
    let seq = SEQUENCE.fetch_add(1, Ordering::Relaxed) as u64;
    (ts << 10) | (seq % 1024)
}

pub fn load(path: &Path) -> Vec<Todo> {
    read_json(path)
}

pub fn save(path: &Path, todos: &[Todo]) {
    write_json(path, todos);
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Note {
    pub id: u64,
    pub title: String,
    pub content: String,
    /// One level of grouping. `None` means the note is unfiled. Folders exist
    /// only while at least one note references them.
    #[serde(default)]
    pub folder: Option<String>,
    pub created_at: NaiveDateTime,
    pub updated_at: NaiveDateTime,
}

pub fn load_notes(path: &Path) -> Vec<Note> {
    let mut notes: Vec<Note> = read_json(path);
    notes.sort_by_key(|n| n.id);
    notes
}

pub fn save_notes(path: &Path, notes: &[Note]) {
    write_json(path, notes);
}

/// The persisted set of folder names. Folders exist independently of notes so
/// that creating one is durable and it survives moving its last note out.
pub fn load_folders(path: &Path) -> Vec<String> {
    read_json(path)
}

pub fn save_folders(path: &Path, folders: &[String]) {
    write_json(path, folders);
}

fn read_json<T: serde::de::DeserializeOwned + Default>(path: &Path) -> T {
    fs::read_to_string(path)
        .ok()
        .and_then(|content| serde_json::from_str(&content).ok())
        .unwrap_or_default()
}

fn write_json<T: Serialize + ?Sized>(path: &Path, value: &T) {
    if let Some(parent) = path.parent() {
        let _ = fs::create_dir_all(parent);
    }
    let content = serde_json::to_string_pretty(value).unwrap();
    let _ = fs::write(path, content);
}

/// Derives a note title from its content: the first `# ` heading, or the
/// first non-empty line (truncated), or "Untitled".
pub fn extract_title(content: &str) -> String {
    for line in content.lines() {
        let trimmed = line.trim();
        if let Some(rest) = trimmed.strip_prefix("# ") {
            return rest.trim().to_string();
        }
        if !trimmed.is_empty() {
            return trimmed.chars().take(60).collect();
        }
    }
    "Untitled".to_string()
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn make_todo(id: u64) -> Todo {
        Todo {
            id,
            description: format!("todo {id}"),
            done: false,
            archived: false,
            created_at: chrono::Local::now().naive_local(),
            completed_at: None,
        }
    }

    #[test]
    fn load_missing_file_returns_empty() {
        let dir = TempDir::new().unwrap();
        assert!(load(&dir.path().join("nope.json")).is_empty());
        assert!(load_notes(&dir.path().join("nope.json")).is_empty());
    }

    #[test]
    fn load_corrupt_file_returns_empty() {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("todos.json");
        fs::write(&path, "not json {").unwrap();
        assert!(load(&path).is_empty());
    }

    #[test]
    fn save_load_round_trip() {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("todos.json");
        let todos = vec![make_todo(1), make_todo(2)];
        save(&path, &todos);
        let loaded = load(&path);
        assert_eq!(loaded.len(), 2);
        assert_eq!(loaded[0].id, 1);
        assert_eq!(loaded[1].description, "todo 2");
    }

    #[test]
    fn save_creates_parent_dirs() {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("deep").join("nested").join("todos.json");
        save(&path, &[make_todo(1)]);
        assert_eq!(load(&path).len(), 1);
    }

    #[test]
    fn todo_missing_optional_fields_defaults() {
        let json =
            r#"[{"id":1,"description":"old","done":true,"created_at":"2025-01-01T10:00:00"}]"#;
        let todos: Vec<Todo> = serde_json::from_str(json).unwrap();
        assert!(!todos[0].archived);
        assert!(todos[0].completed_at.is_none());
    }

    #[test]
    fn note_without_folder_field_defaults_to_none() {
        let json = r#"[{"id":1,"title":"old","content":"x","created_at":"2025-01-01T10:00:00","updated_at":"2025-01-01T10:00:00"}]"#;
        let notes: Vec<Note> = serde_json::from_str(json).unwrap();
        assert!(notes[0].folder.is_none());
    }

    #[test]
    fn load_notes_sorts_by_id() {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("notes.json");
        let now = chrono::Local::now().naive_local();
        let note = |id: u64| Note {
            id,
            title: String::new(),
            content: String::new(),
            folder: None,
            created_at: now,
            updated_at: now,
        };
        save_notes(&path, &[note(3), note(1), note(2)]);
        let loaded = load_notes(&path);
        assert_eq!(loaded.iter().map(|n| n.id).collect::<Vec<_>>(), [1, 2, 3]);
    }

    #[test]
    fn next_id_is_unique() {
        let mut ids: Vec<u64> = (0..100).map(|_| next_id()).collect();
        ids.sort_unstable();
        ids.dedup();
        assert_eq!(ids.len(), 100);
    }

    #[test]
    fn extract_title_prefers_heading() {
        assert_eq!(extract_title("intro\n# My Title\nbody"), "intro");
        assert_eq!(extract_title("# My Title\nbody"), "My Title");
        assert_eq!(extract_title("\n\n#  Spaced  \nbody"), "Spaced");
    }

    #[test]
    fn extract_title_falls_back_to_first_line() {
        assert_eq!(extract_title("just text\nmore"), "just text");
        let long = "x".repeat(100);
        assert_eq!(extract_title(&long).chars().count(), 60);
    }

    #[test]
    fn set_done_keeps_completed_at_in_sync() {
        let mut todo = make_todo(1);
        todo.set_done(true);
        assert!(todo.done);
        assert!(todo.completed_at.is_some());

        todo.set_done(false);
        assert!(!todo.done);
        assert!(todo.completed_at.is_none());
    }

    #[test]
    fn extract_title_empty_content() {
        assert_eq!(extract_title(""), "Untitled");
        assert_eq!(extract_title("\n  \n"), "Untitled");
    }
}
