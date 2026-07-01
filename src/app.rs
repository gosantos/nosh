use chrono::Local;
use std::path::PathBuf;

use crate::storage::{self, Todo};

pub enum InputMode {
    Normal,
    Adding,
    Editing,
}

pub struct App {
    pub todos: Vec<Todo>,
    pub input_mode: InputMode,
    pub input_buffer: String,
    pub selected_index: usize,
    pub should_quit: bool,
    storage_path: PathBuf,
}

impl App {
    pub fn new(storage_path: PathBuf) -> Self {
        let mut todos = storage::load(&storage_path);
        todos.sort_by_key(|t| t.id);
        App {
            todos,
            input_mode: InputMode::Normal,
            input_buffer: String::new(),
            selected_index: 0,
            should_quit: false,
            storage_path,
        }
    }

    pub fn add_todo(&mut self) {
        let desc = self.input_buffer.trim().to_string();
        if desc.is_empty() {
            return;
        }
        self.todos.push(Todo {
            id: storage::next_id(),
            description: desc,
            done: false,
            created_at: Local::now().naive_local(),
        });
        self.input_buffer.clear();
        self.selected_index = self.todos.len().saturating_sub(1);
        storage::save(&self.storage_path, &self.todos);
    }

    pub fn edit_todo(&mut self) {
        let desc = self.input_buffer.trim().to_string();
        if desc.is_empty() {
            return;
        }
        if let Some(todo) = self.todos.get_mut(self.selected_index) {
            todo.description = desc;
            storage::save(&self.storage_path, &self.todos);
        }
        self.input_buffer.clear();
    }

    pub fn toggle_done(&mut self) {
        if let Some(todo) = self.todos.get_mut(self.selected_index) {
            todo.done = !todo.done;
            storage::save(&self.storage_path, &self.todos);
        }
    }

    pub fn delete_selected(&mut self) {
        if self.selected_index < self.todos.len() {
            self.todos.remove(self.selected_index);
            if self.selected_index > 0 && self.selected_index >= self.todos.len() {
                self.selected_index = self.todos.len().saturating_sub(1);
            }
            storage::save(&self.storage_path, &self.todos);
        }
    }

    pub fn move_up(&mut self) {
        if self.selected_index > 0 {
            self.selected_index -= 1;
        }
    }

    pub fn move_down(&mut self) {
        if self.selected_index + 1 < self.todos.len() {
            self.selected_index += 1;
        }
    }
}
