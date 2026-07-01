use ratatui::{
    layout::{Alignment, Constraint, Layout, Rect},
    style::{Color, Style, Stylize},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, Paragraph},
    Frame,
};

use crate::app::{App, InputMode};
use crate::storage::Todo;

pub fn draw(frame: &mut Frame, app: &mut App) {
    let area = frame.area();
    let vertical = Layout::vertical([
        Constraint::Length(3),
        Constraint::Min(1),
        Constraint::Length(3),
    ]);
    let [title_area, list_area, footer_area] = vertical.areas(area);

    render_title(frame, title_area, app);
    render_list(frame, list_area, app);
    render_footer(frame, footer_area, app);
}

fn render_title(frame: &mut Frame, area: Rect, app: &App) {
    let label = if app.show_archived { "Archived" } else { "Active" };
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

    if visible.is_empty() {
        render_empty_state(frame, area, app);
        return;
    }

    let sel = app.selected_index.min(visible.len().saturating_sub(1));
    let items: Vec<ListItem> = visible
        .iter()
        .enumerate()
        .map(|(i, todo)| {
            let is_selected = i == sel;
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
            .border_style(Style::default().fg(Color::Cyan)),
    );

    frame.render_widget(list, area);
}

fn render_empty_state(frame: &mut Frame, area: Rect, app: &App) {
    let list_title = if app.show_archived {
        " Archived "
    } else {
        " Todos "
    };
    let block = Block::default()
        .borders(Borders::ALL)
        .title(Span::styled(list_title, Style::default().fg(Color::White)))
        .border_style(Style::default().fg(Color::Cyan));
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

fn render_footer(frame: &mut Frame, area: Rect, app: &App) {
    let (text, style) = match app.input_mode {
        InputMode::Normal => (
            vec![
                Span::styled("  q ", Style::default().fg(Color::Red).bold()),
                Span::raw("quit  "),
                Span::styled("n ", Style::default().fg(Color::Green).bold()),
                Span::raw("new  "),
                Span::styled("e ", Style::default().fg(Color::Green).bold()),
                Span::raw("edit  "),
                Span::styled("a ", Style::default().fg(Color::Magenta).bold()),
                Span::raw("archive  "),
                Span::styled("d ", Style::default().fg(Color::Red).bold()),
                Span::raw("delete  "),
                Span::styled("space ", Style::default().fg(Color::Yellow).bold()),
                Span::raw("toggle  "),
                Span::styled("/ ", Style::default().fg(Color::Cyan).bold()),
                Span::raw("search  "),
                Span::styled("\u{2191}/\u{2193} ", Style::default().fg(Color::Cyan).bold()),
                Span::raw("navigate  "),
                Span::styled("Tab ", Style::default().fg(Color::DarkGray).bold()),
                Span::raw("view"),
            ],
            Style::default(),
        ),
        InputMode::Adding | InputMode::Editing | InputMode::Searching => {
            let label = match app.input_mode {
                InputMode::Editing => " EDIT: ",
                InputMode::Searching => " FIND: ",
                _ => " INPUT: ",
            };
            (
                vec![
                    Span::styled(label, Style::default().fg(Color::Green).bold()),
                    Span::raw(&app.input_buffer),
                    Span::styled(" | ", Style::default().fg(Color::DarkGray)),
                    Span::styled("Enter ", Style::default().fg(Color::Green).bold()),
                    Span::raw("save  "),
                    Span::styled("Esc ", Style::default().fg(Color::Red).bold()),
                    Span::raw("cancel"),
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
