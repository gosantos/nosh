use ratatui::{
    layout::{Alignment, Constraint, Layout, Rect},
    style::{Color, Style, Stylize},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, Paragraph},
    Frame,
};

use crate::app::{App, InputMode, NoteMode, Panel, SideItem, View};
use crate::markdown;
use crate::storage::Todo;

pub fn draw(frame: &mut Frame, app: &mut App) {
    let area = frame.area();
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
        _ => render_list(frame, main_area, app),
    }
    render_footer(frame, footer_area, app);
}

fn render_title(frame: &mut Frame, area: Rect, app: &App) {
    let label = match (&app.view, &app.note_mode) {
        (View::Note, NoteMode::Editing) => "Editing",
        (View::Note, _) => "Note",
        (View::Todos, _) if app.show_archived => "Archived",
        _ => "Todos",
    };
    let total = app.todos.iter().filter(|t| t.archived == app.show_archived).count();
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

    let title = Paragraph::new(Line::from(vec![
        Span::styled(" ✅", Style::default().fg(Color::Green).bold()),
        Span::styled(" tui-todo", Style::default().fg(Color::White).bold()),
        Span::styled(format!("  [{}]", label), Style::default().fg(Color::Cyan)),
    ]))
    .block(
        Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::Cyan))
            .title(Span::styled(status, Style::default().fg(Color::Yellow).bold()))
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
            let is_active = match item {
                SideItem::Active(a) if !is_focused => *a,
                SideItem::Archive(a) if !is_focused => *a,
                _ => false,
            };
            let cursor = if is_selected { "▸ " } else { "  " };
            let style = if is_selected {
                Style::default().bg(Color::Rgb(35, 40, 48)).bold()
            } else if is_active {
                Style::default().fg(Color::Cyan)
            } else {
                Style::default()
            };

            let line = match item {
                SideItem::Active(_) => Line::from(vec![
                    Span::styled(cursor, Style::default().fg(Color::Cyan)),
                    Span::styled("📋 Todos", style),
                ]),
                SideItem::Archive(_) => Line::from(vec![
                    Span::styled(cursor, Style::default().fg(Color::Cyan)),
                    Span::styled("📦 Archive", style),
                ]),
                SideItem::NotesHeader(count) => {
                    let label = format!("  Notes ({}) ", count);
                    Line::from(vec![Span::styled(
                        label,
                        Style::default().fg(Color::Cyan).bold(),
                    )])
                }
                SideItem::Note(_, title) => Line::from(vec![
                    Span::styled(cursor, Style::default().fg(Color::Cyan)),
                    Span::styled(format!("📝 {}", title), style),
                ]),
            };
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

