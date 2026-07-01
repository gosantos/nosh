use chrono::NaiveDateTime;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Todo {
    pub id: u64,
    pub description: String,
    pub done: bool,
    pub created_at: NaiveDateTime,
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
