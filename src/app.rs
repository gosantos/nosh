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
    MoveToFolder,
}

/// A row in the move-to-folder picker.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FolderChoice {
    /// Assign the note to an existing folder.
    Existing(String),
    /// Clear the note's folder.
    Unfiled,
    /// Prompt for a brand-new folder name.
    New,
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
    folders_path: PathBuf,
    /// Durable folder names. May be empty even when notes reference folders
    /// (legacy data); `folder_names` reconciles the two.
    pub folders: Vec<String>,
    pub search_filter: Option<String>,
    pub search_buffer: String,
    pub undo_state: UndoState,
    pub create_buffer: String,
    pub create_placeholder: String,
    placeholder_idx: usize,
    pub edit_buffer: String,
    pub edit_todo_index: Option<usize>,
    pub confirm_selection: usize,
    /// The note being moved while the folder picker is open.
    pub move_note_index: Option<usize>,
    pub folder_choices: Vec<FolderChoice>,
    pub folder_choice_index: usize,
    /// `Some` while typing a new folder name in the picker.
    pub new_folder_buffer: Option<String>,
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

/// Label shown for the group holding notes with no folder.
pub const UNFILED_LABEL: &str = "No folder";

/// A row in the notes list: either a folder header or a note (index into
/// `App::notes`). Headers only appear once at least one folder exists.
#[derive(Clone)]
pub enum NoteEntry {
    FolderHeader { label: String, count: usize },
    Note(usize),
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
        let folders_path = storage_path.with_file_name("folders.json");
        let folders = storage::load_folders(&folders_path);
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
            folders_path,
            folders,
            search_filter: None,
            search_buffer: String::new(),
            undo_state: UndoState::Inactive,
            create_buffer: String::new(),
            create_placeholder: String::new(),
            placeholder_idx: 0,
            edit_buffer: String::new(),
            edit_todo_index: None,
            confirm_selection: 0,
            move_note_index: None,
            folder_choices: Vec::new(),
            folder_choice_index: 0,
            new_folder_buffer: None,
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

    /// All folders to display: the durable set plus any folder still
    /// referenced by a note (covers legacy data), sorted case-insensitively.
    pub fn folder_names(&self) -> Vec<String> {
        let mut names: Vec<String> = self.folders.clone();
        for note in &self.notes {
            if let Some(folder) = &note.folder {
                if !names.contains(folder) {
                    names.push(folder.clone());
                }
            }
        }
        names.sort_by_key(|s| s.to_lowercase());
        names
    }

    /// Registers a durable folder if new, persisting the folder set.
    fn register_folder(&mut self, name: &str) {
        if !self.folders.iter().any(|f| f == name) {
            self.folders.push(name.to_string());
            storage::save_folders(&self.folders_path, &self.folders);
        }
    }

    /// The notes list as displayed: folder headers interleaved with notes.
    /// When no folders exist, this is a flat list of notes with no headers,
    /// matching the pre-folders layout.
    pub fn note_entries(&self) -> Vec<NoteEntry> {
        let names = self.folder_names();
        if names.is_empty() {
            return (0..self.notes.len()).map(NoteEntry::Note).collect();
        }

        let mut entries = Vec::new();
        for name in &names {
            let indices: Vec<usize> = self
                .notes
                .iter()
                .enumerate()
                .filter(|(_, n)| n.folder.as_deref() == Some(name.as_str()))
                .map(|(i, _)| i)
                .collect();
            entries.push(NoteEntry::FolderHeader {
                label: name.clone(),
                count: indices.len(),
            });
            entries.extend(indices.into_iter().map(NoteEntry::Note));
        }

        let unfiled: Vec<usize> = self
            .notes
            .iter()
            .enumerate()
            .filter(|(_, n)| n.folder.is_none())
            .map(|(i, _)| i)
            .collect();
        if !unfiled.is_empty() {
            entries.push(NoteEntry::FolderHeader {
                label: UNFILED_LABEL.to_string(),
                count: unfiled.len(),
            });
            entries.extend(unfiled.into_iter().map(NoteEntry::Note));
        }
        entries
    }

    /// Note indices in display order (folder grouping applied), excluding
    /// headers. `selected_index` indexes into this list in the Notes view.
    pub fn visible_note_indices(&self) -> Vec<usize> {
        self.note_entries()
            .into_iter()
            .filter_map(|e| match e {
                NoteEntry::Note(i) => Some(i),
                NoteEntry::FolderHeader { .. } => None,
            })
            .collect()
    }

