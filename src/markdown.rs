use pulldown_cmark::{Event, HeadingLevel, Parser, Tag, TagEnd};
use ratatui::{
    style::{Color, Style, Stylize},
    text::{Line, Span},
};

pub fn render(content: &str, width: u16) -> Vec<Line<'_>> {
    let parser = Parser::new(content);
    let mut lines: Vec<Line> = Vec::new();
    let mut current: Vec<Span> = Vec::new();
    let mut style = Style::default().fg(Color::White);
    let mut in_heading = false;
    let mut in_code_block = false;

    for event in parser {
        match event {
            Event::Start(Tag::Heading { level, .. }) => {
                in_heading = true;
                let prefix = match level {
                    HeadingLevel::H1 | HeadingLevel::H2 => "# ",
                    HeadingLevel::H3 => "## ",
                    _ => "### ",
                };
                style = match level {
                    HeadingLevel::H1 | HeadingLevel::H2 => {
                        Style::default().fg(Color::Cyan).bold()
                    }
                    _ => Style::default().fg(Color::Cyan),
                };
                current.push(Span::styled(prefix, style));
            }
            Event::End(TagEnd::Heading(_)) => {
                in_heading = false;
                lines.push(Line::from(current.drain(..).collect::<Vec<_>>()));
                lines.push(Line::from(""));
                style = Style::default().fg(Color::White);
            }

            Event::Start(Tag::Paragraph) => {}
            Event::End(TagEnd::Paragraph) => {
                if !in_heading && !in_code_block {
                    if !current.is_empty() {
                        lines.push(Line::from(current.drain(..).collect::<Vec<_>>()));
                    }
                    lines.push(Line::from(""));
                }
            }

            Event::Start(Tag::CodeBlock(_)) => {
                in_code_block = true;
                style = Style::default().fg(Color::Green);
            }
            Event::End(TagEnd::CodeBlock) => {
                in_code_block = false;
                if !current.is_empty() {
                    lines.push(Line::from(current.drain(..).collect::<Vec<_>>()));
                }
                lines.push(Line::from(""));
                style = Style::default().fg(Color::White);
            }

            Event::Start(Tag::List(_)) => {
                lines.push(Line::from(""));
            }
            Event::End(TagEnd::List(_)) => {
                lines.push(Line::from(""));
            }

            Event::Start(Tag::Item) => {
                current.push(Span::styled("  • ", Style::default().fg(Color::Yellow)));
            }
            Event::End(TagEnd::Item) => {}

            Event::Start(Tag::Strong) => {
                style = style.bold();
            }
            Event::End(TagEnd::Strong) => {
                style = style.not_bold();
            }

            Event::Start(Tag::Emphasis) => {
                style = style.italic();
            }
            Event::End(TagEnd::Emphasis) => {
                style = style.not_italic();
            }

            Event::Start(Tag::BlockQuote(_)) => {
                current.push(Span::styled("▎", Style::default().fg(Color::DarkGray)));
            }
            Event::End(TagEnd::BlockQuote(_)) => {}

            Event::Start(Tag::Strikethrough) => {
                style = style.crossed_out();
            }
            Event::End(TagEnd::Strikethrough) => {
                style = style.not_crossed_out();
            }

            Event::Text(text) => {
                current.push(Span::styled(text.to_string(), style));
            }
            Event::Code(text) => {
                current.push(Span::styled(
                    text.to_string(),
                    Style::default()
                        .bg(Color::Rgb(50, 50, 50))
                        .fg(Color::Green),
                ));
            }

            Event::SoftBreak => {
                if in_heading {
                    lines.push(Line::from(current.drain(..).collect::<Vec<_>>()));
                    lines.push(Line::from(""));
                    style = Style::default().fg(Color::White);
                    in_heading = false;
                } else {
                    current.push(Span::raw(" "));
                }
            }

            Event::HardBreak => {
                lines.push(Line::from(current.drain(..).collect::<Vec<_>>()));
            }

            Event::Rule => {
                let rule = "─".repeat((width.max(20) - 4) as usize);
                lines.push(Line::from(vec![Span::styled(
                    rule,
                    Style::default().fg(Color::DarkGray),
                )]));
                lines.push(Line::from(""));
            }

            _ => {}
        }
    }

    if !current.is_empty() {
        lines.push(Line::from(current));
    }

    lines
}
