use chrono::Local;
use std::path::PathBuf;

use crate::storage::{self, Todo};

pub enum InputMode {
    Normal,
    Adding,
    Editing,
    Searching,
}

pub struct App {
    pub todos: Vec<Todo>,
    pub input_mode: InputMode,
    pub input_buffer: String,
    pub selected_index: usize,
    pub should_quit: bool,
    pub show_archived: bool,
    pub search_query: String,
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
            show_archived: false,
            search_query: String::new(),
            storage_path,
        }
    }

    pub fn visible_count(&self) -> usize {
        let q = self.search_query.to_lowercase();
        self.todos
            .iter()
            .filter(|t| {
                self.show_archived == t.archived
                    && (q.is_empty() || t.description.to_lowercase().contains(&q))
            })
            .count()
    }

    pub fn selected_todo_index(&self) -> Option<usize> {
        let q = self.search_query.to_lowercase();
        self.todos
            .iter()
            .enumerate()
            .filter(|(_, t)| {
                self.show_archived == t.archived
                    && (q.is_empty() || t.description.to_lowercase().contains(&q))
            })
            .map(|(i, _)| i)
            .nth(self.selected_index)
    }

    fn clamp_selection(&mut self) {
        let count = self.visible_count();
        if count == 0 {
            self.selected_index = 0;
        } else if self.selected_index >= count {
            self.selected_index = count - 1;
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
            archived: false,
            created_at: Local::now().naive_local(),
        });
        self.input_buffer.clear();
        self.selected_index = self.visible_count().saturating_sub(1);
        storage::save(&self.storage_path, &self.todos);
    }

    pub fn edit_todo(&mut self) {
        let desc = self.input_buffer.trim().to_string();
        if desc.is_empty() {
            return;
        }
        if let Some(idx) = self.selected_todo_index() {
            self.todos[idx].description = desc;
            storage::save(&self.storage_path, &self.todos);
        }
        self.input_buffer.clear();
    }

    pub fn toggle_done(&mut self) {
        if let Some(idx) = self.selected_todo_index() {
            self.todos[idx].done = !self.todos[idx].done;
            storage::save(&self.storage_path, &self.todos);
        }
    }

    pub fn archive_selected(&mut self) {
        if let Some(idx) = self.selected_todo_index() {
            self.todos[idx].archived = !self.todos[idx].archived;
            storage::save(&self.storage_path, &self.todos);
            self.clamp_selection();
        }
    }

    pub fn delete_selected(&mut self) {
        if let Some(idx) = self.selected_todo_index() {
            self.todos.remove(idx);
            storage::save(&self.storage_path, &self.todos);
            self.clamp_selection();
        }
    }

    pub fn toggle_archived_view(&mut self) {
        self.show_archived = !self.show_archived;
        self.selected_index = 0;
    }

    pub fn move_up(&mut self) {
        if self.selected_index > 0 {
            self.selected_index -= 1;
        }
    }

    pub fn move_down(&mut self) {
        if self.selected_index + 1 < self.visible_count() {
            self.selected_index += 1;
        }
    }
}
