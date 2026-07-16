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
                Screen::Charts { .. } => modals::draw_charts_modal(frame, app),
                Screen::TourReport { .. } => modals::draw_tour_report_modal(frame, app),
                Screen::MarketingRelease { .. } | Screen::MarketingCampaign { .. } => {
                    modals::draw_marketing_modal(frame, app)
                }
                Screen::File { .. } => modals::draw_file_modal(frame, app),
                Screen::VenuePicker { .. } => modals::draw_venue_picker_modal(frame, app),
                Screen::RegionPicker { .. } => modals::draw_region_picker_modal(frame, app),
                Screen::TourBookingPicker { .. } => {
                    modals::draw_tour_booking_picker_modal(frame, app)
                }
                Screen::PressingPicker { .. } => modals::draw_pressing_picker_modal(frame, app),
                Screen::LifestylePicker { .. } => modals::draw_lifestyle_picker_modal(frame, app),
                Screen::RePressPicker { .. } => modals::draw_repress_picker_modal(frame, app),
                Screen::RePressTierPicker { .. } => {
                    modals::draw_repress_tier_picker_modal(frame, app)
                }
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

    /// M1's region picker and rig/length booking picker (design §A) should
    /// render without panicking, both with a valid live quote and with a
    /// picker state that resolves to a quote error (rig gated by fame) —
    /// the modal must degrade gracefully, never panic, when the quote is
    /// unavailable.
    #[test]
    fn tour_booking_picker_renders_without_panicking() {
        let backend = TestBackend::new(80, 24);
        let mut terminal = Terminal::new(backend).expect("test backend");

        let mut app = app_on_main();
        app.screen = Screen::RegionPicker { selected: 0 };
        terminal.draw(|frame| draw(frame, &mut app)).unwrap();

        // A low-fame band: Bus/Truck/Full and the longer lengths are all
        // gated, so the quote for a non-Van selection should resolve to an
        // Err the modal must render as a message rather than panicking.
        app.screen = Screen::TourBookingPicker {
            region_index: 0,
            rig: crate::game::TourRig::Full,
            weeks: 4,
        };
        terminal.draw(|frame| draw(frame, &mut app)).unwrap();

        // A high-fame band: every rig/length should quote cleanly.
        app.game.band.fame = 90;
        terminal.draw(|frame| draw(frame, &mut app)).unwrap();
    }

    /// The charts modal (region tabs, empty board, a populated board
    /// scrolled deep, and the derived Worldwide tab) should render without
    /// panicking (design §C, task M3).
    #[test]
    fn charts_modal_renders_every_region_without_panicking() {
        use crate::game::world::ChartRegion;

        let backend = TestBackend::new(80, 24);
        let mut terminal = Terminal::new(backend).expect("test backend");
        let mut app = app_on_main();

        // Empty Local board.
        app.screen = Screen::Charts {
            region: ChartRegion::Local,
            scroll: 0,
        };
        terminal.draw(|frame| draw(frame, &mut app)).unwrap();

        // A crowded UK board, scrolled past the top 10.
        for i in 0..40 {
            app.game.world.submit_chart_entry(
                ChartRegion::Uk,
                format!("Song {i}"),
                format!("Band {i}"),
                i == 0,
                500 + i as u32,
            );
        }
        app.screen = Screen::Charts {
            region: ChartRegion::Uk,
            scroll: 25,
        };
        terminal.draw(|frame| draw(frame, &mut app)).unwrap();

        // The derived Worldwide tab.
        app.screen = Screen::Charts {
            region: ChartRegion::Worldwide,
            scroll: 0,
        };
        terminal.draw(|frame| draw(frame, &mut app)).unwrap();
    }

    /// The pressing/distribution picker (design §E-3, M6) and the re-press
    /// pickers (§E-1 indie half) should render without panicking, both
    /// locked (low fame, National unavailable) and unlocked (high fame).
    #[test]
    fn pressing_and_repress_pickers_render_without_panicking() {
        use crate::game::music::{DistributionChannel, ReleaseType};

        let backend = TestBackend::new(80, 24);
        let mut terminal = Terminal::new(backend).expect("test backend");
        let mut app = app_on_main();

        // Low fame: National is locked, so the channel row must render its
        // 🔒 state rather than panicking.
        app.screen = Screen::PressingPicker {
            release_type: ReleaseType::Single,
            selected: 0,
            channel: DistributionChannel::National,
        };
        terminal.draw(|frame| draw(frame, &mut app)).unwrap();

        // High fame, every channel unlocked; also exercise the album variant.
        app.game.band.fame = 90;
        app.screen = Screen::PressingPicker {
            release_type: ReleaseType::Album,
            selected: 2,
            channel: DistributionChannel::Regional,
        };
        terminal.draw(|frame| draw(frame, &mut app)).unwrap();

        // Signed: the channel row collapses to a label-handles-it message.
        app.game.band.record_deal = Some(crate::game::band::RecordDeal {
            label_name: "Test Records".to_string(),
            label_tier: "Major".to_string(),
            advance: 0,
            royalty_rate: 0.12,
            albums_required: 2,
            albums_delivered: 0,
            market_reach: 70,
            unrecouped: 0,
            signed_week: 0,
            term_weeks: 0,
        });
        terminal.draw(|frame| draw(frame, &mut app)).unwrap();
        app.game.band.record_deal = None;

        // Re-press pickers: empty list, then a populated one.
        app.screen = Screen::RePressPicker { selected: 0 };
        terminal.draw(|frame| draw(frame, &mut app)).unwrap();

        let mut release = app
            .game
            .band
            .singles_released
            .first()
            .cloned()
            .unwrap_or_else(|| {
                let id = app.game.next_release_id;
                app.game.next_release_id += 1;
                crate::game::music::Release {
                    id,
                    name: "Test Single".to_string(),
                    release_type: ReleaseType::Single,
                    release_quality: 50,
                    week_released: 1,
                    songs_involved_quality_avg: 50,
                    active_marketing: Vec::new(),
                    marketing_level_achieved: 0,
                    initial_sales_score: 0,
                    total_income_generated: 0,
                    genre: None,
                    copies_pressed: 1_000,
                    copies_sold: 0,
                    peak_chart_position: None,
                    singles_cut: 0,
                    certified: 0,
                    distribution_channel: None,
                    label_market_reach: None,
                }
            });
        release.copies_pressed = 1_000;
        release.copies_sold = 950;
        let release_id = release.id;
        app.game.band.singles_released = vec![release];
        app.screen = Screen::RePressPicker { selected: 0 };
        terminal.draw(|frame| draw(frame, &mut app)).unwrap();

        app.screen = Screen::RePressTierPicker {
            release_id,
            selected: 0,
        };
        terminal.draw(|frame| draw(frame, &mut app)).unwrap();
    }
}