    /// The `App::notes` index of the note under the cursor in the Notes view.
    pub fn selected_note_index(&self) -> Option<usize> {
        self.visible_note_indices()
            .get(self.selected_index)
            .copied()
    }

    pub fn note_list_up(&mut self) {
        if self.selected_index > 0 {
            self.selected_index -= 1;
        }
    }

    pub fn note_list_down(&mut self) {
        if self.selected_index + 1 < self.visible_note_indices().len() {
            self.selected_index += 1;
        }
    }

    /// Returns from an open note to the notes list, keeping that note under
    /// the cursor.
    pub fn back_to_notes_list(&mut self) {
        if let Some(cur) = self.current_note_index {
            if let Some(pos) = self.visible_note_indices().iter().position(|&i| i == cur) {
                self.selected_index = pos;
            }
        }
        self.view = View::Notes;
        self.panel = Panel::Main;
    }

    /// Opens the move-to-folder picker for the note under the cursor.
    pub fn start_move_note(&mut self) {
        if let Some(idx) = self.selected_note_index() {
            self.begin_move(idx);
        }
    }

    /// Opens the picker for the note currently open in the Note view.
    pub fn start_move_current_note(&mut self) {
        if let Some(idx) = self.current_note_index {
            self.begin_move(idx);
        }
    }

    fn begin_move(&mut self, note_idx: usize) {
        self.move_note_index = Some(note_idx);
        self.new_folder_buffer = None;
        self.folder_choice_index = 0;
        self.rebuild_move_choices();
        // Preselect the note's current folder, if any.
        if let Some(f) = self.notes[note_idx].folder.clone() {
            if let Some(pos) = self
                .folder_choices
                .iter()
                .position(|c| matches!(c, FolderChoice::Existing(n) if *n == f))
            {
                self.folder_choice_index = pos;
            }
        }
        self.input_mode = InputMode::MoveToFolder;
    }

    /// Rebuilds the picker rows: existing folders, then "No folder" (only when
    /// the note is currently filed), then "New folder…".
    fn rebuild_move_choices(&mut self) {
        let filed = self
            .move_note_index
            .and_then(|i| self.notes.get(i))
            .is_some_and(|n| n.folder.is_some());
        let mut choices: Vec<FolderChoice> = self
            .folder_names()
            .into_iter()
            .map(FolderChoice::Existing)
            .collect();
        if filed {
            choices.push(FolderChoice::Unfiled);
        }
        choices.push(FolderChoice::New);
        self.folder_choice_index = self.folder_choice_index.min(choices.len() - 1);
        self.folder_choices = choices;
    }

    /// Removes the highlighted folder if it holds no notes. Non-empty folders
    /// are left alone so notes are never orphaned.
    pub fn delete_selected_folder(&mut self) {
        let Some(FolderChoice::Existing(name)) =
            self.folder_choices.get(self.folder_choice_index).cloned()
        else {
            return;
        };
        let empty = !self
            .notes
            .iter()
            .any(|n| n.folder.as_deref() == Some(name.as_str()));
        if empty {
            self.folders.retain(|f| f != &name);
            storage::save_folders(&self.folders_path, &self.folders);
            self.rebuild_move_choices();
        }
    }

    pub fn move_picker_up(&mut self) {
        if self.folder_choice_index > 0 {
            self.folder_choice_index -= 1;
        }
    }

    pub fn move_picker_down(&mut self) {
        if self.folder_choice_index + 1 < self.folder_choices.len() {
            self.folder_choice_index += 1;
        }
    }

    pub fn move_picker_select(&mut self) {
        match self.folder_choices.get(self.folder_choice_index).cloned() {
            Some(FolderChoice::Existing(name)) => self.apply_move(Some(name)),
            Some(FolderChoice::Unfiled) => self.apply_move(None),
            Some(FolderChoice::New) => self.new_folder_buffer = Some(String::new()),
            None => {}
        }
    }

    pub fn move_new_char(&mut self, c: char) {
        if let Some(buf) = self.new_folder_buffer.as_mut() {
            buf.push(c);
        }
    }

    pub fn move_new_backspace(&mut self) {
        if let Some(buf) = self.new_folder_buffer.as_mut() {
            buf.pop();
        }
    }

    /// Commits a typed folder name. Empty input drops back to the choice list.
    pub fn move_new_confirm(&mut self) {
        if let Some(buf) = self.new_folder_buffer.take() {
            let name = buf.trim().to_string();
            if name.is_empty() {
                return;
            }
            self.apply_move(Some(name));
        }
    }

