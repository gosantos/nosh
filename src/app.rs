use chrono::Local;
use std::path::PathBuf;

use crate::storage::{self, Note, Todo};

pub enum InputMode {
    Normal,
    Adding,
    Editing,
    Searching,
    Palette,
}

#[derive(Clone, Copy)]
pub enum PaletteKind {
    Omni,
    Notes,
}

pub enum PaletteAction {
    OpenNote(u64),
    CreateNote(String),
    CreateTodo(String),
}

pub struct PaletteItem {
    pub title: String,
    pub subtitle: String,
    pub icon: char,
    pub action: PaletteAction,
    pub score: i64,
    pub matches: Vec<usize>,
}

pub enum NoteMode {
    Viewing,
    Editing,
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
    pub note_mode: NoteMode,
    pub list_scroll: usize,
    pub note_scroll: usize,
    pub note_cursor_line: usize,
    pub note_cursor_col: usize,
    pub note_lines: Vec<String>,
    pub side_index: usize,
    pub palette_kind: PaletteKind,
    pub palette_query: String,
    pub palette_items: Vec<PaletteItem>,
    pub palette_selected: usize,
    storage_path: PathBuf,
    notes_path: PathBuf,
}

fn side_items(view: &View, show_archived: bool, notes: &[Note]) -> Vec<SideItem> {
    let mut items = vec![
        SideItem::Active(view == &View::Todos && !show_archived),
        SideItem::Archive(view == &View::Todos && show_archived),
        SideItem::NotesHeader(notes.len()),
    ];
    items.extend(notes.iter().map(|n| SideItem::Note(n.id, n.title.clone())));
    items
}

pub enum SideItem {
    Active(bool),
    Archive(bool),
    NotesHeader(usize),
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
            note_mode: NoteMode::Viewing,
            list_scroll: 0,
            note_scroll: 0,
            note_cursor_line: 0,
            note_cursor_col: 0,
            note_lines: Vec::new(),
            side_index: 0,
            palette_kind: PaletteKind::Omni,
            palette_query: String::new(),
            palette_items: Vec::new(),
            palette_selected: 0,
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

    pub fn save_current_note(&mut self) {
        let content = self.note_lines.join("\n");
        let title = extract_title(&content);
        let note_id = {
            let items = side_items(&self.view, self.show_archived, &self.notes);
            match items.get(self.side_index) {
                Some(SideItem::Note(id, _)) => *id,
                _ => return,
            }
        };
        if let Some(note) = self.notes.iter_mut().find(|n| n.id == note_id) {
            note.content = content;
            note.title = title;
            note.updated_at = Local::now().naive_local();
            storage::save_notes(&self.notes_path, &self.notes);
        }
    }

    pub fn start_edit_note(&mut self) {
        if let Some(note) = self.current_note() {
            self.note_lines = note.content.lines().map(|l| l.to_string()).collect();
            if self.note_lines.is_empty() {
                self.note_lines.push(String::new());
            }
        } else {
            self.note_lines = vec![String::new()];
        }
        self.note_cursor_line = 0;
        self.note_cursor_col = 0;
        self.note_scroll = 0;
        self.note_mode = NoteMode::Editing;
    }

    pub fn note_cursor_insert(&mut self, c: char) {
        if self.note_lines.is_empty() {
            self.note_lines.push(String::new());
        }
        let line = &mut self.note_lines[self.note_cursor_line];
        if self.note_cursor_col <= line.len() {
            line.insert(self.note_cursor_col, c);
        } else {
            line.push(c);
        }
        self.note_cursor_col += 1;
    }

    pub fn note_cursor_backspace(&mut self) {
        if self.note_cursor_col > 0 {
            let line = &mut self.note_lines[self.note_cursor_line];
            line.remove(self.note_cursor_col - 1);
            self.note_cursor_col -= 1;
        } else if self.note_cursor_line > 0 {
            let below = self.note_lines.remove(self.note_cursor_line);
            self.note_cursor_line -= 1;
            self.note_cursor_col = self.note_lines[self.note_cursor_line].len();
            self.note_lines[self.note_cursor_line].push_str(&below);
        }
    }

    pub fn note_cursor_delete(&mut self) {
        let line = &mut self.note_lines[self.note_cursor_line];
        if self.note_cursor_col < line.len() {
            line.remove(self.note_cursor_col);
        } else if self.note_cursor_line + 1 < self.note_lines.len() {
            let next = self.note_lines.remove(self.note_cursor_line + 1);
            self.note_lines[self.note_cursor_line].push_str(&next);
        }
    }

