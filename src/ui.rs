use ratatui::{
    layout::{Alignment, Constraint, Layout, Rect},
    style::{Color, Style, Stylize},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, List, ListItem, Paragraph, Wrap},
    Frame,
};

use crate::app::{
    App, FolderChoice, InputMode, NoteEntry, NoteMode, Panel, SideItem, View, VisibleEntry,
};
use crate::markdown;

pub fn draw(frame: &mut Frame, app: &mut App) {
    let area = frame.area();
    if area.width == 0 || area.height == 0 {
        return;
    }
    let vertical = Layout::vertical([
        Constraint::Length(3),
        Constraint::Min(1),
        Constraint::Length(3),
    ]);
    let [title_area, content_area, footer_area] = vertical.areas(area);

    let horizontal = Layout::horizontal([Constraint::Ratio(3, 4), Constraint::Ratio(1, 4)]);
    let [main_area, side_area] = horizontal.areas(content_area);

    render_title(frame, title_area, app);
    render_sidebar(frame, side_area, app);
    match (&app.view, &app.note_mode) {
        (View::Note, NoteMode::Editing) => render_note_editor(frame, main_area, app),
        (View::Note, NoteMode::Viewing) => render_note_view(frame, main_area, app),
        (View::Notes, _) => render_note_list(frame, main_area, app),
        _ => render_list(frame, main_area, app),
    }
    render_footer(frame, footer_area, app);

    if matches!(app.input_mode, InputMode::Search) {
        render_search_bar(frame, area, app);
    }

    if matches!(app.input_mode, InputMode::ConfirmDelete) {
        render_confirm_delete(frame, area, app);
    }

    if matches!(app.input_mode, InputMode::MoveToFolder) {
        render_move_picker(frame, area, app);
    }

    if app.undo_state.is_active() && matches!(app.input_mode, InputMode::Normal) {
        render_undo_toast(frame, area);
    }
}

fn render_title(frame: &mut Frame, area: Rect, app: &App) {
    let label = match (&app.view, &app.note_mode) {
        (View::Note, NoteMode::Editing) => "Editing",
        (View::Note, _) => "Note",
        (View::Notes, _) => "Notes",
        (View::Todos, _) if app.show_archived => "Archived",
        _ => "Todos",
    };
    let total = app
        .todos
        .iter()
        .filter(|t| t.archived == app.show_archived)
        .count();
    let done = app
        .todos
        .iter()
        .filter(|t| t.archived == app.show_archived && t.done)
        .count();
    let status = if total == 0 {
        "empty".to_string()
    } else {
        format!("{} / {} done", done, total)
    };

    let filter_display = app
        .search_filter
        .as_ref()
        .map(|q| format!("  /{}/", q))
        .unwrap_or_default();

    let title = Paragraph::new(Line::from(vec![
        Span::styled(" ✅", Style::default().fg(Color::Green).bold()),
        Span::styled(" nosh", Style::default().fg(Color::White).bold()),
        Span::styled(format!("  [{}]", label), Style::default().fg(Color::Cyan)),
        Span::styled(filter_display, Style::default().fg(Color::Magenta).bold()),
    ]))
    .block(
        Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::Cyan))
            .title(Span::styled(
                status,
                Style::default().fg(Color::Yellow).bold(),
            ))
            .title_alignment(Alignment::Right),
    );
    frame.render_widget(title, area);
}

fn render_sidebar(frame: &mut Frame, area: Rect, app: &App) {
    let items = app.side_items();
    let list_items: Vec<ListItem> = items
        .iter()
        .enumerate()
        .map(|(i, item)| {
            let is_focused = app.panel == Panel::Sidebar;
            let is_selected = is_focused && i == app.side_index;
            let is_active = !is_focused && item.is_active(&app.view, app.show_archived);
            let cursor = if is_selected { "▸ " } else { "  " };
            let style = if is_selected {
                Style::default().bg(Color::Rgb(35, 40, 48)).bold()
            } else if is_active {
                Style::default().fg(Color::Cyan)
            } else {
                Style::default()
            };

            let (icon, label, count) = match item {
                SideItem::Todos(c) => ("📋", "Todos", *c),
                SideItem::Archive(c) => ("📦", "Archive", *c),
                SideItem::Notes(c) => ("📝", "Notes", *c),
            };

            let line = Line::from(vec![
                Span::styled(cursor, Style::default().fg(Color::Cyan)),
                Span::styled(format!("{} {} ({})", icon, label, count), style),
            ]);
            ListItem::new(line)
        })
        .collect();

    let border_color = if app.panel == Panel::Sidebar {
        Color::Yellow
    } else {
        Color::Cyan
    };

    let list = List::new(list_items).block(
        Block::default()
            .borders(Borders::ALL)
            .title(Span::styled(" Explore ", Style::default().fg(Color::White)))
            .border_style(Style::default().fg(border_color)),
    );

    frame.render_widget(list, area);
}

