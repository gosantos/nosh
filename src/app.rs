use std::path::PathBuf;

use chrono::{Datelike, Duration, Local, NaiveDate};

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
    Search,
    Creating,
    Editing,
    ConfirmDelete,
}

pub enum NoteMode {
    Viewing,
    Editing,
}

#[derive(Debug, PartialEq, Eq)]
pub enum Panel {
    Main,
    Sidebar,
}

#[derive(Debug, PartialEq, Eq)]
pub enum View {
    Todos,
    Note,
    Notes,
}

pub enum UndoState {
    Inactive,
    TodoDeleted(Todo),
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
    pub current_note_index: Option<usize>,
    pub storage_path: PathBuf,
    notes_path: PathBuf,
    pub search_filter: Option<String>,
    pub search_buffer: String,
    pub undo_state: UndoState,
    pub create_buffer: String,
    pub create_placeholder: String,
    placeholder_idx: usize,
    pub edit_buffer: String,
    pub edit_todo_index: Option<usize>,
    pub confirm_selection: usize,
}

fn side_items(todos: &[Todo], notes: &[Note]) -> Vec<SideItem> {
    vec![
        SideItem::Todos(todos.iter().filter(|t| !t.archived).count()),
        SideItem::Archive(todos.iter().filter(|t| t.archived).count()),
        SideItem::Notes(notes.len()),
    ]
}

pub enum SideItem {
    Todos(usize),
    Archive(usize),
    Notes(usize),
}

impl SideItem {
    pub fn is_active(&self, view: &View, show_archived: bool) -> bool {
        match self {
            SideItem::Todos(_) => *view == View::Todos && !show_archived,
            SideItem::Archive(_) => *view == View::Todos && show_archived,
            SideItem::Notes(_) => matches!(view, View::Note | View::Notes),
        }
    }
}

#[derive(Clone)]
pub enum VisibleEntry {
    GroupHeader(String),
    Todo(usize),
}

fn date_label(date: NaiveDate, today: NaiveDate) -> String {
    if date == today {
        format!("Today (W{})", date.iso_week().week())
    } else if date == today - Duration::days(1) {
        "Yesterday".to_string()
    } else if date > today - Duration::days(7) {
        format!("{} (W{})", date.format("%A"), date.iso_week().week())
    } else if date.year() == today.year() {
        date.format("%B %d").to_string()
    } else {
        date.format("%B %d, %Y").to_string()
    }
}

impl App {
    fn matches_view(&self, todo: &Todo) -> bool {
        todo.archived == self.show_archived
            && self.search_filter.as_ref().is_none_or(|query| {
                todo.description
                    .to_lowercase()
                    .contains(&query.to_lowercase())
            })
    }

