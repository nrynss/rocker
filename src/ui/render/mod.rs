//! UI rendering submodules, orchestrating setup, game over, main layouts, panels, and modals.

mod game_over;
mod layout;
mod modals;
mod panels;
mod setup;

use ratatui::{
    Frame,
    layout::{Constraint, Layout, Rect},
    style::{Color, Style},
    widgets::Gauge,
};

use super::app::{App, Screen};

pub(crate) const ACCENT: Color = Color::Red;

pub fn draw(frame: &mut Frame, app: &mut App) {
    match &app.screen {
        Screen::Setup { .. } => setup::draw_setup(frame, app),
        Screen::GameOver => game_over::draw_game_over(frame, app),
        _ => {
            layout::draw_main(frame, app);
            match &app.screen {
                Screen::Deals { .. } => modals::draw_deals_modal(frame, app),
                Screen::SupportOffer => modals::draw_support_modal(frame, app),
                Screen::Charts => modals::draw_charts_modal(frame, app),
                Screen::MarketingRelease { .. } | Screen::MarketingCampaign { .. } => {
                    modals::draw_marketing_modal(frame, app)
                }
                Screen::File { .. } => modals::draw_file_modal(frame, app),
                Screen::VenuePicker { .. } => modals::draw_venue_picker_modal(frame, app),
                Screen::RegionPicker { .. } => modals::draw_region_picker_modal(frame, app),
                Screen::PressingPicker { .. } => modals::draw_pressing_picker_modal(frame, app),
                _ => {}
            }
        }
    }
}

// --- Shared helpers ---

pub(crate) fn centered_rect(percent_x: u16, percent_y: u16, r: Rect) -> Rect {
    let [_, mid, _] = Layout::vertical([
        Constraint::Percentage((100 - percent_y) / 2),
        Constraint::Percentage(percent_y),
        Constraint::Percentage((100 - percent_y) / 2),
    ])
    .areas(r);
    let [_, area, _] = Layout::horizontal([
        Constraint::Percentage((100 - percent_x) / 2),
        Constraint::Percentage(percent_x),
        Constraint::Percentage((100 - percent_x) / 2),
    ])
    .areas(mid);
    area
}

pub(crate) fn gauge(label: String, value: u8, color: Color) -> Gauge<'static> {
    Gauge::default()
        .ratio(f64::from(value.min(100)) / 100.0)
        .label(label)
        .gauge_style(Style::new().fg(color).bg(Color::Black))
        .use_unicode(true)
}

pub(crate) fn scale_color(value: u8, low_is_bad: bool) -> Color {
    let good = if low_is_bad { value } else { 100 - value };
    match good {
        0..=30 => Color::Red,
        31..=60 => Color::Yellow,
        _ => Color::Green,
    }
}

pub(crate) fn format_population(pop: u32) -> String {
    if pop >= 1_000_000 {
        format!("{:.1}M", pop as f32 / 1_000_000.0)
    } else {
        format!("{}k", pop / 1000)
    }
}
