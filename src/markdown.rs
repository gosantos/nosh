use ratatui::style::Color;
use ratatui::text::Line;
use ratatui_markdown::markdown::MarkdownRenderer;
use ratatui_markdown::theme::{Generation, RichTextTheme};

struct Theme;

impl RichTextTheme for Theme {
    fn generation(&self) -> Generation {
        Generation(1)
    }
    fn get_text_color(&self) -> Color {
        Color::White
    }
    fn get_muted_text_color(&self) -> Color {
        Color::Gray
    }
    fn get_background_color(&self) -> Color {
        Color::Reset
    }
    fn get_primary_color(&self) -> Color {
        Color::Cyan
    }
    fn get_secondary_color(&self) -> Color {
        Color::Blue
    }
    fn get_info_color(&self) -> Color {
        Color::LightBlue
    }
    fn get_border_color(&self) -> Color {
        Color::DarkGray
    }
    fn get_focused_border_color(&self) -> Color {
        Color::White
    }
    fn get_popup_selected_background(&self) -> Color {
        Color::DarkGray
    }
    fn get_popup_selected_text_color(&self) -> Color {
        Color::White
    }
    fn get_json_key_color(&self) -> Color {
        Color::LightCyan
    }
    fn get_json_string_color(&self) -> Color {
        Color::Green
    }
    fn get_json_number_color(&self) -> Color {
        Color::Yellow
    }
    fn get_json_bool_color(&self) -> Color {
        Color::Magenta
    }
    fn get_json_null_color(&self) -> Color {
        Color::DarkGray
    }
    fn get_accent_yellow(&self) -> Color {
        Color::Yellow
    }
}

pub fn render(content: &str, width: u16) -> Vec<Line<'static>> {
    let renderer = MarkdownRenderer::new(width as usize);
    let theme = Theme;
    let blocks = renderer.parse(content);
    renderer.render(&blocks, &theme)
}