    pub fn visible_entries(&self) -> Vec<VisibleEntry> {
        let today = Local::now().naive_local().date();
        let mut entries = Vec::new();
        let mut prev_date: Option<NaiveDate> = None;

        for idx in self.visible_indices() {
            let date = self.todos[idx].created_at.date();
            if prev_date != Some(date) {
                prev_date = Some(date);
                entries.push(VisibleEntry::GroupHeader(date_label(date, today)));
            }
            entries.push(VisibleEntry::Todo(idx));
        }

        entries
    }

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
            current_note_index: None,
            storage_path,
            notes_path,
            search_filter: None,
            search_buffer: String::new(),
            undo_state: UndoState::Inactive,
            create_buffer: String::new(),
            create_placeholder: String::new(),
            placeholder_idx: 0,
            edit_buffer: String::new(),
            edit_todo_index: None,
            confirm_selection: 0,
        }
    }

    pub fn side_items(&self) -> Vec<SideItem> {
        side_items(&self.todos, &self.notes)
    }

    pub fn side_count(&self) -> usize {
        self.side_items().len()
    }

    pub fn current_note(&self) -> Option<&Note> {
        self.current_note_index.and_then(|i| self.notes.get(i))
    }

    pub fn save_current_note(&mut self) {
        let content = self.note_lines.join("\n");
        let title = storage::extract_title(&content);
        if let Some(note) = self.current_note_index.and_then(|i| self.notes.get_mut(i)) {
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

    /// `note_cursor_col` counts characters; string mutations need the
    /// corresponding byte offset to stay on a char boundary.
    fn current_line(&self) -> &str {
        &self.note_lines[self.note_cursor_line]
    }

    pub fn note_cursor_insert(&mut self, c: char) {
        if self.note_lines.is_empty() {
            self.note_lines.push(String::new());
        }
        let idx = byte_index(self.current_line(), self.note_cursor_col);
        self.note_lines[self.note_cursor_line].insert(idx, c);
        self.note_cursor_col += 1;
    }

    pub fn note_cursor_backspace(&mut self) {
        if self.note_cursor_col > 0 {
            let idx = byte_index(self.current_line(), self.note_cursor_col - 1);
            self.note_lines[self.note_cursor_line].remove(idx);
            self.note_cursor_col -= 1;
        } else if self.note_cursor_line > 0 {
            let below = self.note_lines.remove(self.note_cursor_line);
            self.note_cursor_line -= 1;
            self.note_cursor_col = char_count(self.current_line());
            self.note_lines[self.note_cursor_line].push_str(&below);
        }
    }

    pub fn note_cursor_delete(&mut self) {
        if self.note_cursor_col < char_count(self.current_line()) {
            let idx = byte_index(self.current_line(), self.note_cursor_col);
            self.note_lines[self.note_cursor_line].remove(idx);
        } else if self.note_cursor_line + 1 < self.note_lines.len() {
            let next = self.note_lines.remove(self.note_cursor_line + 1);
            self.note_lines[self.note_cursor_line].push_str(&next);
        }
    }

    pub fn note_cursor_enter(&mut self) {
        if self.note_lines.is_empty() {
            self.note_lines.push(String::new());
        }
        let idx = byte_index(self.current_line(), self.note_cursor_col);
        let rest = self.note_lines[self.note_cursor_line].split_off(idx);
        self.note_cursor_line += 1;
        self.note_lines.insert(self.note_cursor_line, rest);
        self.note_cursor_col = 0;
    }

    pub fn note_cursor_up(&mut self) {
        if self.note_cursor_line > 0 {
            self.note_cursor_line -= 1;
            self.note_cursor_col = self.note_cursor_col.min(char_count(self.current_line()));
        }
    }

    pub fn note_cursor_down(&mut self) {
        if self.note_cursor_line + 1 < self.note_lines.len() {
            self.note_cursor_line += 1;
            self.note_cursor_col = self.note_cursor_col.min(char_count(self.current_line()));
        }
    }

    pub fn note_cursor_left(&mut self) {
        if self.note_cursor_col > 0 {
            self.note_cursor_col -= 1;
        } else if self.note_cursor_line > 0 {
            self.note_cursor_line -= 1;
            self.note_cursor_col = char_count(self.current_line());
        }
    }

    pub fn note_cursor_right(&mut self) {
        if self.note_cursor_line < self.note_lines.len() {
            if self.note_cursor_col < char_count(self.current_line()) {
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
            self.note_cursor_col = char_count(self.current_line());
        }
    }

    pub fn visible_indices(&self) -> Vec<usize> {
        self.todos
            .iter()
            .enumerate()
            .rev()
            .filter(|(_, t)| self.matches_view(t))
            .map(|(i, _)| i)
            .collect()
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

    pub fn start_creating_note(&mut self) {
        let now = Local::now().naive_local();
        let note = Note {
            id: storage::next_id(),
            title: String::new(),
            content: String::new(),
            created_at: now,
            updated_at: now,
        };
        self.notes.push(note);
        self.notes.sort_by_key(|n| n.id);
        storage::save_notes(&self.notes_path, &self.notes);
        self.current_note_index = Some(self.notes.len().saturating_sub(1));
        self.view = View::Note;
        self.panel = Panel::Main;
        self.side_index = 2;
        self.start_edit_note();
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
            todo.set_done(!todo.done);
            storage::save(&self.storage_path, &self.todos);
        }
    }

    pub fn archive_selected(&mut self) {
        if let Some(idx) = self.selected_todo_index() {
            let todo = &mut self.todos[idx];
            todo.archived = !todo.archived;
            todo.set_done(todo.archived);
            storage::save(&self.storage_path, &self.todos);
            self.clamp_selection();
        }
    }

    pub fn undo_delete(&mut self) {
        if let UndoState::TodoDeleted(ref todo) = self.undo_state {
            self.todos.push(todo.clone());
            storage::save(&self.storage_path, &self.todos);
            self.selected_index = 0;
            self.undo_state = UndoState::Inactive;
        }
    }

    pub fn deletion_target_label(&self) -> String {
        match self.panel {
            Panel::Main => match self.view {
                View::Note => self
                    .current_note()
                    .map(|n| n.title.clone())
                    .unwrap_or_else(|| "this note".to_string()),
                View::Notes => self
                    .notes
                    .get(self.selected_index)
                    .map(|n| n.title.clone())
                    .unwrap_or_else(|| "this note".to_string()),
                View::Todos => self
                    .selected_todo_index()
                    .and_then(|i| self.todos.get(i))
                    .map(|t| t.description.clone())
                    .unwrap_or_else(|| "this todo".to_string()),
            },
            Panel::Sidebar => match self.side_index {
                0 => "all active todos? (not implemented)".to_string(),
                1 => "all archived todos? (not implemented)".to_string(),
                2 => self
                    .current_note()
                    .map(|n| n.title.clone())
                    .unwrap_or_else(|| "this note".to_string()),
                _ => "this item".to_string(),
            },
        }
    }

    pub fn start_deletion(&mut self) {
        self.input_mode = InputMode::ConfirmDelete;
        self.confirm_selection = 0;
    }

    pub fn confirm_move_left(&mut self) {
        self.confirm_selection = if self.confirm_selection == 0 { 1 } else { 0 };
    }

    pub fn confirm_move_right(&mut self) {
        self.confirm_selection = if self.confirm_selection == 1 { 0 } else { 1 };
    }

    /// Deletes the note open in the Note view. `empty_fallback` is the view
    /// shown when the last note is removed.
    fn delete_current_note(&mut self, empty_fallback: View) {
        if let Some(idx) = self.current_note_index {
            self.notes.remove(idx);
            if self.notes.is_empty() {
                self.current_note_index = None;
                self.view = empty_fallback;
            } else {
                self.current_note_index = Some(idx.min(self.notes.len() - 1));
            }
            storage::save_notes(&self.notes_path, &self.notes);
        }
    }

    pub fn confirm_delete(&mut self) {
        if self.confirm_selection == 1 {
            match self.panel {
                Panel::Main => match self.view {
                    View::Note => self.delete_current_note(View::Notes),
                    View::Notes => {
                        let idx = self.selected_index;
                        if idx < self.notes.len() {
                            self.notes.remove(idx);
                            if self.notes.is_empty() {
                                self.view = View::Todos;
                                self.selected_index = 0;
                            } else if self.selected_index >= self.notes.len() {
                                self.selected_index = self.notes.len() - 1;
                            }
                            self.current_note_index = None;
                            storage::save_notes(&self.notes_path, &self.notes);
                        }
                    }
                    View::Todos => {
                        if let Some(idx) = self.selected_todo_index() {
                            let todo = self.todos.remove(idx);
                            storage::save(&self.storage_path, &self.todos);
                            self.clamp_selection();
                            self.undo_state = UndoState::TodoDeleted(todo);
                        }
                    }
                },
                Panel::Sidebar => {
                    if self.side_index == 2 {
                        self.delete_current_note(View::Todos);
                    }
                }
            }
        }
        self.input_mode = InputMode::Normal;
    }

    pub fn cancel_confirm(&mut self) {
        self.input_mode = InputMode::Normal;
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
                todo.set_done(true);
                changed = true;
            }
        }
        if changed {
            storage::save(&self.storage_path, &self.todos);
        }
    }

    pub fn reload(&mut self) {
        self.todos = storage::load(&self.storage_path);
        self.todos.sort_by_key(|t| t.id);
        self.clamp_selection();

        self.notes = storage::load_notes(&self.notes_path);

        self.side_index = self.side_index.min(self.side_count() - 1);

        if let Some(idx) = self.current_note_index {
            if idx >= self.notes.len() {
                self.current_note_index = self.notes.len().checked_sub(1);
            }
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
        match self.side_index {
            0 => {
                self.view = View::Todos;
                self.show_archived = false;
                self.panel = Panel::Main;
                self.selected_index = 0;
            }
            1 => {
                self.view = View::Todos;
                self.show_archived = true;
                self.panel = Panel::Main;
                self.selected_index = 0;
            }
            2 => {
                self.view = View::Notes;
                self.panel = Panel::Main;
                self.selected_index = 0;
            }
            _ => {}
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

    pub fn start_search(&mut self) {
        if self.search_filter.is_none() {
            self.search_buffer.clear();
        }
        self.input_mode = InputMode::Search;
    }

    /// Derives the active filter from the search buffer and re-clamps the
    /// selection against the newly filtered list.
    fn sync_search_filter(&mut self) {
        let query = self.search_buffer.trim();
        self.search_filter = if query.is_empty() {
            None
        } else {
            Some(query.to_string())
        };
        self.clamp_selection();
    }

    pub fn apply_search(&mut self) {
        self.sync_search_filter();
        self.input_mode = InputMode::Normal;
    }

    pub fn cancel_search(&mut self) {
        self.search_buffer.clear();
        self.search_filter = None;
        self.input_mode = InputMode::Normal;
        self.clamp_selection();
    }

    pub fn search_buffer_push(&mut self, c: char) {
        self.search_buffer.push(c);
        self.sync_search_filter();
    }

    pub fn search_buffer_pop(&mut self) {
        self.search_buffer.pop();
        self.sync_search_filter();
    }
}

/// Byte offset of the `col`-th character of `line` (or the line's end).
pub fn byte_index(line: &str, col: usize) -> usize {
    line.char_indices()
        .nth(col)
        .map(|(i, _)| i)
        .unwrap_or(line.len())
}

fn char_count(line: &str) -> usize {
    line.chars().count()
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

    // --- visibility & filtering ---

    #[test]
    fn visible_indices_newest_first_and_respects_archived() {
        let mut archived = make_todo(3, "archived", true);
        archived.archived = true;
        let (_dir, mut app) = setup(
            vec![
                make_todo(1, "first", false),
                make_todo(2, "second", false),
                archived,
            ],
            vec![],
        );

        assert_eq!(app.visible_indices(), vec![1, 0]);

        app.show_archived = true;
        assert_eq!(app.visible_indices(), vec![2]);
    }

    #[test]
    fn search_filter_is_case_insensitive() {
        let (_dir, mut app) = setup(
            vec![
                make_todo(1, "Buy milk", false),
                make_todo(2, "Write code", false),
            ],
            vec![],
        );

        app.search_buffer = "MILK".to_string();
        app.apply_search();

        assert_eq!(app.visible_count(), 1);
        assert_eq!(app.todos[app.visible_indices()[0]].description, "Buy milk");
    }

    #[test]
    fn search_narrows_as_typed_and_clamps_selection() {
        let (_dir, mut app) = setup(
            vec![make_todo(1, "aaa", false), make_todo(2, "bbb", false)],
            vec![],
        );
        app.selected_index = 1;

        app.start_search();
        app.search_buffer_push('b');
        assert_eq!(app.visible_count(), 1);
        assert_eq!(app.selected_index, 0);

        app.search_buffer_pop();
        assert_eq!(app.visible_count(), 2);
        assert!(app.search_filter.is_none());
    }

    #[test]
    fn cancel_search_clears_filter() {
        let (_dir, mut app) = setup(vec![make_todo(1, "aaa", false)], vec![]);
        app.start_search();
        app.search_buffer_push('z');
        assert_eq!(app.visible_count(), 0);

        app.cancel_search();
        assert!(app.search_filter.is_none());
        assert!(app.search_buffer.is_empty());
        assert_eq!(app.visible_count(), 1);
    }

    #[test]
    fn visible_entries_start_with_group_header() {
        let (_dir, app) = setup(
            vec![make_todo(1, "a", false), make_todo(2, "b", false)],
            vec![],
        );
        let entries = app.visible_entries();
        // Both todos created "now": one header followed by both todos.
        assert_eq!(entries.len(), 3);
        assert!(matches!(entries[0], VisibleEntry::GroupHeader(_)));
        assert!(matches!(entries[1], VisibleEntry::Todo(1)));
        assert!(matches!(entries[2], VisibleEntry::Todo(0)));
    }

    #[test]
    fn date_label_variants() {
        let today = NaiveDate::from_ymd_opt(2025, 6, 15).unwrap(); // Sunday, W24
        assert_eq!(date_label(today, today), "Today (W24)");
        assert_eq!(
            date_label(NaiveDate::from_ymd_opt(2025, 6, 14).unwrap(), today),
            "Yesterday"
        );
        assert_eq!(
            date_label(NaiveDate::from_ymd_opt(2025, 6, 10).unwrap(), today),
            "Tuesday (W24)"
        );
        assert_eq!(
            date_label(NaiveDate::from_ymd_opt(2025, 6, 1).unwrap(), today),
            "June 01"
        );
        assert_eq!(
            date_label(NaiveDate::from_ymd_opt(2024, 12, 25).unwrap(), today),
            "December 25, 2024"
        );
    }

    // --- todo mutations ---

    #[test]
    fn push_todo_persists_and_selects_top() {
        let (_dir, mut app) = setup(vec![], vec![]);
        app.push_todo("new task".to_string());

        assert_eq!(app.todos.len(), 1);
        assert_eq!(app.selected_index, 0);
        assert_eq!(storage::load(&app.storage_path).len(), 1);
    }

    #[test]
    fn toggle_done_sets_and_clears_completed_at() {
        let (_dir, mut app) = setup(vec![make_todo(1, "task", false)], vec![]);

        app.toggle_done();
        assert!(app.todos[0].done);
        assert!(app.todos[0].completed_at.is_some());

        app.toggle_done();
        assert!(!app.todos[0].done);
        assert!(app.todos[0].completed_at.is_none());
    }

    #[test]
    fn archive_selected_marks_done_and_hides() {
        let (_dir, mut app) = setup(vec![make_todo(1, "task", false)], vec![]);

        app.archive_selected();
        assert!(app.todos[0].archived);
        assert!(app.todos[0].done);
        assert_eq!(app.visible_count(), 0);

        let saved = storage::load(&app.storage_path);
        assert!(saved[0].archived);
    }

    #[test]
    fn creating_flow_trims_and_rejects_empty() {
        let (_dir, mut app) = setup(vec![], vec![]);

        app.start_creating();
        assert!(matches!(app.input_mode, InputMode::Creating));
        for c in "  hi  ".chars() {
            app.create_type_char(c);
        }
        app.confirm_creating();
        assert_eq!(app.todos.len(), 1);
        assert_eq!(app.todos[0].description, "hi");

        app.start_creating();
        app.create_type_char(' ');
        app.confirm_creating();
        assert_eq!(app.todos.len(), 1, "whitespace-only input is discarded");
    }

    #[test]
    fn editing_flow_updates_description() {
        let (_dir, mut app) = setup(vec![make_todo(1, "old", false)], vec![]);

        app.start_editing();
        assert_eq!(app.edit_buffer, "old");
        app.edit_backspace();
        app.edit_backspace();
        app.edit_backspace();
        for c in "new".chars() {
            app.edit_type_char(c);
        }
        app.confirm_editing();

        assert_eq!(app.todos[0].description, "new");
        assert!(matches!(app.input_mode, InputMode::Normal));
    }

    #[test]
    fn cancel_editing_keeps_original() {
        let (_dir, mut app) = setup(vec![make_todo(1, "old", false)], vec![]);
        app.start_editing();
        app.edit_type_char('x');
        app.cancel_editing();
        assert_eq!(app.todos[0].description, "old");
    }

    #[test]
    fn confirm_delete_todo_enables_undo() {
        let (_dir, mut app) = setup(vec![make_todo(1, "task", false)], vec![]);

        app.start_deletion();
        assert_eq!(app.confirm_selection, 0, "defaults to No");
        app.confirm_delete();
        assert_eq!(app.todos.len(), 1, "confirming No keeps the todo");

        app.start_deletion();
        app.confirm_move_right();
        app.confirm_delete();
        assert!(app.todos.is_empty());
        assert!(app.undo_state.is_active());

        app.undo_delete();
        assert_eq!(app.todos.len(), 1);
        assert!(!app.undo_state.is_active());
        assert_eq!(storage::load(&app.storage_path).len(), 1);
    }

    #[test]
    fn archive_old_archives_week_old_todos() {
        let mut old = make_todo(1, "stale", false);
        old.created_at = Local::now().naive_local() - Duration::days(8);
        let (_dir, mut app) = setup(vec![old, make_todo(2, "fresh", false)], vec![]);

        app.archive_old();

        assert!(app.todos[0].archived);
        assert!(app.todos[0].done);
        assert!(!app.todos[1].archived);
    }

    // --- note deletion ---

    #[test]
    fn delete_open_note_selects_neighbor() {
        let (_dir, mut app) = setup(
            vec![],
            vec![
                make_note(1, "one"),
                make_note(2, "two"),
                make_note(3, "three"),
            ],
        );
        app.view = View::Note;
        app.current_note_index = Some(1);

        app.start_deletion();
        app.confirm_move_right();
        app.confirm_delete();

        assert_eq!(app.notes.len(), 2);
        assert_eq!(app.current_note_index, Some(1));
        assert_eq!(app.current_note().unwrap().title, "three");
    }

    #[test]
    fn delete_last_open_note_falls_back_to_notes_view() {
        let (_dir, mut app) = setup(vec![], vec![make_note(1, "only")]);
        app.view = View::Note;
        app.current_note_index = Some(0);

        app.start_deletion();
        app.confirm_move_right();
        app.confirm_delete();

        assert!(app.notes.is_empty());
        assert_eq!(app.current_note_index, None);
        assert_eq!(app.view, View::Notes);
        assert!(storage::load_notes(&app.notes_path).is_empty());
    }

    #[test]
    fn delete_note_from_list_clamps_selection() {
        let (_dir, mut app) = setup(vec![], vec![make_note(1, "one"), make_note(2, "two")]);
        app.view = View::Notes;
        app.selected_index = 1;

        app.start_deletion();
        app.confirm_move_right();
        app.confirm_delete();

        assert_eq!(app.notes.len(), 1);
        assert_eq!(app.selected_index, 0);
    }

    // --- note editor ---

    fn note_app(content: &str) -> (TempDir, App) {
        let mut note = make_note(1, "note");
        note.content = content.to_string();
        let (dir, mut app) = setup(vec![], vec![note]);
        app.view = View::Note;
        app.current_note_index = Some(0);
        app.start_edit_note();
        (dir, app)
    }

    #[test]
    fn note_insert_and_save_updates_title() {
        let (_dir, mut app) = note_app("");
        for c in "# Hello".chars() {
            app.note_cursor_insert(c);
        }
        app.note_cursor_enter();
        for c in "body".chars() {
            app.note_cursor_insert(c);
        }
        app.save_current_note();

        let note = app.current_note().unwrap();
        assert_eq!(note.content, "# Hello\nbody");
        assert_eq!(note.title, "Hello");
        assert_eq!(storage::load_notes(&app.notes_path)[0].title, "Hello");
    }

    #[test]
    fn note_cursor_handles_multibyte_chars() {
        let (_dir, mut app) = note_app("héllo");
        app.note_cursor_end();
        assert_eq!(app.note_cursor_col, 5, "columns count chars, not bytes");

        app.note_cursor_home();
        app.note_cursor_right();
        app.note_cursor_right();
        app.note_cursor_insert('x'); // typing after the multibyte char must not panic
        assert_eq!(app.note_lines[0], "héxllo");

        app.note_cursor_backspace();
        app.note_cursor_backspace();
        assert_eq!(app.note_lines[0], "hllo");
    }

    #[test]
    fn note_backspace_at_line_start_joins_lines() {
        let (_dir, mut app) = note_app("abc\ndef");
        app.note_cursor_down();
        app.note_cursor_home();
        app.note_cursor_backspace();

        assert_eq!(app.note_lines, vec!["abcdef"]);
        assert_eq!(app.note_cursor_line, 0);
        assert_eq!(app.note_cursor_col, 3);
    }

    #[test]
    fn note_delete_at_line_end_joins_lines() {
        let (_dir, mut app) = note_app("abc\ndef");
        app.note_cursor_end();
        app.note_cursor_delete();
        assert_eq!(app.note_lines, vec!["abcdef"]);
    }

    #[test]
    fn note_enter_splits_line_at_cursor() {
        let (_dir, mut app) = note_app("abcdef");
        app.note_cursor_right();
        app.note_cursor_right();
        app.note_cursor_right();
        app.note_cursor_enter();

        assert_eq!(app.note_lines, vec!["abc", "def"]);
        assert_eq!(app.note_cursor_line, 1);
        assert_eq!(app.note_cursor_col, 0);
    }

    #[test]
    fn note_cursor_moves_across_line_boundaries() {
        let (_dir, mut app) = note_app("ab\ncd");
        app.note_cursor_end();
        app.note_cursor_right();
        assert_eq!((app.note_cursor_line, app.note_cursor_col), (1, 0));

        app.note_cursor_left();
        assert_eq!((app.note_cursor_line, app.note_cursor_col), (0, 2));
    }

    #[test]
    fn note_cursor_vertical_clamps_column() {
        let (_dir, mut app) = note_app("long line\nab");
        app.note_cursor_end();
        app.note_cursor_down();
        assert_eq!((app.note_cursor_line, app.note_cursor_col), (1, 2));

        app.note_cursor_up();
        assert_eq!((app.note_cursor_line, app.note_cursor_col), (0, 2));
    }

    #[test]
    fn start_creating_note_opens_editor() {
        let (_dir, mut app) = setup(vec![], vec![]);
        app.start_creating_note();

        assert_eq!(app.notes.len(), 1);
        assert_eq!(app.current_note_index, Some(0));
        assert_eq!(app.view, View::Note);
        assert!(matches!(app.note_mode, NoteMode::Editing));
        assert_eq!(app.note_lines, vec![String::new()]);
    }

    // --- sidebar & misc ---

    #[test]
    fn sidebar_navigation_and_selection() {
        let (_dir, mut app) = setup(vec![make_todo(1, "t", false)], vec![]);
        app.panel = Panel::Sidebar;

        app.side_down();
        app.select_sidebar();
        assert!(app.show_archived);
        assert_eq!(app.panel, Panel::Main);

        app.panel = Panel::Sidebar;
        app.side_down();
        app.select_sidebar();
        assert_eq!(app.view, View::Notes);

        app.panel = Panel::Sidebar;
        app.side_down();
        assert_eq!(app.side_index, 2, "cannot move past the last item");
    }

    #[test]
    fn byte_index_maps_char_columns() {
        assert_eq!(byte_index("héllo", 0), 0);
        assert_eq!(byte_index("héllo", 1), 1);
        assert_eq!(byte_index("héllo", 2), 3);
        assert_eq!(byte_index("héllo", 99), 6);
        assert_eq!(byte_index("", 0), 0);
    }
}