fn render_list(frame: &mut Frame, area: Rect, app: &mut App) {
    let visible = app.visible_indices();
    let entries = app.visible_entries();
    let creating = matches!(app.input_mode, InputMode::Creating);
    let editing = matches!(app.input_mode, InputMode::Editing);

    let border_color = if app.panel == Panel::Main {
        Color::Yellow
    } else {
        Color::Cyan
    };

    if visible.is_empty() && !creating {
        render_list_empty(frame, area, border_color, app);
        return;
    }

    if creating {
        app.list_scroll = 0;
    }

    let sel = if creating {
        0
    } else {
        app.selected_index.min(visible.len().saturating_sub(1))
    };

    let mut selected_visual_pos: usize = 0;
    {
        let mut todo_count = 0;
        for (pos, entry) in entries.iter().enumerate() {
            if matches!(entry, VisibleEntry::Todo(_)) {
                if todo_count == app.selected_index {
                    selected_visual_pos = pos;
                    break;
                }
                todo_count += 1;
            }
        }
    }

    let total_entries = entries.len();
    let list_height = area.height.saturating_sub(2) as usize;
    let max_scroll = total_entries.saturating_sub(list_height);

    if selected_visual_pos < app.list_scroll {
        app.list_scroll = selected_visual_pos;
    } else if selected_visual_pos >= app.list_scroll + list_height {
        app.list_scroll = selected_visual_pos
            .saturating_sub(list_height)
            .saturating_add(1);
    }
    app.list_scroll = app.list_scroll.min(max_scroll);

    let max_entries = if creating {
        list_height.saturating_sub(1)
    } else {
        list_height
    };

    let mut items: Vec<ListItem> = Vec::new();

    if creating {
        let desc_text = if app.create_buffer.is_empty() {
            Span::styled(
                app.create_placeholder.clone(),
                Style::default().fg(Color::Rgb(100, 100, 100)),
            )
        } else {
            Span::styled(app.create_buffer.clone(), Style::default().fg(Color::White))
        };
        let line = Line::from(vec![
            Span::styled("▸ ", Style::default().fg(Color::Green).bold()),
            Span::styled("○ ", Style::default().fg(Color::Yellow)),
            desc_text,
            Span::styled("▎", Style::default().fg(Color::Yellow)),
        ]);
        items.push(ListItem::new(line).style(Style::default().bg(Color::Rgb(35, 40, 48))));
    }

    let mut flat_pos: usize = 0;
    for (_visual_pos, entry) in entries
        .iter()
        .enumerate()
        .skip(app.list_scroll)
        .take(max_entries)
    {
        match entry {
            VisibleEntry::GroupHeader(label) => {
                items.push(ListItem::new(Line::from(vec![Span::styled(
                    label.clone(),
                    Style::default().fg(Color::Rgb(140, 140, 140)).bold(),
                )])));
            }
            VisibleEntry::Todo(todo_idx) => {
                let todo = &app.todos[*todo_idx];
                let is_selected = app.panel == Panel::Main && !creating && flat_pos == sel;
                let is_being_edited = editing && Some(*todo_idx) == app.edit_todo_index;

                let checkbox = if todo.done { "✓" } else { "○" };
                let check_color = if todo.done {
                    Color::Green
                } else {
                    Color::Yellow
                };
                let prefix = if is_selected { "▸" } else { " " };
                let date = todo
                    .completed_at
                    .unwrap_or(todo.created_at)
                    .format("%m-%d %H:%M")
                    .to_string();

                let desc_style = if todo.done {
                    Style::default().fg(Color::DarkGray).crossed_out()
                } else {
                    Style::default().fg(Color::White)
                };

                if is_being_edited {
                    let line = Line::from(vec![
                        Span::styled(
                            format!("{} ", prefix),
                            Style::default().fg(Color::Cyan).bold(),
                        ),
                        Span::styled(format!("{} ", checkbox), Style::default().fg(check_color)),
                        Span::styled(app.edit_buffer.clone(), Style::default().fg(Color::White)),
                        Span::styled("▎", Style::default().fg(Color::Yellow)),
                    ]);
                    let item_style = Style::default().bg(Color::Rgb(35, 40, 48));
                    items.push(ListItem::new(line).style(item_style));
                } else {
                    let line = Line::from(vec![
                        Span::styled(
                            format!("{} ", prefix),
                            Style::default().fg(Color::Cyan).bold(),
                        ),
                        Span::styled(format!("{} ", checkbox), Style::default().fg(check_color)),
                        Span::styled(todo.description.clone(), desc_style),
                        Span::styled(format!("  {}", date), Style::default().fg(Color::DarkGray)),
                    ]);

                    let item_style = if is_selected {
                        Style::default().bg(Color::Rgb(35, 40, 48))
                    } else {
                        Style::default()
                    };
                    items.push(ListItem::new(line).style(item_style));
                }

                flat_pos += 1;
            }
        }
    }

    let list_title = if app.show_archived {
        " Archived "
    } else {
        " Todos "
    };

    let list = List::new(items).block(
        Block::default()
            .borders(Borders::ALL)
            .title(Span::styled(list_title, Style::default().fg(Color::White)))
            .border_style(Style::default().fg(border_color)),
    );

    frame.render_widget(list, area);
}

