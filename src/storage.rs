use chrono::{NaiveDate, NaiveDateTime};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;
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
    #[serde(default)]
    pub due_date: Option<NaiveDate>,
}

static SEQUENCE: AtomicU16 = AtomicU16::new(0);

pub fn next_id() -> u64 {
    let ts = chrono::Utc::now().timestamp_millis() as u64;
    let seq = SEQUENCE.fetch_add(1, Ordering::Relaxed) as u64;
    (ts << 10) | (seq % 1024)
}

pub fn load(path: &PathBuf) -> Vec<Todo> {
    if !path.exists() {
        return Vec::new();
    }
    let content = match fs::read_to_string(path) {
        Ok(c) => c,
        Err(_) => return Vec::new(),
    };
    serde_json::from_str(&content).unwrap_or_default()
}

pub fn save(path: &PathBuf, todos: &[Todo]) {
    if let Some(parent) = path.parent() {
        let _ = fs::create_dir_all(parent);
    }
    let content = serde_json::to_string_pretty(todos).unwrap();
    let _ = fs::write(path, content);
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Note {
    pub id: u64,
    pub title: String,
    pub content: String,
    pub created_at: NaiveDateTime,
    pub updated_at: NaiveDateTime,
}

pub fn load_notes(path: &PathBuf) -> Vec<Note> {
    if !path.exists() {
        return Vec::new();
    }
    let content = match fs::read_to_string(path) {
        Ok(c) => c,
        Err(_) => return Vec::new(),
    };
    let mut notes: Vec<Note> = serde_json::from_str(&content).unwrap_or_default();
    notes.sort_by_key(|n| n.id);
    notes
}

pub fn save_notes(path: &PathBuf, notes: &[Note]) {
    if let Some(parent) = path.parent() {
        let _ = fs::create_dir_all(parent);
    }
    let content = serde_json::to_string_pretty(notes).unwrap();
    let _ = fs::write(path, content);
}
