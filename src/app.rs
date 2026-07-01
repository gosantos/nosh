use chrono::Local;
use std::path::PathBuf;

use crate::storage::{self, Note, Todo};

pub enum InputMode {
    Normal,
    Adding,
    Editing,
    Searching,
}

#[derive(PartialEq, Eq)]
pub enum Panel {
    Main,
    Sidebar,
}

#[derive(PartialEq, Eq)]
pub enum View {
    Todos,
    Note,
}

pub struct App {
    pub todos: Vec<Todo>,
    pub notes: Vec<Note>,
    pub input_mode: InputMode,
    pub input_buffer: String,
    pub selected_index: usize,
    pub should_quit: bool,
    pub show_archived: bool,
    pub search_query: String,
    pub panel: Panel,
    pub view: View,
    pub note_scroll: usize,
    pub side_index: usize,
    storage_path: PathBuf,
    notes_path: PathBuf,
}

fn side_items(view: &View, show_archived: bool, notes: &[Note]) -> Vec<SideItem> {
    let mut items = vec![
        SideItem::Active(view == &View::Todos && !show_archived),
        SideItem::Archive(view == &View::Todos && show_archived),
        SideItem::Sep,
    ];
    items.extend(notes.iter().map(|n| SideItem::Note(n.id, n.title.clone())));
    items
}

pub enum SideItem {
    Active(bool),
    Archive(bool),
    Sep,
    Note(u64, String),
}

impl App {
    pub fn new(storage_path: PathBuf) -> Self {
        let mut todos = storage::load(&storage_path);
        todos.sort_by_key(|t| t.id);
        let notes_path = storage_path.with_file_name(".tui-todo-notes.json");
        let notes = storage::load_notes(&notes_path);
        App {
            todos,
            notes,
            input_mode: InputMode::Normal,
            input_buffer: String::new(),
            selected_index: 0,
            should_quit: false,
            show_archived: false,
            search_query: String::new(),
            panel: Panel::Main,
            view: View::Todos,
            note_scroll: 0,
            side_index: 0,
            storage_path,
            notes_path,
        }
    }

    pub fn side_items(&self) -> Vec<SideItem> {
        side_items(&self.view, self.show_archived, &self.notes)
    }

    pub fn side_count(&self) -> usize {
        self.side_items().len()
    }

    pub fn current_note(&self) -> Option<&Note> {
        let items = self.side_items();
        match items.get(self.side_index) {
            Some(SideItem::Note(id, _)) => self.notes.iter().find(|n| n.id == *id),
            _ => None,
        }
    }

    pub fn current_note_mut(&mut self) -> Option<&mut Note> {
        let items = side_items(&self.view, self.show_archived, &self.notes);
        match items.get(self.side_index) {
            Some(SideItem::Note(id, _)) => self.notes.iter_mut().find(|n| n.id == *id),
            _ => None,
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

    #[allow(dead_code)]
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

    pub fn side_up(&mut self) {
        if self.side_index > 0 {
            self.side_index -= 1;
        }
    }

    pub fn side_down(&mut self) {
        if self.side_index + 1 < self.side_count() {
            self.side_index += 1;
        }
    }

    pub fn select_sidebar(&mut self) {
        let items = self.side_items();
        match items.get(self.side_index) {
            Some(SideItem::Active(_)) => {
                self.view = View::Todos;
                self.show_archived = false;
                self.panel = Panel::Main;
                self.selected_index = 0;
            }
            Some(SideItem::Archive(_)) => {
                self.view = View::Todos;
                self.show_archived = true;
                self.panel = Panel::Main;
                self.selected_index = 0;
            }
            Some(SideItem::Note(..)) => {
                self.view = View::Note;
                self.panel = Panel::Main;
                self.note_scroll = 0;
            }
            _ => {}
        }
    }

    pub fn add_note(&mut self, title: String) {
        let now = Local::now().naive_local();
        self.notes.push(Note {
            id: storage::next_id(),
            title,
            content: String::new(),
            created_at: now,
            updated_at: now,
        });
        self.notes.sort_by_key(|n| n.id);
        self.side_index = 2 + self.notes.len().saturating_sub(1);
        storage::save_notes(&self.notes_path, &self.notes);
    }

    pub fn update_note_content(&mut self, content: String) {
        if let Some(note) = self.current_note_mut() {
            note.content = content;
            note.updated_at = Local::now().naive_local();
            storage::save_notes(&self.notes_path, &self.notes);
        }
    }

    pub fn delete_note_by_side_index(&mut self) {
        let id = {
            let items = self.side_items();
            match items.get(self.side_index) {
                Some(SideItem::Note(id, _)) => *id,
                _ => return,
            }
        };
        self.notes.retain(|n| n.id != id);
        if self.side_index >= self.side_count() && self.side_index > 0 {
            self.side_index -= 1;
        }
        if self.notes.is_empty() && self.side_index == 0 {
            self.view = View::Todos;
        }
        storage::save_notes(&self.notes_path, &self.notes);
    }

    pub fn scroll_note_up(&mut self) {
        if self.note_scroll > 0 {
            self.note_scroll -= 1;
        }
    }

    pub fn scroll_note_down(&mut self) {
        self.note_scroll += 1;
    }
}