fn render_list_empty(frame: &mut Frame, area: Rect, border_color: Color, app: &App) {
    let list_title = if app.show_archived {
        " Archived "
    } else {
        " Todos "
    };
    let block = Block::default()
        .borders(Borders::ALL)
        .title(Span::styled(list_title, Style::default().fg(Color::White)))
        .border_style(Style::default().fg(border_color));
    frame.render_widget(block.clone(), area);

    let inner = block.inner(area);
    let vertical = Layout::vertical([
        Constraint::Percentage(50),
        Constraint::Length(6),
        Constraint::Percentage(50),
    ]);
    let [_, content_area, _] = vertical.areas(inner);

    let message = if app.show_archived {
        "No archived todos".to_string()
    } else {
        "No todos yet".to_string()
    };

    let sub = if app.show_archived {
        ""
    } else {
        "Press 'c' to add your first one"
    };

    let mut lines = vec![
        Line::from(vec![Span::styled(
            "○",
            Style::default().fg(Color::Yellow).bold(),
        )]),
        Line::from(""),
        Line::from(vec![Span::styled(
            message,
            Style::default().fg(Color::White).bold(),
        )]),
    ];
    if !sub.is_empty() {
        lines.push(Line::from(vec![Span::styled(
            sub,
            Style::default().fg(Color::DarkGray),
        )]));
    }

    let paragraph = Paragraph::new(lines).alignment(Alignment::Center);
    frame.render_widget(paragraph, content_area);
}

fn render_note_list(frame: &mut Frame, area: Rect, app: &mut App) {
    let border_color = if app.panel == Panel::Main {
        Color::Yellow
    } else {
        Color::Cyan
    };

    let list_height = area.height.saturating_sub(2) as usize;

    if app.notes.is_empty() {
        let block = Block::default()
            .borders(Borders::ALL)
            .title(" Notes ")
            .border_style(Style::default().fg(border_color));
        frame.render_widget(block.clone(), area);
        let inner = block.inner(area);
        let vertical = Layout::vertical([
            Constraint::Percentage(50),
            Constraint::Length(4),
            Constraint::Percentage(50),
        ]);
        let [_, content_area, _] = vertical.areas(inner);
        let paragraph = Paragraph::new(vec![
            Line::from(""),
            Line::from(vec![Span::styled(
                "No notes yet",
                Style::default().fg(Color::White).bold(),
            )]),
            Line::from(vec![Span::styled(
                "Press 'c' to create one",
                Style::default().fg(Color::DarkGray),
            )]),
        ])
        .alignment(Alignment::Center);
        frame.render_widget(paragraph, content_area);
        return;
    }

    let count = app.notes.len();
    let entries = app.note_entries();
    let grouped = entries
        .iter()
        .any(|e| matches!(e, NoteEntry::FolderHeader { .. }));
    let total_entries = entries.len();

    // Keep the selected note visible, measuring in entry rows (headers count).
    let sel = app.selected_index.min(count.saturating_sub(1));
    let mut selected_visual_pos = 0;
    {
        let mut note_pos = 0;
        for (pos, entry) in entries.iter().enumerate() {
            if let NoteEntry::Note(_) = entry {
                if note_pos == sel {
                    selected_visual_pos = pos;
                    break;
                }
                note_pos += 1;
            }
        }
    }

    let max_scroll = total_entries.saturating_sub(list_height);
    if selected_visual_pos < app.list_scroll {
        app.list_scroll = selected_visual_pos;
    } else if selected_visual_pos >= app.list_scroll + list_height {
        app.list_scroll = selected_visual_pos
            .saturating_sub(list_height)
            .saturating_add(1);
    }
    app.list_scroll = app.list_scroll.min(max_scroll);

    let selected_real = if app.panel == Panel::Main {
        app.selected_note_index()
    } else {
        None
    };

    let items: Vec<ListItem> = entries
        .iter()
        .skip(app.list_scroll)
        .take(list_height)
        .map(|entry| match entry {
            NoteEntry::FolderHeader { label, count } => ListItem::new(Line::from(vec![
                Span::styled(
                    format!("▾ {}", label),
                    Style::default().fg(Color::Rgb(140, 140, 140)).bold(),
                ),
                Span::styled(
                    format!("  ({})", count),
                    Style::default().fg(Color::Rgb(90, 90, 90)),
                ),
            ])),
            NoteEntry::Note(i) => {
                let note = &app.notes[*i];
                let is_selected = Some(*i) == selected_real;
                let prefix = if is_selected { "▸" } else { " " };
                let indent = if grouped { "  " } else { "" };
                let title = if note.title.is_empty() {
                    "Untitled"
                } else {
                    &note.title
                };
                let preview = preview_note(&note.content);
                let date = note.created_at.format("%m-%d %H:%M").to_string();

                let line = Line::from(vec![
                    Span::styled(
                        format!("{}{} ", indent, prefix),
                        Style::default().fg(Color::Cyan).bold(),
                    ),
                    Span::styled(format!("📝 {}", title), Style::default().fg(Color::White)),
                    Span::styled(
                        format!("  {}", preview),
                        Style::default().fg(Color::DarkGray),
                    ),
                    Span::styled(format!("  {}", date), Style::default().fg(Color::DarkGray)),
                ]);

                let item_style = if is_selected {
                    Style::default().bg(Color::Rgb(35, 40, 48))
                } else {
                    Style::default()
                };
                ListItem::new(line).style(item_style)
            }
        })
        .collect();

    let list = List::new(items).block(
        Block::default()
            .borders(Borders::ALL)
            .title(Span::styled(
                format!(" Notes ({}) ", count),
                Style::default().fg(Color::White),
            ))
            .border_style(Style::default().fg(border_color)),
    );

    frame.render_widget(list, area);
}

