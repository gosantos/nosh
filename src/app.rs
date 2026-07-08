use std::path::PathBuf;

use chrono::{Duration, Local};

use crate::storage::{self, Note, Todo};

const FUNNY_PLACEHOLDERS: &[&str] = &[
    "Buy more todo apps",
    "Delete all todos and start fresh",
    "Write a todo about writing todos",
    "Nap aggressively",
    "Stare at ceiling until epiphany occurs",
    "Panic in an organized fashion",
    "Schedule panic for later",
    "Fix the thing I broke fixing the other thing",
    "Pet the cat (critical priority)",
    "Contemplate existence of unpaid invoices",
    "Overthink this task",
    "Add 'learn to say no' to tomorrow's list",
    "Drink coffee, then panic, then coffee again",
    "Pretend to be productive",
    "Optimize something that didn't need optimizing",
    "Write beautiful code nobody will read",
    "Reply to that email from 2019",
    "Explain to rubber duck why this bug isn't my fault",
    "Find the bug I wrote at 3am",
    "Convince myself this is the last slide",
    "Reorganize desk as proxy for reorganizing life",
    "Postpone procrastination",
    "Debug my debugging strategy",
    "Turn it off and on again (emotionally)",
    "Invent new word for procrastination",
    "Microdose productivity",
    "Aggressively close all browser tabs",
    "Wonder where the day went",
    "Add glitter to the burn-down chart",
    "Figure out why it works before it stops working",
];

pub enum InputMode {
    Normal,
    Palette,
    Search,
    Renaming,
    NoteSearch,
    Creating,
    Editing,
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

pub enum UndoState {
    Inactive,
    TodoDeleted(Todo),
    NoteDeleted(Note),
}

impl UndoState {
    pub fn is_active(&self) -> bool {
        !matches!(self, UndoState::Inactive)
    }
}

pub struct App {
    pub todos: Vec<Todo>,
    pub notes: Vec<Note>,
    pub input_mode: InputMode,
    pub selected_index: usize,
    pub should_quit: bool,
    pub show_archived: bool,
    pub panel: Panel,
    pub view: View,
    pub note_mode: NoteMode,
    pub list_scroll: usize,
    pub note_scroll: usize,
    pub note_view_max_scroll: usize,
    pub note_view_page_size: usize,
    pub note_cursor_line: usize,
    pub note_cursor_col: usize,
    pub note_lines: Vec<String>,
    pub side_index: usize,
    pub palette_query: String,
    pub palette_items: Vec<PaletteItem>,
    pub palette_selected: usize,
    pub storage_path: PathBuf,
    notes_path: PathBuf,
    pub search_filter: Option<String>,
    pub search_buffer: String,
    pub undo_state: UndoState,
    pub rename_buffer: String,
    pub note_search_filter: Option<String>,
    pub note_search_buffer: String,
    pub create_buffer: String,
    pub create_placeholder: String,
    placeholder_idx: usize,
    pub edit_buffer: String,
    pub edit_todo_index: Option<usize>,
}

fn side_items(
    view: &View,
    show_archived: bool,
    notes: &[Note],
    note_filter: Option<&str>,
) -> Vec<SideItem> {
    let mut items = vec![
        SideItem::Active(view == &View::Todos && !show_archived),
        SideItem::Archive(view == &View::Todos && show_archived),
        SideItem::NotesHeader(notes.len()),
    ];
    items.extend(
        notes
            .iter()
            .filter(|n| {
                note_filter.is_none_or(|q| {
                    let q = q.to_lowercase();
                    n.title.to_lowercase().contains(&q) || n.content.to_lowercase().contains(&q)
                })
            })
            .map(|n| SideItem::Note(n.id, n.title.clone())),
    );
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
        let notes_path = storage_path.with_file_name("notes.json");
        let notes = storage::load_notes(&notes_path);
        App {
            todos,
            notes,
            input_mode: InputMode::Normal,
            selected_index: 0,
            should_quit: false,
            show_archived: false,
            panel: Panel::Main,
            view: View::Todos,
            note_mode: NoteMode::Viewing,
            list_scroll: 0,
            note_scroll: 0,
            note_view_max_scroll: 0,
            note_view_page_size: 0,
            note_cursor_line: 0,
            note_cursor_col: 0,
            note_lines: Vec::new(),
            side_index: 0,
            palette_query: String::new(),
            palette_items: Vec::new(),
            palette_selected: 0,
            storage_path,
            notes_path,
            search_filter: None,
            search_buffer: String::new(),
            undo_state: UndoState::Inactive,
            rename_buffer: String::new(),
            note_search_filter: None,
            note_search_buffer: String::new(),
            create_buffer: String::new(),
            create_placeholder: String::new(),
            placeholder_idx: 0,
            edit_buffer: String::new(),
            edit_todo_index: None,
        }
    }