fn render_list(frame: &mut Frame, area: Rect, app: &App) {
    let q = app.search_query.to_lowercase();
    let visible: Vec<&Todo> = app
        .todos
        .iter()
        .filter(|t| {
            app.show_archived == t.archived
                && (q.is_empty() || t.description.to_lowercase().contains(&q))
        })
        .collect();

    let border_color = if app.panel == Panel::Main {
        Color::Yellow
    } else {
        Color::Cyan
    };

    if visible.is_empty() {
        render_list_empty(frame, area, border_color, app);
        return;
    }

    let sel = app.selected_index.min(visible.len().saturating_sub(1));
    let items: Vec<ListItem> = visible
        .iter()
        .enumerate()
        .map(|(i, todo)| {
            let is_selected = app.panel == Panel::Main && i == sel;
            let checkbox = if todo.done { "✓" } else { "○" };
            let check_color = if todo.done { Color::Green } else { Color::Yellow };
            let text_style = if todo.done {
                Style::default().fg(Color::DarkGray).crossed_out()
            } else {
                Style::default().fg(Color::White)
            };
            let prefix = if is_selected { "▸" } else { " " };
            let date = todo.created_at.format("%m-%d %H:%M").to_string();

            let line = Line::from(vec![
                Span::styled(format!("{} ", prefix), Style::default().fg(Color::Cyan).bold()),
                Span::styled(format!("{} ", checkbox), Style::default().fg(check_color)),
                Span::styled(todo.description.clone(), text_style),
                Span::styled(format!("  {}", date), Style::default().fg(Color::DarkGray)),
            ]);

            let item_style = if is_selected {
                Style::default().bg(Color::Rgb(35, 40, 48))
            } else {
                Style::default()
            };

            ListItem::new(line).style(item_style)
        })
        .collect();

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

    let message = if !app.search_query.is_empty() {
        format!("No matches for '{}'", app.search_query)
    } else if app.show_archived {
        "No archived todos".to_string()
    } else {
        "No todos yet".to_string()
    };

    let sub = if !app.search_query.is_empty() {
        "Press Esc to clear"
    } else if app.show_archived {
        ""
    } else {
        "Press 'n' to add your first one"
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

fn render_note_view(frame: &mut Frame, area: Rect, app: &App) {
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
            return;
        }
    };

    let title = if note.title.is_empty() { "Note" } else { &note.title };
    let block = Block::default()
        .borders(Borders::ALL)
        .title(format!(" {} ", title))
        .border_style(Style::default().fg(border_color));
    frame.render_widget(block.clone(), area);

    let inner = block.inner(area);
    let max_lines = inner.height as usize;

    let lines = markdown::render(&note.content, inner.width);
    let total_lines = lines.len();

    let visible: Vec<&Line> = lines
        .iter()
        .skip(app.note_scroll)
        .take(max_lines)
        .collect();

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

    let paragraph = Paragraph::new(content);
    frame.render_widget(paragraph, inner);

    if total_lines > max_lines {
        let pct = (app.note_scroll as f64 / (total_lines - max_lines) as f64 * 100.0).min(100.0);
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

fn render_note_editor(frame: &mut Frame, area: Rect, app: &App) {
    let border_color = Color::Yellow;

    let title = app.current_note().map(|n| n.title.clone()).unwrap_or_default();
    let display_title = if title.is_empty() { "Editing" } else { &title };

    let block = Block::default()
        .borders(Borders::ALL)
        .title(format!(" {} ", display_title))
        .border_style(Style::default().fg(border_color));
    frame.render_widget(block.clone(), area);

    let inner = block.inner(area);
    let visible_height = inner.height as usize;
    let max_scroll = app.note_lines.len().saturating_sub(visible_height);
    let scroll = app.note_scroll.min(max_scroll);

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
                let before = &line[..app.note_cursor_col.min(line.len())];
                let at = line
                    .chars()
                    .nth(app.note_cursor_col)
                    .map(|c| c.to_string())
                    .unwrap_or_else(|| " ".to_string());
                let after = if app.note_cursor_col < line.len() {
                    &line[(app.note_cursor_col + 1).min(line.len())..]
                } else {
                    ""
                };
                Line::from(vec![
                    Span::styled(numbered, Style::default().fg(Color::DarkGray)),
                    Span::raw(before),
                    Span::styled(at, Style::default().bg(Color::Rgb(80, 80, 80)).fg(Color::White)),
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

fn render_footer(frame: &mut Frame, area: Rect, app: &App) {
    let (text, style) = match (&app.input_mode, &app.note_mode) {
        (InputMode::Normal, NoteMode::Editing) => (
            vec![
                Span::styled("  Esc ", Style::default().fg(Color::Red).bold()),
                Span::raw("save  "),
                Span::styled("\u{2191}/\u{2193}\u{2190}/\u{2192} ", Style::default().fg(Color::Cyan).bold()),
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
        (InputMode::Normal, _) => {
            let focused = match (&app.panel, &app.view) {
                (Panel::Sidebar, _) => Span::styled(
                    "  focused: sidebar ",
                    Style::default().fg(Color::Yellow),
                ),
                (Panel::Main, View::Note) => Span::styled(
                    "  focused: note ",
                    Style::default().fg(Color::Yellow),
                ),
                _ => Span::styled(
                    "  focused: todos ",
                    Style::default().fg(Color::Yellow),
                ),
            };
            (
                vec![
                    Span::styled("  q ", Style::default().fg(Color::Red).bold()),
                    Span::raw("quit  "),
                    Span::styled("t ", Style::default().fg(Color::Cyan).bold()),
                    Span::raw("todos  "),
                    Span::styled("a ", Style::default().fg(Color::Magenta).bold()),
                    Span::raw("archived  "),
                    Span::styled("n ", Style::default().fg(Color::Green).bold()),
                    Span::raw("note  "),
                    Span::styled("c ", Style::default().fg(Color::Green).bold()),
                    Span::raw("create  "),
                    Span::styled("e ", Style::default().fg(Color::Green).bold()),
                    Span::raw("edit  "),
                    Span::styled("d ", Style::default().fg(Color::Red).bold()),
                    Span::raw("delete  "),
                    Span::styled("space ", Style::default().fg(Color::Yellow).bold()),
                    Span::raw("toggle  "),
                    Span::styled("/ ", Style::default().fg(Color::Cyan).bold()),
                    Span::raw("search  "),
                    Span::styled("\u{2191}/\u{2193} ", Style::default().fg(Color::Cyan).bold()),
                    Span::raw("nav  "),
                    Span::styled("Tab ", Style::default().fg(Color::DarkGray).bold()),
                    Span::raw("panel"),
                    focused,
                ],
                Style::default(),
            )
        }
        _ => {
            let label = match app.input_mode {
                InputMode::Editing => " EDIT: ",
                InputMode::Searching => " FIND: ",
                _ => " CREATE: ",
            };
            let hint = match app.input_mode {
                InputMode::Adding => "Enter create  Esc save+quit",
                InputMode::Editing => "Enter/Esc save",
                _ => "Enter save  Esc cancel",
            };
            (
                vec![
                    Span::styled(label, Style::default().fg(Color::Green).bold()),
                    Span::raw(&app.input_buffer),
                    Span::styled(format!(" | {}", hint), Style::default().fg(Color::DarkGray)),
                ],
                Style::default(),
            )
        }
    };

    let footer = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Cyan));
    let inner = footer.inner(area);
    frame.render_widget(footer, area);
    frame.render_widget(Paragraph::new(Line::from(text)).style(style), inner);
}