fn render_note_view(frame: &mut Frame, area: Rect, app: &mut App) {
    let border_color = if app.panel == Panel::Main {
        Color::Yellow
    } else {
        Color::Cyan
    };

    let note = match app.current_note() {
        Some(n) => n,
        None => {
            let block = Block::default()
                .borders(Borders::ALL)
                .title(" Note ")
                .border_style(Style::default().fg(border_color));
            frame.render_widget(block, area);
            app.note_view_max_scroll = 0;
            app.note_view_page_size = 0;
            return;
        }
    };

    let title = if note.title.is_empty() {
        "Note"
    } else {
        &note.title
    };
    let title_text = match &note.folder {
        Some(folder) => format!(" {} / {} ", folder, title),
        None => format!(" {} ", title),
    };
    let block = Block::default()
        .borders(Borders::ALL)
        .title(title_text)
        .border_style(Style::default().fg(border_color));
    frame.render_widget(block.clone(), area);

    let inner = block.inner(area);
    let max_lines = inner.height as usize;

    let lines = markdown::render(&note.content, inner.width);
    let total_lines = lines.len();
    let has_overflow = total_lines > max_lines;
    let visible_count = if has_overflow {
        max_lines.saturating_sub(1).max(1)
    } else {
        max_lines
    };
    let max_scroll = total_lines.saturating_sub(visible_count);
    app.note_view_max_scroll = max_scroll;
    app.note_view_page_size = visible_count;
    app.note_scroll = app.note_scroll.min(max_scroll);
    let scroll = app.note_scroll;

    let visible: Vec<&Line> = lines.iter().skip(scroll).take(visible_count).collect();

    let content = if visible.is_empty() {
        vec![
            Line::from(""),
            Line::from(vec![Span::styled(
                "Press 'i' to start editing",
                Style::default().fg(Color::DarkGray),
            )]),
        ]
    } else {
        visible.into_iter().cloned().collect()
    };

    let paragraph = Paragraph::new(content).wrap(Wrap { trim: false });
    frame.render_widget(paragraph, inner);

    if has_overflow {
        let pct = if max_scroll > 0 {
            (scroll as f64 / max_scroll as f64 * 100.0).min(100.0)
        } else {
            100.0
        };
        let scroll_info = format!(" {}% ", pct as u32);
        let scroll_line = Line::from(vec![Span::styled(
            scroll_info,
            Style::default().fg(Color::DarkGray),
        )]);
        frame.render_widget(
            Paragraph::new(scroll_line).alignment(Alignment::Right),
            Rect {
                y: inner.y + inner.height.saturating_sub(1),
                height: 1,
                ..inner
            },
        );
    }
}