    pub fn note_cursor_enter(&mut self) {
        if self.note_lines.is_empty() {
            self.note_lines.push(String::new());
        }
        let line = &mut self.note_lines[self.note_cursor_line];
        let rest = if self.note_cursor_col <= line.len() {
            line.split_off(self.note_cursor_col)
        } else {
            String::new()
        };
        self.note_cursor_line += 1;
        self.note_lines.insert(self.note_cursor_line, rest);
        self.note_cursor_col = 0;
    }

    pub fn note_cursor_up(&mut self) {
        if self.note_cursor_line > 0 {
            self.note_cursor_line -= 1;
            self.note_cursor_col = self
                .note_cursor_col
                .min(self.note_lines[self.note_cursor_line].len());
            if self.note_cursor_line < self.note_scroll {
                self.note_scroll = self.note_cursor_line;
            }
        }
    }

    pub fn note_cursor_down(&mut self) {
        if self.note_cursor_line + 1 < self.note_lines.len() {
            self.note_cursor_line += 1;
            self.note_cursor_col = self
                .note_cursor_col
                .min(self.note_lines[self.note_cursor_line].len());
            let max_scroll = self.note_lines.len().saturating_sub(1);
            if self.note_cursor_line >= self.note_scroll + max_scroll.min(20) {
                self.note_scroll = self.note_cursor_line.saturating_sub(19);
            }
        }
    }

    pub fn note_cursor_left(&mut self) {
        if self.note_cursor_col > 0 {
            self.note_cursor_col -= 1;
        } else if self.note_cursor_line > 0 {
            self.note_cursor_line -= 1;
            self.note_cursor_col = self.note_lines[self.note_cursor_line].len();
        }
    }

    pub fn note_cursor_right(&mut self) {
        if self.note_cursor_line < self.note_lines.len() {
            let line_len = self.note_lines[self.note_cursor_line].len();
            if self.note_cursor_col < line_len {
                self.note_cursor_col += 1;
            } else if self.note_cursor_line + 1 < self.note_lines.len() {
                self.note_cursor_line += 1;
                self.note_cursor_col = 0;
            }
        }
    }

    pub fn note_cursor_home(&mut self) {
        self.note_cursor_col = 0;
    }

    pub fn note_cursor_end(&mut self) {
        if self.note_cursor_line < self.note_lines.len() {
            self.note_cursor_col = self.note_lines[self.note_cursor_line].len();
        }
    }

    pub fn visible_count(&self) -> usize {
        self.fuzzy_filter_todos().len()
    }

    pub fn selected_todo_index(&self) -> Option<usize> {
        let filtered = self.fuzzy_filter_todos();
        filtered.get(self.selected_index).map(|(i, _)| *i)
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
                self.start_edit_note();
            }
            _ => {}
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

    pub fn note_scroll_up(&mut self) {
        if self.note_scroll > 0 {
            self.note_scroll -= 1;
        }
    }

    pub fn note_scroll_down(&mut self) {
        self.note_scroll += 1;
    }

    pub fn open_palette(&mut self, kind: PaletteKind) {
        self.input_mode = InputMode::Palette;
        self.palette_kind = kind;
        self.palette_query.clear();
        self.palette_selected = 0;
        self.refresh_palette();
    }

    pub fn close_palette(&mut self) {
        self.input_mode = InputMode::Normal;
        self.palette_query.clear();
        self.palette_items.clear();
        self.palette_selected = 0;
    }

    pub fn palette_type_char(&mut self, c: char) {
        self.palette_query.push(c);
        self.palette_selected = 0;
        self.refresh_palette();
    }

    pub fn palette_backspace(&mut self) {
        self.palette_query.pop();
        self.palette_selected = 0;
        self.refresh_palette();
    }

    pub fn palette_move_up(&mut self) {
        if self.palette_selected > 0 {
            self.palette_selected -= 1;
        }
    }

    pub fn palette_move_down(&mut self) {
        if self.palette_selected + 1 < self.palette_items.len() {
            self.palette_selected += 1;
        }
    }

