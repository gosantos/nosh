use chrono::{Datelike, Duration, Local, NaiveDate, Weekday};
use ratatui::{
    layout::{Alignment, Rect},
    style::{Color, Style},
    text::{Line, Span},
    widgets::Paragraph,
    Frame,
};

static DAY_NAMES: [&str; 7] = ["Mo", "Tu", "We", "Th", "Fr", "Sa", "Su"];
static MONTH_NAMES: [&str; 12] = [
    "January", "February", "March", "April", "May", "June",
    "July", "August", "September", "October", "November", "December",
];

pub struct CalendarState {
    pub view_year: i32,
    pub view_month: u32,
    pub selected: Option<NaiveDate>,
    pub confirmed: Option<NaiveDate>,
    pub today: NaiveDate,
}

impl CalendarState {
    pub fn new(initial: Option<NaiveDate>) -> Self {
        let today = Local::now().naive_local().date();
        let (view_year, view_month) = initial
            .map(|d| (d.year(), d.month()))
            .unwrap_or((today.year(), today.month()));
        CalendarState {
            view_year,
            view_month,
            selected: initial.or(Some(today)),
            confirmed: initial,
            today,
        }
    }

    pub fn move_left(&mut self) {
        if let Some(d) = self.selected {
            let new_d = d - Duration::days(1);
            self.selected = Some(new_d);
            self.sync_view(new_d);
        }
    }

    pub fn move_right(&mut self) {
        if let Some(d) = self.selected {
            let new_d = d + Duration::days(1);
            self.selected = Some(new_d);
            self.sync_view(new_d);
        }
    }

    pub fn move_up(&mut self) {
        if let Some(d) = self.selected {
            let new_d = d - Duration::days(7);
            self.selected = Some(new_d);
            self.sync_view(new_d);
        }
    }

    pub fn move_down(&mut self) {
        if let Some(d) = self.selected {
            let new_d = d + Duration::days(7);
            self.selected = Some(new_d);
            self.sync_view(new_d);
        }
    }

    pub fn next_month(&mut self) {
        if self.view_month == 12 {
            self.view_month = 1;
            self.view_year += 1;
        } else {
            self.view_month += 1;
        }
    }

    pub fn prev_month(&mut self) {
        if self.view_month == 1 {
            self.view_month = 12;
            self.view_year -= 1;
        } else {
            self.view_month -= 1;
        }
    }

    fn sync_view(&mut self, d: NaiveDate) {
        self.view_year = d.year();
        self.view_month = d.month();
    }

    pub fn clear_selection(&mut self) {
        self.selected = None;
        self.confirmed = None;
    }

    pub fn confirm(&mut self) {
        self.confirmed = self.selected;
    }

    pub fn jump_today(&mut self) {
        self.selected = Some(self.today);
        self.sync_view(self.today);
    }

    pub fn jump_end_of_week(&mut self) {
        let friday = next_friday(self.today, 0);
        self.selected = Some(friday);
        self.sync_view(friday);
    }

    pub fn jump_next_week(&mut self) {
        let friday = next_friday(self.today, 7);
        self.selected = Some(friday);
        self.sync_view(friday);
    }

    pub fn jump_30_days(&mut self) {
        let target = self.today + Duration::days(30);
        self.selected = Some(target);
        self.sync_view(target);
    }
}

fn next_friday(from: NaiveDate, add_weeks: i64) -> NaiveDate {
    let days_until_friday = match from.weekday() {
        Weekday::Mon => 4,
        Weekday::Tue => 3,
        Weekday::Wed => 2,
        Weekday::Thu => 1,
        Weekday::Fri => 0,
        Weekday::Sat => 6,
        Weekday::Sun => 5,
    };
    from + Duration::days(days_until_friday + add_weeks)
}

fn days_in_month(year: i32, month: u32) -> u32 {
    NaiveDate::from_ymd_opt(
        year,
        month,
        1,
    )
    .map(|d| {
        if month == 12 {
            NaiveDate::from_ymd_opt(year + 1, 1, 1)
        } else {
            NaiveDate::from_ymd_opt(year, month + 1, 1)
        }
        .map(|next| (next - d).num_days() as u32)
        .unwrap_or(30)
    })
    .unwrap_or(30)
}

fn compute_grid(year: i32, month: u32) -> Vec<Vec<Option<u32>>> {
    let first = NaiveDate::from_ymd_opt(year, month, 1).unwrap();
    let num_days = days_in_month(year, month);
    let start_col = first.weekday().num_days_from_monday() as usize;

    let mut grid = vec![vec![None; 7]; 6];
    let mut day = 1u32;
    for row in 0..6 {
        for col in 0..7 {
            if row == 0 && col < start_col {
                continue;
            }
            if day > num_days {
                break;
            }
            grid[row][col] = Some(day);
            day += 1;
        }
    }
    grid
}

use crate::app::TodoFormFocus;

pub fn grid_rows(year: i32, month: u32) -> usize {
    let first = NaiveDate::from_ymd_opt(year, month, 1).unwrap();
    let num_days = days_in_month(year, month);
    let start_col = first.weekday().num_days_from_monday() as usize;
    let total_cells = start_col + num_days as usize;
    if total_cells <= 28 { 4 } else { 5 }
}

pub fn month_name_str(month: u32) -> &'static str {
    MONTH_NAMES[month as usize - 1]
}

pub fn render_day_header(frame: &mut Frame, area: Rect) {
    let spans: Vec<Span> = DAY_NAMES
        .iter()
        .map(|name| {
            let s = format!("{:>2} ", name);
            let color = if *name == "Sa" || *name == "Su" {
                Color::DarkGray
            } else {
                Color::Gray
            };
            Span::styled(s, Style::default().fg(color))
        })
        .collect();
    frame.render_widget(
        Paragraph::new(Line::from(spans)).alignment(Alignment::Center),
        area,
    );
}

pub fn render_grid_compact(
    frame: &mut Frame,
    area: Rect,
    state: &CalendarState,
    focus: &TodoFormFocus,
) {
    let is_focused = matches!(focus, TodoFormFocus::Calendar);
    let grid = compute_grid(state.view_year, state.view_month);
    let cell_width = 3u16;
    let grid_width = cell_width * 7;
    let spacing = area.width.saturating_sub(grid_width) / 2;
    let x_offset = area.x + spacing;

    for (row_idx, row) in grid.iter().enumerate() {
        if row_idx >= area.height as usize {
            break;
        }
        let cy = area.y + row_idx as u16;
        for (col_idx, cell) in row.iter().enumerate() {
            let cx = x_offset + (col_idx as u16) * cell_width;
            let cell_area = Rect::new(cx, cy, cell_width, 1);

            if let Some(day) = cell {
                let date =
                    NaiveDate::from_ymd_opt(state.view_year, state.view_month, *day).unwrap();
                let is_sel = state.selected == Some(date);
                let is_today = date == state.today;

                let fg = if date < state.today {
                    Color::DarkGray
                } else {
                    Color::White
                };

                let bg = if is_sel && is_focused {
                    Color::Rgb(50, 80, 110)
                } else if is_sel {
                    Color::Rgb(40, 40, 50)
                } else if is_today {
                    Color::Rgb(30, 50, 70)
                } else {
                    Color::Reset
                };

                let text = format!("{:>2} ", day);
                frame.render_widget(
                    Paragraph::new(Line::from(vec![Span::styled(text, Style::default().fg(fg).bg(bg))])),
                    cell_area,
                );
            }
        }
    }
}