fn render_note_editor(frame: &mut Frame, area: Rect, app: &mut App) {
    let border_color = Color::Yellow;

    let (title, folder) = app
        .current_note()
        .map(|n| (n.title.clone(), n.folder.clone()))
        .unwrap_or_default();
    let display_title = if title.is_empty() { "Editing" } else { &title };
    let title_text = match &folder {
        Some(folder) => format!(" {} / {} ", folder, display_title),
        None => format!(" {} ", display_title),
    };

    let block = Block::default()
        .borders(Borders::ALL)
        .title(title_text)
        .border_style(Style::default().fg(border_color));
    frame.render_widget(block.clone(), area);

    let inner = block.inner(area);
    let visible_height = inner.height as usize;
    // Keep the cursor line visible, above the status bar on the last row.
    let follow_height = visible_height.saturating_sub(1).max(1);
    if app.note_cursor_line < app.note_scroll {
        app.note_scroll = app.note_cursor_line;
    } else if app.note_cursor_line >= app.note_scroll + follow_height {
        app.note_scroll = app.note_cursor_line + 1 - follow_height;
    }
    let max_scroll = app.note_lines.len().saturating_sub(visible_height);
    app.note_scroll = app.note_scroll.min(max_scroll);
    let scroll = app.note_scroll;

    let visible: Vec<Line> = app
        .note_lines
        .iter()
        .skip(scroll)
        .take(visible_height)
        .enumerate()
        .map(|(i, line)| {
            let abs_line = i + scroll;
            let is_cursor_line = abs_line == app.note_cursor_line;
            let numbered = format!("{:>3} │ ", abs_line + 1);

            if is_cursor_line {
                let cursor_byte = crate::app::byte_index(line, app.note_cursor_col);
                let before = &line[..cursor_byte];
                let rest = &line[cursor_byte..];
                let cursor_char = rest.chars().next();
                let at = cursor_char.map_or_else(|| " ".to_string(), |c| c.to_string());
                let after = &rest[cursor_char.map_or(0, char::len_utf8)..];
                Line::from(vec![
                    Span::styled(numbered, Style::default().fg(Color::DarkGray)),
                    Span::raw(before),
                    Span::styled(
                        at,
                        Style::default().bg(Color::Rgb(80, 80, 80)).fg(Color::White),
                    ),
                    Span::raw(after),
                ])
            } else {
                Line::from(vec![
                    Span::styled(numbered, Style::default().fg(Color::DarkGray)),
                    Span::raw(line.clone()),
                ])
            }
        })
        .collect();

    let paragraph = Paragraph::new(visible);
    frame.render_widget(paragraph, inner);

    let status_line = format!(
        " INSERT  Ln {}, Col {}  [Esc to save] ",
        app.note_cursor_line + 1,
        app.note_cursor_col + 1
    );
    let status_rect = Rect {
        y: inner.y + inner.height.saturating_sub(1),
        height: 1,
        ..inner
    };
    frame.render_widget(
        Paragraph::new(Line::from(vec![Span::styled(
            status_line,
            Style::default().bg(Color::Rgb(60, 60, 60)).fg(Color::White),
        )])),
        status_rect,
    );
}

fn render_search_bar(frame: &mut Frame, area: Rect, app: &App) {
    let popup = Rect::new(
        area.x + 2,
        area.y + area.height.saturating_sub(4),
        area.width.saturating_sub(4),
        3,
    );
    frame.render_widget(Clear, popup);

    let block = Block::default()
        .borders(Borders::ALL)
        .title(Span::styled(
            " Search ",
            Style::default().fg(Color::Cyan).bold(),
        ))
        .border_style(Style::default().fg(Color::Yellow));
    frame.render_widget(block.clone(), popup);

    let inner = block.inner(popup);
    let input_text = Line::from(vec![
        Span::styled("/", Style::default().fg(Color::Magenta).bold()),
        Span::raw(&app.search_buffer),
        Span::styled("▎", Style::default().fg(Color::Yellow)),
    ]);
    frame.render_widget(
        Paragraph::new(if app.search_buffer.is_empty() {
            vec![Line::from(vec![
                Span::styled("/", Style::default().fg(Color::Magenta).bold()),
                Span::styled("type to filter", Style::default().fg(Color::DarkGray)),
            ])]
        } else {
            vec![input_text]
        }),
        inner,
    );
}