    /// Esc backs out of new-name entry first, then closes the picker.
    pub fn cancel_move(&mut self) {
        if self.new_folder_buffer.is_some() {
            self.new_folder_buffer = None;
        } else {
            self.close_move();
        }
    }

    fn apply_move(&mut self, folder: Option<String>) {
        if let Some(name) = &folder {
            self.register_folder(name);
        }
        if let Some(idx) = self.move_note_index {
            self.notes[idx].folder = folder;
            self.notes[idx].updated_at = Local::now().naive_local();
            storage::save_notes(&self.notes_path, &self.notes);
            if let Some(pos) = self.visible_note_indices().iter().position(|&i| i == idx) {
                self.selected_index = pos;
            }
        }
        self.close_move();
    }

    fn close_move(&mut self) {
        self.move_note_index = None;
        self.folder_choices.clear();
        self.folder_choice_index = 0;
        self.new_folder_buffer = None;
        self.input_mode = InputMode::Normal;
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
            folder: None,
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
                    .selected_note_index()
                    .and_then(|i| self.notes.get(i))
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
                        if let Some(idx) = self.selected_note_index() {
                            self.notes.remove(idx);
                            self.current_note_index = None;
                            storage::save_notes(&self.notes_path, &self.notes);
                            if self.notes.is_empty() {
                                self.view = View::Todos;
                                self.selected_index = 0;
                            } else {
                                let count = self.visible_note_indices().len();
                                if self.selected_index >= count {
                                    self.selected_index = count.saturating_sub(1);
                                }
                            }
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
        self.folders = storage::load_folders(&self.folders_path);

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
            folder: None,
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

    // --- note folders ---

    fn note_labels(app: &App) -> Vec<String> {
        app.note_entries()
            .into_iter()
            .map(|e| match e {
                NoteEntry::FolderHeader { label, count } => format!("#{label}({count})"),
                NoteEntry::Note(i) => app.notes[i].title.clone(),
            })
            .collect()
    }

    #[test]
    fn note_entries_flat_when_no_folders() {
        let (_dir, app) = setup(vec![], vec![make_note(1, "a"), make_note(2, "b")]);
        // No folder headers at all — identical to the pre-folders layout.
        assert_eq!(note_labels(&app), vec!["a", "b"]);
        assert_eq!(app.visible_note_indices(), vec![0, 1]);
    }

    #[test]
    fn note_entries_group_by_folder_alpha_unfiled_last() {
        let mut work = make_note(1, "standup");
        work.folder = Some("Work".to_string());
        let loose = make_note(2, "scratch");
        let mut personal = make_note(3, "groceries");
        personal.folder = Some("Personal".to_string());

        let (_dir, app) = setup(vec![], vec![work, loose, personal]);
        // Folders alphabetical (Personal before Work), unfiled group last.
        assert_eq!(
            note_labels(&app),
            vec![
                "#Personal(1)",
                "groceries",
                "#Work(1)",
                "standup",
                "#No folder(1)",
                "scratch",
            ]
        );
        // visible order follows the grouped display, not storage order.
        assert_eq!(app.visible_note_indices(), vec![2, 0, 1]);
    }

    #[test]
    fn selected_note_index_maps_through_grouping() {
        let mut work = make_note(1, "w");
        work.folder = Some("Work".to_string());
        let mut personal = make_note(2, "p");
        personal.folder = Some("Personal".to_string());
        let (_dir, mut app) = setup(vec![], vec![work, personal]);

        app.view = View::Notes;
        // Display order: Personal/p (idx 1), Work/w (idx 0).
        app.selected_index = 0;
        assert_eq!(app.selected_note_index(), Some(1));
        app.selected_index = 1;
        assert_eq!(app.selected_note_index(), Some(0));
    }

    #[test]
    fn move_note_to_new_folder_and_cursor_follows() {
        let (_dir, mut app) = setup(vec![], vec![make_note(1, "a"), make_note(2, "b")]);
        app.view = View::Notes;
        app.selected_index = 1; // note "b"

        app.start_move_note();
        assert!(matches!(app.input_mode, InputMode::MoveToFolder));
        // Only "+ New folder…" is offered when there are no folders yet and the
        // note is already unfiled.
        assert_eq!(app.folder_choices.len(), 1);
        assert_eq!(app.folder_choices[0], FolderChoice::New);

        app.move_picker_select(); // choose "New folder…"
        assert!(app.new_folder_buffer.is_some());
        for c in "Work".chars() {
            app.move_new_char(c);
        }
        app.move_new_confirm();

        assert!(matches!(app.input_mode, InputMode::Normal));
        assert_eq!(app.notes[1].folder.as_deref(), Some("Work"));
        // Cursor stays on "b" in its new group.
        assert_eq!(app.selected_note_index(), Some(1));
        assert_eq!(storage::load_notes(&app.notes_path)[1].folder.as_deref(), Some("Work"));
    }

    #[test]
    fn move_note_to_existing_folder_then_unfiled() {
        let mut filed = make_note(1, "a");
        filed.folder = Some("Work".to_string());
        let (_dir, mut app) = setup(vec![], vec![filed, make_note(2, "b")]);
        app.view = View::Notes;

        // Select unfiled "b" (display order: Work/a, then No folder/b).
        app.selected_index = 1;
        assert_eq!(app.selected_note_index(), Some(1));

        app.start_move_note();
        // Offers: existing "Work", then "+ New folder…" (note is unfiled → no
        // "No folder" choice).
        assert_eq!(app.folder_choices.len(), 2);
        assert_eq!(app.folder_choices[0], FolderChoice::Existing("Work".to_string()));
        app.move_picker_select(); // Work
        assert_eq!(app.notes[1].folder.as_deref(), Some("Work"));

        // Now move it back to unfiled.
        app.start_move_note();
        assert!(app.folder_choices.contains(&FolderChoice::Unfiled));
        let pos = app
            .folder_choices
            .iter()
            .position(|c| *c == FolderChoice::Unfiled)
            .unwrap();
        app.folder_choice_index = pos;
        app.move_picker_select();
        assert!(app.notes[1].folder.is_none());
    }

    #[test]
    fn cancel_move_backs_out_of_typing_then_closes() {
        let (_dir, mut app) = setup(vec![], vec![make_note(1, "a")]);
        app.view = View::Notes;
        app.start_move_note();
        app.move_picker_select(); // enter new-name typing
        assert!(app.new_folder_buffer.is_some());

        app.cancel_move(); // first Esc: leave typing, stay in picker
        assert!(app.new_folder_buffer.is_none());
        assert!(matches!(app.input_mode, InputMode::MoveToFolder));

        app.cancel_move(); // second Esc: close picker
        assert!(matches!(app.input_mode, InputMode::Normal));
        assert!(app.notes[0].folder.is_none());
    }

    #[test]
    fn empty_new_folder_name_is_ignored() {
        let (_dir, mut app) = setup(vec![], vec![make_note(1, "a")]);
        app.view = View::Notes;
        app.start_move_note();
        app.move_picker_select(); // typing
        app.move_new_char(' ');
        app.move_new_confirm(); // whitespace-only → no-op, back to list
        assert!(app.notes[0].folder.is_none());
        assert!(matches!(app.input_mode, InputMode::MoveToFolder));
    }

    #[test]
    fn move_creates_durable_folder_persisted_to_disk() {
        let (_dir, mut app) = setup(vec![], vec![make_note(1, "a")]);
        app.view = View::Notes;
        app.start_move_note();
        app.move_picker_select(); // New folder…
        for c in "Work".chars() {
            app.move_new_char(c);
        }
        app.move_new_confirm();

        assert!(app.folders.contains(&"Work".to_string()));
        let persisted = storage::load_folders(&app.storage_path.with_file_name("folders.json"));
        assert_eq!(persisted, vec!["Work"]);
    }

    // Reproduces the reported bug: moving the SAME note through several new
    // folder names used to "rename" the first folder because empty folders
    // vanished. With durable folders each one persists.
    #[test]
    fn moving_one_note_through_three_names_keeps_all_folders() {
        let (_dir, mut app) = setup(vec![], vec![make_note(1, "a")]);
        app.view = View::Notes;

        for name in ["Work", "Personal", "Ideas"] {
            app.start_move_note();
            // Jump to "New folder…" (always the last choice).
            app.folder_choice_index = app.folder_choices.len() - 1;
            app.move_picker_select();
            for c in name.chars() {
                app.move_new_char(c);
            }
            app.move_new_confirm();
        }

        let mut folders = app.folder_names();
        folders.sort();
        assert_eq!(folders, vec!["Ideas", "Personal", "Work"]);
        // The note ends up in the last folder it was moved to.
        assert_eq!(app.notes[0].folder.as_deref(), Some("Ideas"));
    }

    #[test]
    fn durable_empty_folder_survives_moving_last_note_out() {
        let (_dir, mut app) = setup(vec![], vec![make_note(1, "a")]);
        app.view = View::Notes;
        // File the note, creating a durable folder.
        app.start_move_note();
        app.move_picker_select();
        for c in "Work".chars() {
            app.move_new_char(c);
        }
        app.move_new_confirm();
        assert_eq!(app.notes[0].folder.as_deref(), Some("Work"));

        // Move it back to unfiled; the empty folder remains.
        app.start_move_note();
        let pos = app
            .folder_choices
            .iter()
            .position(|c| *c == FolderChoice::Unfiled)
            .unwrap();
        app.folder_choice_index = pos;
        app.move_picker_select();

        assert!(app.notes[0].folder.is_none());
        assert!(app.folder_names().contains(&"Work".to_string()));
    }

    #[test]
    fn durable_empty_folder_shows_as_entry_and_choice() {
        let (_dir, mut app) = setup(vec![], vec![make_note(1, "a")]);
        app.folders = vec!["Work".to_string()]; // durable but empty
        app.view = View::Notes;

        // Empty folder header (count 0), then the unfiled note.
        assert_eq!(
            note_labels(&app),
            vec!["#Work(0)", "#No folder(1)", "a"]
        );

        // And it is offered as a move target.
        app.start_move_note();
        assert!(app
            .folder_choices
            .contains(&FolderChoice::Existing("Work".to_string())));
    }

    #[test]
    fn delete_empty_folder_from_picker() {
        let (_dir, mut app) = setup(vec![], vec![make_note(1, "a")]);
        app.folders = vec!["Work".to_string()];
        storage::save_folders(&app.storage_path.with_file_name("folders.json"), &app.folders);
        app.view = View::Notes;

        app.start_move_note();
        let pos = app
            .folder_choices
            .iter()
            .position(|c| *c == FolderChoice::Existing("Work".to_string()))
            .unwrap();
        app.folder_choice_index = pos;
        app.delete_selected_folder();

        assert!(!app.folder_names().contains(&"Work".to_string()));
        // Choice list rebuilt without it.
        assert!(!app
            .folder_choices
            .contains(&FolderChoice::Existing("Work".to_string())));
        let persisted = storage::load_folders(&app.storage_path.with_file_name("folders.json"));
        assert!(persisted.is_empty());
    }

    #[test]
    fn delete_nonempty_folder_is_ignored() {
        let mut filed = make_note(1, "a");
        filed.folder = Some("Work".to_string());
        let (_dir, mut app) = setup(vec![], vec![filed, make_note(2, "b")]);
        app.folders = vec!["Work".to_string()];
        app.view = View::Notes;
        app.selected_index = app
            .visible_note_indices()
            .iter()
            .position(|&i| i == 1) // unfiled "b", so its folder isn't Work
            .unwrap();

        app.start_move_note();
        let pos = app
            .folder_choices
            .iter()
            .position(|c| *c == FolderChoice::Existing("Work".to_string()))
            .unwrap();
        app.folder_choice_index = pos;
        app.delete_selected_folder();

        // Work still holds note "a", so it is not removed.
        assert!(app.folder_names().contains(&"Work".to_string()));
    }

    #[test]
    fn back_to_notes_list_highlights_the_open_note() {
        let mut work = make_note(1, "w");
        work.folder = Some("Work".to_string());
        let personal = make_note(2, "p");
        let (_dir, mut app) = setup(vec![], vec![work, personal]);
        app.view = View::Note;
        app.current_note_index = Some(0); // the "Work" note

        app.back_to_notes_list();

        assert_eq!(app.view, View::Notes);
        assert_eq!(app.selected_note_index(), Some(0));
    }

    #[test]
    fn deleting_last_note_in_folder_removes_the_folder() {
        let mut a = make_note(1, "a");
        a.folder = Some("Work".to_string());
        let (_dir, mut app) = setup(vec![], vec![a, make_note(2, "b")]);
        app.view = View::Notes;
        app.selected_index = 0; // Work/a

        app.start_deletion();
        app.confirm_move_right();
        app.confirm_delete();

        assert_eq!(app.notes.len(), 1);
        assert!(app.folder_names().is_empty(), "empty folder disappears");
        assert_eq!(note_labels(&app), vec!["b"]);
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