    pub fn palette_select(&mut self) {
        let action = self
            .palette_items
            .get(self.palette_selected)
            .map(|item| match &item.action {
                PaletteAction::OpenNote(id) => PaletteAction::OpenNote(*id),
                PaletteAction::CreateNote(title) => PaletteAction::CreateNote(title.clone()),
                PaletteAction::CreateTodo(desc) => PaletteAction::CreateTodo(desc.clone()),
            });

        match action {
            Some(PaletteAction::OpenNote(id)) => {
                self.close_palette();
                self.view = View::Note;
                self.panel = Panel::Main;
                self.note_mode = NoteMode::Viewing;
                let items = self.side_items();
                if let Some(idx) = items
                    .iter()
                    .position(|item| matches!(item, SideItem::Note(note_id, _) if *note_id == id))
                {
                    self.side_index = idx;
                }
            }
            Some(PaletteAction::CreateNote(title)) => {
                self.close_palette();
                let now = Local::now().naive_local();
                let content = if title.is_empty() {
                    String::new()
                } else {
                    format!("# {}\n", title)
                };
                let note = Note {
                    id: storage::next_id(),
                    title: extract_title(&content),
                    content,
                    created_at: now,
                    updated_at: now,
                };
                self.notes.push(note);
                self.notes.sort_by_key(|n| n.id);
                storage::save_notes(&self.notes_path, &self.notes);
                self.view = View::Note;
                self.panel = Panel::Main;
                self.side_index = 2 + self.notes.len().saturating_sub(1);
                self.start_edit_note();
            }
            Some(PaletteAction::CreateTodo(desc)) => {
                self.close_palette();
                let desc = desc.trim().to_string();
                if !desc.is_empty() {
                    self.todos.push(Todo {
                        id: storage::next_id(),
                        description: desc,
                        done: false,
                        archived: false,
                        created_at: Local::now().naive_local(),
                    });
                    self.todos.sort_by_key(|t| t.id);
                    storage::save(&self.storage_path, &self.todos);
                }
                self.view = View::Todos;
                self.show_archived = false;
                self.panel = Panel::Main;
                self.selected_index = 0;
            }
            None => {}
        }
    }

    pub fn refresh_palette(&mut self) {
        let query = self.palette_query.trim();
        let mut items: Vec<PaletteItem> = Vec::new();

        match self.palette_kind {
            PaletteKind::Omni | PaletteKind::Notes => {
                let candidates: Vec<String> = self
                    .notes
                    .iter()
                    .map(|n| format!("{} {}", n.title, preview(&n.content)))
                    .collect();
                let matches = crate::fuzzy::filter(query, &candidates);

                for (note_idx, m) in matches {
                    let note = &self.notes[note_idx];
                    let title_matches: Vec<usize> = if query.is_empty() {
                        Vec::new()
                    } else {
                        crate::fuzzy::fuzzy_match(query, &note.title)
                            .map(|fm| fm.indices)
                            .unwrap_or_default()
                    };
                    items.push(PaletteItem {
                        title: note.title.clone(),
                        subtitle: preview(&note.content),
                        icon: '📝',
                        action: PaletteAction::OpenNote(note.id),
                        score: m.score,
                        matches: title_matches,
                    });
                }
            }
        }

        if !query.is_empty() {
            let create_score = 1000_i64; // Always keep create actions near the top.
            if matches!(self.palette_kind, PaletteKind::Omni | PaletteKind::Notes) {
                items.push(PaletteItem {
                    title: format!("Create note: {}", query),
                    subtitle: "Start a new note".to_string(),
                    icon: '✨',
                    action: PaletteAction::CreateNote(query.to_string()),
                    score: create_score,
                    matches: Vec::new(),
                });
            }
            if matches!(self.palette_kind, PaletteKind::Omni) {
                items.push(PaletteItem {
                    title: format!("Create todo: {}", query),
                    subtitle: "Add a new todo".to_string(),
                    icon: '✅',
                    action: PaletteAction::CreateTodo(query.to_string()),
                    score: create_score - 1,
                    matches: Vec::new(),
                });
            }
        }

        if query.is_empty() {
            // Stable order for empty query.
            items.sort_by_key(|item| match &item.action {
                PaletteAction::OpenNote(id) => *id,
                _ => u64::MAX,
            });
        } else {
            items.sort_by_key(|b| std::cmp::Reverse(b.score));
        }

        self.palette_items = items;
        self.palette_selected = self
            .palette_selected
            .min(self.palette_items.len().saturating_sub(1));
    }

    pub fn fuzzy_filter_todos(&self) -> Vec<(usize, crate::fuzzy::FuzzyMatch)> {
        let q = self.search_query.trim();
        let candidates: Vec<String> = self.todos.iter().map(|t| t.description.clone()).collect();
        crate::fuzzy::filter(q, &candidates)
            .into_iter()
            .filter(|(idx, _)| self.show_archived == self.todos[*idx].archived)
            .collect()
    }
}

fn preview(content: &str) -> String {
    content
        .lines()
        .find(|l| !l.trim().is_empty())
        .unwrap_or("")
        .chars()
        .take(80)
        .collect::<String>()
        .trim()
        .to_string()
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