fn render_confirm_delete(frame: &mut Frame, area: Rect, app: &App) {
    let label = app.deletion_target_label();
    let truncated: String = if label.chars().count() > 28 {
        let mut s = label.chars().take(25).collect::<String>();
        s.push_str("...");
        s
    } else {
        label
    };

    let msg_text = format!("Delete \"{}\"?", truncated);
    let buttons_text = "▶ Yes    No";
    let hint_text = "Enter confirm  ·  Esc cancel";
    let content_width = msg_text.len().max(buttons_text.len()).max(hint_text.len()) as u16;
    let width = (content_width + 8).min(area.width.saturating_sub(4)).max(1);
    let height = 7_u16;
    let x = area.x + (area.width.saturating_sub(width)) / 2;
    let y = area.y + (area.height.saturating_sub(height)) / 2;
    let popup = Rect::new(x, y, width, height);
    frame.render_widget(Clear, popup);

    let block = Block::default()
        .borders(Borders::ALL)
        .title(Span::styled(
            " Confirm Delete ",
            Style::default().fg(Color::Red).bold(),
        ))
        .border_style(Style::default().fg(Color::Red));
    frame.render_widget(block.clone(), popup);

    let inner = block.inner(popup);
    let layout = Layout::vertical([
        Constraint::Fill(1),
        Constraint::Length(1),
        Constraint::Fill(1),
        Constraint::Length(1),
        Constraint::Fill(1),
        Constraint::Length(1),
        Constraint::Fill(1),
    ]);
    let [_top, msg_area, _gap1, buttons_area, _gap2, hint_area, _bottom] = layout.areas(inner);

    let msg = Paragraph::new(vec![Line::from(vec![Span::styled(
        msg_text,
        Style::default().fg(Color::White),
    )])])
    .alignment(Alignment::Center);
    frame.render_widget(msg, msg_area);

    let yes_style = if app.confirm_selection == 1 {
        Style::default()
            .bg(Color::Rgb(180, 40, 40))
            .fg(Color::White)
            .bold()
    } else {
        Style::default().fg(Color::Red)
    };
    let no_style = if app.confirm_selection == 0 {
        Style::default()
            .bg(Color::Rgb(60, 60, 60))
            .fg(Color::White)
            .bold()
    } else {
        Style::default().fg(Color::DarkGray)
    };

    let yes_arrow = if app.confirm_selection == 1 {
        "▶ "
    } else {
        "  "
    };
    let no_arrow = if app.confirm_selection == 0 {
        "▶ "
    } else {
        "  "
    };

    let buttons = Paragraph::new(Line::from(vec![
        Span::styled(format!("{}Yes", yes_arrow), yes_style),
        Span::raw("    "),
        Span::styled(format!("{}No", no_arrow), no_style),
    ]))
    .alignment(Alignment::Center);
    frame.render_widget(buttons, buttons_area);

    let hint = Paragraph::new(Line::from(vec![
        Span::styled("Enter", Style::default().fg(Color::Green).bold()),
        Span::raw(" confirm  ·  "),
        Span::styled("Esc", Style::default().fg(Color::Red).bold()),
        Span::raw(" cancel"),
    ]))
    .alignment(Alignment::Center);
    frame.render_widget(hint, hint_area);
}

