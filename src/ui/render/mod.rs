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
                Screen::TourReport { .. } => modals::draw_tour_report_modal(frame, app),
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::game::{Game, ShowReport, TourReport};
    use ratatui::Terminal;
    use ratatui::backend::TestBackend;

    fn app_on_main() -> App {
        let mut app = App::new(Game::new().expect("data files present"));
        app.game.initialize_player(
            "Ray",
            "The Rayguns",
            crate::game::genre::MusicGenre::ALL[0].clone(),
        );
        app.screen = Screen::Main;
        app
    }

    /// The main screen (four bars, energy gone) and the tour report modal
    /// (empty and populated) should render without panicking on an 80x24
    /// terminal — the panel/modal smoke test called for by L4.
    #[test]
    fn main_screen_and_tour_report_render_without_panicking() {
        let backend = TestBackend::new(80, 24);
        let mut terminal = Terminal::new(backend).expect("test backend");

        let mut app = app_on_main();
        terminal.draw(|frame| draw(frame, &mut app)).unwrap();

        // Empty state: no report yet.
        app.screen = Screen::TourReport { scroll: 0 };
        terminal.draw(|frame| draw(frame, &mut app)).unwrap();

        // Populated state: a 20-show tour, scrolled to the last row.
        let rows: Vec<ShowReport> = (0..20)
            .map(|i| ShowReport {
                week: 1 + i / 5,
                venue_name: format!("Venue {i} (City {i})"),
                verdict: ["rough", "solid", "great", "transcendent"][i as usize % 4].to_string(),
                reception: 40 + (i as u8 * 3) % 60,
                attendance: 300 + i * 10,
                capacity: 500,
                take: 1000 + i * 50,
            })
            .collect();
        app.game.last_tour_report = Some(TourReport {
            avg_reception: 65,
            total_gross: rows.iter().map(|r| r.take).sum(),
            fame_gained: 5,
            rows,
        });
        app.screen = Screen::TourReport { scroll: 19 };
        terminal.draw(|frame| draw(frame, &mut app)).unwrap();
    }
}