    pub fn side_items(&self) -> Vec<SideItem> {
        side_items(
            &self.view,
            self.show_archived,
            &self.notes,
            self.note_search_filter.as_deref(),
        )
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
            let items = self.side_items();
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

    pub fn visible_indices(&self) -> Vec<usize> {
        let indices: Vec<usize> = self
            .todos
            .iter()
            .enumerate()
            .rev()
            .filter(|(_, t)| t.archived == self.show_archived)
            .filter(|(_, t)| {
                self.search_filter.as_ref().is_none_or(|query| {
                    t.description.to_lowercase().contains(&query.to_lowercase())
                })
            })
            .map(|(i, _)| i)
            .collect();
        indices
    }

    pub fn visible_count(&self) -> usize {
        self.visible_indices().len()
    }

    pub fn selected_todo_index(&self) -> Option<usize> {
        self.visible_indices().get(self.selected_index).copied()
    }

    fn clamp_selection(&mut self) {
        let count = self.visible_count();
        if count == 0 {
            self.selected_index = 0;
        } else if self.selected_index >= count {
            self.selected_index = count - 1;
        }
    }

    pub fn push_todo(&mut self, description: String) {
        self.todos.push(Todo {
            id: storage::next_id(),
            description,
            done: false,
            archived: false,
            created_at: Local::now().naive_local(),
            completed_at: None,
        });
        self.selected_index = 0;
        storage::save(&self.storage_path, &self.todos);
    }

    pub fn start_editing(&mut self) {
        if let Some(idx) = self.selected_todo_index() {
            self.edit_buffer = self.todos[idx].description.clone();
            self.edit_todo_index = Some(idx);
            self.input_mode = InputMode::Editing;
        }
    }

    pub fn confirm_editing(&mut self) {
        let desc = self.edit_buffer.trim().to_string();
        if !desc.is_empty() {
            if let Some(idx) = self.edit_todo_index {
                self.todos[idx].description = desc;
                storage::save(&self.storage_path, &self.todos);
            }
        }
        self.edit_buffer.clear();
        self.edit_todo_index = None;
        self.input_mode = InputMode::Normal;
    }

    pub fn cancel_editing(&mut self) {
        self.edit_buffer.clear();
        self.edit_todo_index = None;
        self.input_mode = InputMode::Normal;
    }

    pub fn edit_type_char(&mut self, c: char) {
        self.edit_buffer.push(c);
    }

    pub fn edit_backspace(&mut self) {
        self.edit_buffer.pop();
    }

    pub fn start_creating(&mut self) {
        self.view = View::Todos;
        self.panel = Panel::Main;
        self.show_archived = false;
        self.create_buffer.clear();
        self.input_mode = InputMode::Creating;
        self.selected_index = 0;
        self.list_scroll = 0;
        self.create_placeholder = FUNNY_PLACEHOLDERS[self.placeholder_idx].to_string();
        self.placeholder_idx = (self.placeholder_idx + 1) % FUNNY_PLACEHOLDERS.len();
    }

    pub fn confirm_creating(&mut self) {
        let desc = self.create_buffer.trim().to_string();
        if !desc.is_empty() {
            self.push_todo(desc);
        }
        self.create_buffer.clear();
        self.input_mode = InputMode::Normal;
    }

    pub fn cancel_creating(&mut self) {
        self.create_buffer.clear();
        self.input_mode = InputMode::Normal;
    }

    pub fn create_type_char(&mut self, c: char) {
        self.create_buffer.push(c);
    }

    pub fn create_backspace(&mut self) {
        self.create_buffer.pop();
    }

    pub fn toggle_done(&mut self) {
        if let Some(idx) = self.selected_todo_index() {
            let todo = &mut self.todos[idx];
            todo.done = !todo.done;
            todo.completed_at = if todo.done {
                Some(Local::now().naive_local())
            } else {
                None
            };
            storage::save(&self.storage_path, &self.todos);
        }
    }

    pub fn archive_selected(&mut self) {
        if let Some(idx) = self.selected_todo_index() {
            let todo = &mut self.todos[idx];
            todo.archived = !todo.archived;
            if todo.archived {
                todo.done = true;
                todo.completed_at = Some(Local::now().naive_local());
            } else {
                todo.done = false;
                todo.completed_at = None;
            }
            storage::save(&self.storage_path, &self.todos);
            self.clamp_selection();
        }
    }

    pub fn delete_selected(&mut self) {
        if let Some(idx) = self.selected_todo_index() {
            let todo = self.todos.remove(idx);
            storage::save(&self.storage_path, &self.todos);
            self.clamp_selection();
            self.undo_state = UndoState::TodoDeleted(todo);
        }
    }

    pub fn undo_delete(&mut self) {
        let restored = match self.undo_state {
            UndoState::TodoDeleted(ref todo) => {
                self.todos.push(todo.clone());
                storage::save(&self.storage_path, &self.todos);
                self.selected_index = 0;
                true
            }
            UndoState::NoteDeleted(ref note) => {
                self.notes.push(note.clone());
                self.notes.sort_by_key(|n| n.id);
                storage::save_notes(&self.notes_path, &self.notes);
                true
            }
            UndoState::Inactive => false,
        };
        if restored {
            self.undo_state = UndoState::Inactive;
        }
    }

    pub fn clear_undo(&mut self) {
        self.undo_state = UndoState::Inactive;
    }

    pub fn archive_old(&mut self) {
        let threshold = Local::now().naive_local() - Duration::days(7);
        let mut changed = false;
        for todo in &mut self.todos {
            if !todo.archived && todo.created_at < threshold {
                todo.archived = true;
                todo.done = true;
                todo.completed_at = Some(Local::now().naive_local());
                changed = true;
            }
        }
        if changed {
            storage::save(&self.storage_path, &self.todos);
        }
    }

    pub fn reload(&mut self) {
        let todos = storage::load(&self.storage_path);
        let mut sorted = todos;
        sorted.sort_by_key(|t| t.id);
        self.todos = sorted;
        self.clamp_selection();

        let notes = storage::load_notes(&self.notes_path);
        self.notes = notes;

        let count = self.side_count();
        if self.side_index >= count {
            self.side_index = count.saturating_sub(1);
        }

        if matches!(self.input_mode, InputMode::Palette) {
            self.refresh_palette();
        }
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
                self.note_view_max_scroll = 0;
                self.start_edit_note();
            }
            _ => {}
        }
    }

    pub fn delete_note_by_side_index(&mut self) {
        let note_idx = {
            let items = self.side_items();
            match items.get(self.side_index) {
                Some(SideItem::Note(id, _)) => match self.notes.iter().position(|n| n.id == *id) {
                    Some(idx) => idx,
                    None => return,
                },
                _ => return,
            }
        };
        let note = self.notes.remove(note_idx);
        if self.side_index >= self.side_count() && self.side_index > 0 {
            self.side_index -= 1;
        }
        if self.notes.is_empty() && self.side_index == 0 {
            self.view = View::Todos;
        }
        storage::save_notes(&self.notes_path, &self.notes);
        self.undo_state = UndoState::NoteDeleted(note);
    }

    pub fn start_rename(&mut self) {
        let items = self.side_items();
        if let Some(SideItem::Note(_, title)) = items.get(self.side_index) {
            self.rename_buffer = title.clone();
            self.input_mode = InputMode::Renaming;
        }
    }

    pub fn confirm_rename(&mut self) {
        let new_title = self.rename_buffer.trim().to_string();
        if new_title.is_empty() {
            self.cancel_rename();
            return;
        }
        let note_id = {
            let items = self.side_items();
            match items.get(self.side_index) {
                Some(SideItem::Note(id, _)) => *id,
                _ => {
                    self.cancel_rename();
                    return;
                }
            }
        };
        if let Some(note) = self.notes.iter_mut().find(|n| n.id == note_id) {
            note.title = new_title;
            note.updated_at = Local::now().naive_local();
            storage::save_notes(&self.notes_path, &self.notes);
        }
        self.input_mode = InputMode::Normal;
        self.rename_buffer.clear();
    }

    pub fn cancel_rename(&mut self) {
        self.input_mode = InputMode::Normal;
        self.rename_buffer.clear();
    }

    pub fn rename_type_char(&mut self, c: char) {
        self.rename_buffer.push(c);
    }

    pub fn rename_backspace(&mut self) {
        self.rename_buffer.pop();
    }

    pub fn duplicate_note_by_side_index(&mut self) {
        let original = {
            let items = self.side_items();
            match items.get(self.side_index) {
                Some(SideItem::Note(id, _)) => match self.notes.iter().find(|n| n.id == *id) {
                    Some(n) => n.clone(),
                    None => return,
                },
                _ => return,
            }
        };
        let new_title = if original.title.is_empty() {
            String::new()
        } else {
            format!("{} (copy)", original.title)
        };
        let now = Local::now().naive_local();
        let new_id = storage::next_id();
        self.notes.push(Note {
            id: new_id,
            title: new_title,
            content: original.content,
            created_at: now,
            updated_at: now,
        });
        self.notes.sort_by_key(|n| n.id);
        storage::save_notes(&self.notes_path, &self.notes);
        let items = self.side_items();
        if let Some(pos) = items
            .iter()
            .position(|item| matches!(item, SideItem::Note(id, _) if *id == new_id))
        {
            self.side_index = pos;
        }
    }

    pub fn start_note_search(&mut self) {
        self.note_search_buffer.clear();
        self.input_mode = InputMode::NoteSearch;
    }

    pub fn apply_note_search(&mut self) {
        let query = self.note_search_buffer.trim().to_string();
        if query.is_empty() {
            self.note_search_filter = None;
        } else {
            self.note_search_filter = Some(query);
        }
        self.input_mode = InputMode::Normal;
        if self.side_index >= self.side_count() {
            self.side_index = self.side_count().saturating_sub(1);
        }
    }

    pub fn cancel_note_search(&mut self) {
        self.note_search_buffer.clear();
        self.note_search_filter = None;
        self.input_mode = InputMode::Normal;
    }

    pub fn note_search_buffer_push(&mut self, c: char) {
        self.note_search_buffer.push(c);
        let query = self.note_search_buffer.trim();
        if query.is_empty() {
            self.note_search_filter = None;
        } else {
            self.note_search_filter = Some(query.to_string());
        }
        if self.side_index >= self.side_count() {
            self.side_index = self.side_count().saturating_sub(1);
        }
    }

    pub fn note_search_buffer_pop(&mut self) {
        self.note_search_buffer.pop();
        let query = self.note_search_buffer.trim();
        if query.is_empty() {
            self.note_search_filter = None;
        } else {
            self.note_search_filter = Some(query.to_string());
        }
        if self.side_index >= self.side_count() {
            self.side_index = self.side_count().saturating_sub(1);
        }
    }

    pub fn note_scroll_up(&mut self) {
        if self.note_scroll > 0 {
            self.note_scroll -= 1;
        }
    }

    pub fn note_scroll_down(&mut self) {
        let max = self.note_view_max_scroll;
        if self.note_scroll < max {
            self.note_scroll += 1;
        }
    }

    pub fn note_scroll_page_down(&mut self) {
        let max = self.note_view_max_scroll;
        let size = self.note_view_page_size.max(1);
        self.note_scroll = (self.note_scroll + size).min(max);
    }

    pub fn note_scroll_page_up(&mut self) {
        let size = self.note_view_page_size.max(1);
        self.note_scroll = self.note_scroll.saturating_sub(size);
    }

    pub fn note_scroll_top(&mut self) {
        self.note_scroll = 0;
    }

    pub fn note_scroll_bottom(&mut self) {
        self.note_scroll = self.note_view_max_scroll;
    }

    pub fn open_palette(&mut self) {
        self.input_mode = InputMode::Palette;
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
                self.note_scroll = 0;
                self.note_view_max_scroll = 0;
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
                    self.push_todo(desc);
                }
                self.view = View::Todos;
                self.show_archived = false;
                self.panel = Panel::Main;
                self.selected_index = 0;
            }
            None => {}
        }
    }

    pub fn start_search(&mut self) {
        if self.search_filter.is_none() {
            self.search_buffer.clear();
        }
        self.input_mode = InputMode::Search;
    }

    pub fn apply_search(&mut self) {
        let query = self.search_buffer.trim().to_string();
        if query.is_empty() {
            self.search_filter = None;
        } else {
            self.search_filter = Some(query);
        }
        self.input_mode = InputMode::Normal;
        self.clamp_selection();
    }

    pub fn cancel_search(&mut self) {
        self.search_buffer.clear();
        self.search_filter = None;
        self.input_mode = InputMode::Normal;
        self.clamp_selection();
    }

    pub fn search_buffer_push(&mut self, c: char) {
        self.search_buffer.push(c);
        let query = self.search_buffer.trim();
        if query.is_empty() {
            self.search_filter = None;
        } else {
            self.search_filter = Some(query.to_string());
        }
        self.clamp_selection();
    }

    pub fn search_buffer_pop(&mut self) {
        self.search_buffer.pop();
        let query = self.search_buffer.trim();
        if query.is_empty() {
            self.search_filter = None;
        } else {
            self.search_filter = Some(query.to_string());
        }
        self.clamp_selection();
    }

    pub fn refresh_palette(&mut self) {
        let query = self.palette_query.trim();
        let mut items: Vec<PaletteItem> = Vec::new();

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
                icon: '\u{1F4DD}',
                action: PaletteAction::OpenNote(note.id),
                score: m.score,
                matches: title_matches,
            });
        }

        if !query.is_empty() {
            let create_score = 1000_i64;
            items.push(PaletteItem {
                title: format!("Create note: {}", query),
                subtitle: "Start a new note".to_string(),
                icon: '\u{2728}',
                action: PaletteAction::CreateNote(query.to_string()),
                score: create_score,
                matches: Vec::new(),
            });
            items.push(PaletteItem {
                title: format!("Create todo: {}", query),
                subtitle: "Add a new todo".to_string(),
                icon: '\u{2705}',
                action: PaletteAction::CreateTodo(query.to_string()),
                score: create_score - 1,
                matches: Vec::new(),
            });
        }

        if query.is_empty() {
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
    use std::fs;
    use tempfile::TempDir;

    fn make_todo(id: u64, desc: &str, done: bool) -> Todo {
        Todo {
            id,
            description: desc.to_string(),
            done,
            archived: false,
            created_at: Local::now().naive_local(),
            completed_at: if done {
                Some(Local::now().naive_local())
            } else {
                None
            },
        }
    }

    fn make_note(id: u64, title: &str) -> Note {
        Note {
            id,
            title: title.to_string(),
            content: format!("# {title}"),
            created_at: Local::now().naive_local(),
            updated_at: Local::now().naive_local(),
        }
    }

    fn setup(todos: Vec<Todo>, notes: Vec<Note>) -> (TempDir, App) {
        let dir = TempDir::new().unwrap();
        let storage_path = dir.path().join("todos.json");
        fs::write(&storage_path, serde_json::to_string_pretty(&todos).unwrap()).unwrap();
        let notes_path = dir.path().join("notes.json");
        fs::write(&notes_path, serde_json::to_string_pretty(&notes).unwrap()).unwrap();
        let app = App::new(storage_path);
        (dir, app)
    }

    #[test]
    fn reload_picks_up_new_todo() {
        let (_dir, mut app) = setup(vec![make_todo(1, "existing", false)], vec![]);
        assert_eq!(app.todos.len(), 1);

        let mut todos = storage::load(&app.storage_path);
        todos.push(make_todo(2, "added via cli", false));
        storage::save(&app.storage_path, &todos);

        app.reload();

        assert_eq!(app.todos.len(), 2);
        assert!(app.todos.iter().any(|t| t.description == "added via cli"));
    }

    #[test]
    fn reload_picks_up_deleted_todo() {
        let (_dir, mut app) = setup(
            vec![make_todo(1, "first", false), make_todo(2, "second", false)],
            vec![],
        );
        assert_eq!(app.todos.len(), 2);

        let todos: Vec<Todo> = storage::load(&app.storage_path)
            .into_iter()
            .filter(|t| t.id != 1)
            .collect();
        storage::save(&app.storage_path, &todos);

        app.reload();

        assert_eq!(app.todos.len(), 1);
        assert_eq!(app.todos[0].id, 2);
    }

    #[test]
    fn reload_picks_up_new_note() {
        let (_dir, mut app) = setup(vec![], vec![make_note(1, "first")]);
        assert_eq!(app.notes.len(), 1);

        let mut notes = storage::load_notes(&app.notes_path);
        notes.push(make_note(2, "added via cli"));
        notes.sort_by_key(|n| n.id);
        storage::save_notes(&app.notes_path, &notes);

        app.reload();

        assert_eq!(app.notes.len(), 2);
    }

    #[test]
    fn reload_clamps_selection_when_all_todos_gone() {
        let (_dir, mut app) = setup(
            vec![make_todo(1, "a", false), make_todo(2, "b", false)],
            vec![],
        );
        app.selected_index = 1;

        storage::save(&app.storage_path, &[] as &[Todo]);
        app.reload();

        assert!(app.todos.is_empty());
        assert_eq!(app.selected_index, 0);
    }

    #[test]
    fn reload_handles_empty_files() {
        let (_dir, mut app) = setup(
            vec![make_todo(1, "gone", false)],
            vec![make_note(1, "gone")],
        );

        storage::save(&app.storage_path, &[] as &[Todo]);
        storage::save_notes(&app.notes_path, &[] as &[Note]);

        app.reload();

        assert!(app.todos.is_empty());
        assert!(app.notes.is_empty());
    }
}