fn render_move_picker(frame: &mut Frame, area: Rect, app: &App) {
    let choice_label = |choice: &FolderChoice| match choice {
        FolderChoice::Existing(name) => name.clone(),
        FolderChoice::Unfiled => "— No folder —".to_string(),
        FolderChoice::New => "+ New folder…".to_string(),
    };

    let content: Vec<Line> = if let Some(buf) = &app.new_folder_buffer {
        vec![Line::from(vec![
            Span::styled("Name: ", Style::default().fg(Color::Cyan)),
            Span::raw(buf.clone()),
            Span::styled("▎", Style::default().fg(Color::Yellow)),
        ])]
    } else {
        app.folder_choices
            .iter()
            .enumerate()
            .map(|(i, choice)| {
                let selected = i == app.folder_choice_index;
                let prefix = if selected { "▸ " } else { "  " };
                let style = if selected {
                    Style::default().fg(Color::White).bold()
                } else {
                    match choice {
                        FolderChoice::New => Style::default().fg(Color::Green),
                        FolderChoice::Unfiled => Style::default().fg(Color::DarkGray),
                        FolderChoice::Existing(_) => Style::default().fg(Color::White),
                    }
                };
                let line = Line::from(vec![
                    Span::styled(prefix, Style::default().fg(Color::Cyan).bold()),
                    Span::styled(choice_label(choice), style),
                ]);
                if selected {
                    line.style(Style::default().bg(Color::Rgb(35, 40, 48)))
                } else {
                    line
                }
            })
            .collect()
    };

    let hint = if app.new_folder_buffer.is_some() {
        "Enter create  ·  Esc back"
    } else {
        "↑/↓ choose  ·  Enter move  ·  Esc cancel"
    };

    let longest = app
        .folder_choices
        .iter()
        .map(|c| choice_label(c).chars().count())
        .max()
        .unwrap_or(0)
        .max(app.new_folder_buffer.as_ref().map_or(0, |b| b.chars().count() + 6))
        .max(hint.chars().count());
    let width = ((longest as u16) + 6).clamp(24, area.width.saturating_sub(4));

    let content_h = content.len() as u16;
    // borders (2) + content + gap (1) + hint (1)
    let height = (content_h + 4).min(area.height.saturating_sub(2)).max(5);
    let x = area.x + area.width.saturating_sub(width) / 2;
    let y = area.y + area.height.saturating_sub(height) / 2;
    let popup = Rect::new(x, y, width, height);
    frame.render_widget(Clear, popup);

    let block = Block::default()
        .borders(Borders::ALL)
        .title(Span::styled(
            " Move to folder ",
            Style::default().fg(Color::Cyan).bold(),
        ))
        .border_style(Style::default().fg(Color::Yellow));
    frame.render_widget(block.clone(), popup);

    let inner = block.inner(popup);
    let [content_area, _gap, hint_area] = Layout::vertical([
        Constraint::Min(1),
        Constraint::Length(1),
        Constraint::Length(1),
    ])
    .areas(inner);

    frame.render_widget(Paragraph::new(content), content_area);
    frame.render_widget(
        Paragraph::new(Line::from(vec![Span::styled(
            hint,
            Style::default().fg(Color::DarkGray),
        )]))
        .alignment(Alignment::Center),
        hint_area,
    );
}

fn render_undo_toast(frame: &mut Frame, area: Rect) {
    let toast_y = area.y + area.height.saturating_sub(6);
    let toast_rect = Rect::new(area.x + area.width / 5, toast_y, 3 * area.width / 5, 3);
    frame.render_widget(Clear, toast_rect);

    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Yellow));
    frame.render_widget(block.clone(), toast_rect);

    let inner = block.inner(toast_rect);
    let text = Paragraph::new(Line::from(vec![
        Span::styled(" Deleted. ", Style::default().fg(Color::Red).bold()),
        Span::styled("u", Style::default().fg(Color::Yellow).bold()),
        Span::raw(" to undo, any other key to dismiss"),
    ]))
    .alignment(Alignment::Center);
    frame.render_widget(text, inner);
}

