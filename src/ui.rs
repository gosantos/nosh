use ratatui::{
    layout::{Alignment, Constraint, Layout, Rect},
    style::{Color, Style, Stylize},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, Paragraph},
    Frame,
};

use crate::app::{App, InputMode};

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
    let total = app.todos.len();
    let done = app.todos.iter().filter(|t| t.done).count();
    let status = if total == 0 {
        "empty".to_string()
    } else {
        format!("{} / {} done", done, total)
    };

    let title = Paragraph::new(Line::from(vec![
        Span::styled(" ✅", Style::default().fg(Color::Green).bold()),
        Span::styled(" tui-todo", Style::default().fg(Color::White).bold()),
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
    if app.todos.is_empty() {
        render_empty_state(frame, area);
        return;
    }

    let items: Vec<ListItem> = app
        .todos
        .iter()
        .enumerate()
        .map(|(i, todo)| {
            let is_selected = i == app.selected_index;
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
                Span::styled(
                    format!("{} ", prefix),
                    Style::default().fg(Color::Cyan).bold(),
                ),
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

    let list = List::new(items).block(
        Block::default()
            .borders(Borders::ALL)
            .title(Span::styled(" Todos ", Style::default().fg(Color::White)))
            .border_style(Style::default().fg(Color::Cyan)),
    );

    frame.render_widget(list, area);
}

fn render_empty_state(frame: &mut Frame, area: Rect) {
    let block = Block::default()
        .borders(Borders::ALL)
        .title(Span::styled(" Todos ", Style::default().fg(Color::White)))
        .border_style(Style::default().fg(Color::Cyan));
    frame.render_widget(block.clone(), area);

    let inner = block.inner(area);
    let vertical = Layout::vertical([
        Constraint::Percentage(50),
        Constraint::Length(6),
        Constraint::Percentage(50),
    ]);
    let [_, content_area, _] = vertical.areas(inner);

    let text = vec![
        Line::from(vec![Span::styled(
            "○",
            Style::default().fg(Color::Yellow).bold(),
        )]),
        Line::from(""),
        Line::from(vec![Span::styled(
            "No todos yet",
            Style::default().fg(Color::White).bold(),
        )]),
        Line::from(vec![Span::styled(
            "Press 'n' to add your first one",
            Style::default().fg(Color::DarkGray),
        )]),
    ];

    let paragraph = Paragraph::new(text).alignment(Alignment::Center);
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
                Span::styled("d ", Style::default().fg(Color::Red).bold()),
                Span::raw("delete  "),
                Span::styled("space ", Style::default().fg(Color::Yellow).bold()),
                Span::raw("toggle  "),
                Span::styled("\u{2191}/\u{2193} ", Style::default().fg(Color::Cyan).bold()),
                Span::raw("navigate"),
            ],
            Style::default(),
        ),
        InputMode::Adding | InputMode::Editing => {
            let label = match app.input_mode {
                InputMode::Editing => " EDIT: ",
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