fn render_footer(frame: &mut Frame, area: Rect, app: &App) {
    let (text, style) = match (&app.input_mode, &app.note_mode) {
        (InputMode::Normal, NoteMode::Editing) => (
            vec![
                Span::styled("  Esc ", Style::default().fg(Color::Red).bold()),
                Span::raw("save  "),
                Span::styled(
                    "\u{2191}/\u{2193}\u{2190}/\u{2192} ",
                    Style::default().fg(Color::Cyan).bold(),
                ),
                Span::raw("move  "),
                Span::styled("Enter ", Style::default().fg(Color::Green).bold()),
                Span::raw("newline  "),
                Span::styled("Backspace ", Style::default().fg(Color::Red).bold()),
                Span::raw("delete  "),
                Span::styled("Tab ", Style::default().fg(Color::DarkGray).bold()),
                Span::raw("save+panel"),
            ],
            Style::default(),
        ),
        (InputMode::Normal, _) if app.view == View::Notes => (
            vec![
                Span::styled("  q ", Style::default().fg(Color::Red).bold()),
                Span::raw("quit  "),
                Span::styled("t ", Style::default().fg(Color::Cyan).bold()),
                Span::raw("todos  "),
                Span::styled("c ", Style::default().fg(Color::Green).bold()),
                Span::raw("create  "),
                Span::styled("Enter ", Style::default().fg(Color::Green).bold()),
                Span::raw("open  "),
                Span::styled("m ", Style::default().fg(Color::Yellow).bold()),
                Span::raw("move  "),
                Span::styled("d ", Style::default().fg(Color::Red).bold()),
                Span::raw("delete  "),
                Span::styled(
                    "\u{2191}/\u{2193} ",
                    Style::default().fg(Color::Cyan).bold(),
                ),
                Span::raw("nav  "),
                Span::styled("Tab ", Style::default().fg(Color::DarkGray).bold()),
                Span::raw("panel"),
            ],
            Style::default(),
        ),
        (InputMode::Normal, _) if app.view == View::Note => (
            vec![
                Span::styled("  q ", Style::default().fg(Color::Red).bold()),
                Span::raw("quit  "),
                Span::styled("i ", Style::default().fg(Color::Blue).bold()),
                Span::raw("edit  "),
                Span::styled("m ", Style::default().fg(Color::Yellow).bold()),
                Span::raw("move  "),
                Span::styled("d ", Style::default().fg(Color::Red).bold()),
                Span::raw("delete  "),
                Span::styled("n ", Style::default().fg(Color::Cyan).bold()),
                Span::raw("notes  "),
                Span::styled(
                    "\u{2191}/\u{2193} ",
                    Style::default().fg(Color::Cyan).bold(),
                ),
                Span::raw("scroll  "),
                Span::styled("Tab ", Style::default().fg(Color::DarkGray).bold()),
                Span::raw("panel"),
            ],
            Style::default(),
        ),
        (InputMode::Normal, _) => (
            vec![
                Span::styled("  q ", Style::default().fg(Color::Red).bold()),
                Span::raw("quit  "),
                Span::styled("t ", Style::default().fg(Color::Cyan).bold()),
                Span::raw("todos  "),
                Span::styled("a ", Style::default().fg(Color::Magenta).bold()),
                Span::raw("archived  "),
                Span::styled("n ", Style::default().fg(Color::Cyan).bold()),
                Span::raw("notes  "),
                Span::styled("c ", Style::default().fg(Color::Green).bold()),
                Span::raw("create  "),
                Span::styled("e ", Style::default().fg(Color::Blue).bold()),
                Span::raw("edit  "),
                Span::styled("d ", Style::default().fg(Color::Red).bold()),
                Span::raw("delete  "),
                Span::styled("/ s", Style::default().fg(Color::Magenta).bold()),
                Span::raw(" search  "),
                Span::styled("space ", Style::default().fg(Color::Yellow).bold()),
                Span::raw("toggle  "),
                Span::styled(
                    "\u{2191}/\u{2193} ",
                    Style::default().fg(Color::Cyan).bold(),
                ),
                Span::raw("nav  "),
                Span::styled("Tab ", Style::default().fg(Color::DarkGray).bold()),
                Span::raw("panel"),
            ],
            Style::default(),
        ),
        (InputMode::Search, _) => (
            vec![
                Span::styled("  Search ", Style::default().fg(Color::Magenta).bold()),
                Span::styled(
                    "type to filter  Esc clear",
                    Style::default().fg(Color::DarkGray),
                ),
            ],
            Style::default(),
        ),
        (InputMode::Creating, _) => (
            vec![
                Span::styled("  Creating ", Style::default().fg(Color::Green).bold()),
                Span::styled(
                    "Enter:save  Esc:cancel",
                    Style::default().fg(Color::DarkGray),
                ),
            ],
            Style::default(),
        ),
        (InputMode::ConfirmDelete, _) => (
            vec![
                Span::styled("  Confirm ", Style::default().fg(Color::Red).bold()),
                Span::styled(
                    "←/→ choose  Enter confirm  Esc cancel",
                    Style::default().fg(Color::DarkGray),
                ),
            ],
            Style::default(),
        ),
        (InputMode::Editing, _) => (
            vec![
                Span::styled("  Editing ", Style::default().fg(Color::Blue).bold()),
                Span::styled(
                    "Enter:save  Esc:cancel",
                    Style::default().fg(Color::DarkGray),
                ),
            ],
            Style::default(),
        ),
        (InputMode::MoveToFolder, _) => (
            vec![
                Span::styled("  Move ", Style::default().fg(Color::Yellow).bold()),
                Span::styled(
                    "pick a folder for this note",
                    Style::default().fg(Color::DarkGray),
                ),
            ],
            Style::default(),
        ),
    };

    let footer = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Cyan));
    let inner = footer.inner(area);
    frame.render_widget(footer, area);
    frame.render_widget(Paragraph::new(Line::from(text)).style(style), inner);
}

fn preview_note(content: &str) -> String {
    content
        .lines()
        .find(|l| !l.trim().is_empty())
        .unwrap_or("")
        .chars()
        .take(60)
        .collect::<String>()
        .trim()
        .to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn preview_note_first_non_empty_line() {
        assert_eq!(preview_note("\n\n  hello world  \nmore"), "hello world");
        assert_eq!(preview_note(""), "");
        assert_eq!(preview_note("\n  \n"), "");
        let long = format!("{}\nnext", "x".repeat(100));
        assert_eq!(preview_note(&long).chars().count(), 60);
    }
}
