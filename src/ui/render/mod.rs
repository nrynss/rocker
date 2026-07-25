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
                Screen::Discography { .. } => modals::draw_discography_modal(frame, app),
                Screen::Labels { .. } => modals::draw_labels_modal(frame, app),
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
                Screen::Vault { .. } => modals::draw_vault_modal(frame, app),
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

/// Group an integer with commas every three digits (e.g. `1234567` →
/// `"1,234,567"`). The building block for the discography's copies/income and
/// the label ledger's exact capital readout (L-UI-1).
pub(crate) fn group_thousands(n: u64) -> String {
    let digits = n.to_string();
    let len = digits.len();
    let mut out = String::with_capacity(len + len / 3);
    for (i, ch) in digits.chars().enumerate() {
        if i > 0 && (len - i).is_multiple_of(3) {
            out.push(',');
        }
        out.push(ch);
    }
    out
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

    /// The charts modal (region tabs, the §E format toggle, empty board, a
    /// populated board scrolled deep, and the derived Worldwide tab) should
    /// render without panicking (design §C, task M3).
    #[test]
    fn charts_modal_renders_every_region_without_panicking() {
        use crate::game::music::ReleaseType;
        use crate::game::world::ChartRegion;

        let backend = TestBackend::new(80, 24);
        let mut terminal = Terminal::new(backend).expect("test backend");
        let mut app = app_on_main();

        // Empty Local board, both formats.
        for format in [ReleaseType::Single, ReleaseType::Album] {
            app.screen = Screen::Charts {
                region: ChartRegion::Local,
                format,
                scroll: 0,
            };
            terminal.draw(|frame| draw(frame, &mut app)).unwrap();
        }

        // A crowded UK singles board, scrolled past the top 10 — and its
        // albums half, which stays empty and must render its own quiet line.
        for i in 0..40 {
            app.game.world.submit_chart_entry(
                ChartRegion::Uk,
                ReleaseType::Single,
                format!("Song {i}"),
                format!("Band {i}"),
                i == 0,
                500 + i as u32,
            );
        }
        for format in [ReleaseType::Single, ReleaseType::Album] {
            app.screen = Screen::Charts {
                region: ChartRegion::Uk,
                format,
                scroll: 25,
            };
            terminal.draw(|frame| draw(frame, &mut app)).unwrap();
        }

        // The derived Worldwide tab, per format.
        for format in [ReleaseType::Single, ReleaseType::Album] {
            app.screen = Screen::Charts {
                region: ChartRegion::Worldwide,
                format,
                scroll: 0,
            };
            terminal.draw(|frame| draw(frame, &mut app)).unwrap();
        }
    }

    /// A master as a finished session would leave it (design §D), so the
    /// vault and the release picker have something to draw.
    fn vault_master(
        name: &str,
        release_type: crate::game::music::ReleaseType,
    ) -> crate::game::music::Master {
        crate::game::music::Master {
            name: name.to_string(),
            release_type,
            release_quality: 62,
            songs_involved_quality_avg: 58,
            songs: Vec::new(),
            studio: crate::game::studios::RecordingStudio::DemoRoom,
            week_finished: 1,
            genre: None,
        }
    }

    /// The vault (design §D), the pressing/distribution picker (§E-3, M6) and
    /// the re-press pickers (§E-1 indie half) should render without panicking,
    /// both locked (low fame, National unavailable) and unlocked (high fame).
    #[test]
    fn pressing_and_repress_pickers_render_without_panicking() {
        use crate::game::music::{DistributionChannel, ReleaseType};

        let backend = TestBackend::new(80, 24);
        let mut terminal = Terminal::new(backend).expect("test backend");
        let mut app = app_on_main();

        // The vault, empty first — a band with nothing finished must still get
        // a screen rather than an index panic.
        app.screen = Screen::Vault { selected: 0 };
        terminal.draw(|frame| draw(frame, &mut app)).unwrap();

        app.game.band.masters = vec![
            vault_master("Single: Opening Shot", ReleaseType::Single),
            vault_master("The Long Player", ReleaseType::Album),
        ];
        app.screen = Screen::Vault { selected: 1 };
        terminal.draw(|frame| draw(frame, &mut app)).unwrap();

        // Low fame: National is locked, so the channel row must render its
        // 🔒 state rather than panicking.
        app.screen = Screen::PressingPicker {
            master: 0,
            selected: 0,
            channel: DistributionChannel::National,
        };
        terminal.draw(|frame| draw(frame, &mut app)).unwrap();

        // High fame, every channel unlocked; also exercise the album variant.
        app.game.band.fame = 90;
        app.screen = Screen::PressingPicker {
            master: 1,
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
            label_id: None,
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

    /// Read the whole rendered screen back as a flat string, so a test can
    /// assert that specific text actually reached the buffer.
    fn buffer_text(terminal: &Terminal<TestBackend>) -> String {
        terminal
            .backend()
            .buffer()
            .content
            .iter()
            .map(ratatui::buffer::Cell::symbol)
            .collect()
    }

    /// L-UI-1 headline: the discography browser must render each record's
    /// commercial story — copies sold (comma-grouped, or `M` for the
    /// platinum sellers), income, and chart peak — not just its title. This
    /// is the "no way to see the sales of each record" gap the user reported.
    #[test]
    fn discography_modal_shows_each_records_sales() {
        use crate::game::music::{Release, ReleaseType};

        let backend = TestBackend::new(80, 24);
        let mut terminal = Terminal::new(backend).expect("test backend");
        let mut app = app_on_main();

        let make = |id: u32,
                    name: &str,
                    release_type: ReleaseType,
                    copies_sold: u32,
                    total_income_generated: u32,
                    peak_chart_position: Option<u8>,
                    certified: u8,
                    week_released: u32| Release {
            id,
            name: name.to_string(),
            release_type,
            release_quality: 60,
            week_released,
            songs_involved_quality_avg: 60,
            active_marketing: Vec::new(),
            marketing_level_achieved: 0,
            initial_sales_score: 0,
            total_income_generated,
            genre: None,
            copies_pressed: 2_000_000,
            copies_sold,
            peak_chart_position,
            singles_cut: 0,
            certified,
            distribution_channel: None,
            label_market_reach: None,
        };

        // Empty catalogue first: the modal must render its empty state, not
        // panic on an empty list.
        app.screen = Screen::Discography { scroll: 0 };
        terminal.draw(|frame| draw(frame, &mut app)).unwrap();

        // A gold album that charted, and a platinum-scale single.
        app.game.band.albums_released = vec![make(
            1,
            "Debut LP",
            ReleaseType::Album,
            48_200,
            91_500,
            Some(7),
            2,
            10,
        )];
        app.game.band.singles_released = vec![make(
            2,
            "Big Hit",
            ReleaseType::Single,
            1_200_000,
            640_000,
            Some(1),
            4,
            20,
        )];

        terminal.draw(|frame| draw(frame, &mut app)).unwrap();
        let text = buffer_text(&terminal);

        assert!(
            text.contains("48,200 copies"),
            "album copies should render comma-grouped; got:\n{text}"
        );
        assert!(
            text.contains("$91,500"),
            "album income should render; got:\n{text}"
        );
        assert!(
            text.contains("peak #7"),
            "album chart peak should render; got:\n{text}"
        );
        assert!(
            text.contains("1.2M copies"),
            "a million-selling single should render in M form; got:\n{text}"
        );
        assert!(
            text.contains("$640,000"),
            "single income should render; got:\n{text}"
        );
    }

    /// L-UI-1 / design §H: the label ledger lists every world label with its
    /// tier, and shows the player's own label's *exact* capital while keeping
    /// other labels' figures qualitative.
    #[test]
    fn labels_modal_lists_labels_and_reveals_player_label_capital() {
        let backend = TestBackend::new(80, 24);
        let mut terminal = Terminal::new(backend).expect("test backend");
        let mut app = app_on_main();

        // Worldgen seeds the label industry. Make the first label the
        // player's and give it a known capital so we can assert the exact
        // figure renders.
        let first_id = *app
            .game
            .world
            .labels
            .keys()
            .next()
            .expect("worldgen seeds labels");
        app.game
            .world
            .labels
            .get_mut(&first_id)
            .expect("label present")
            .capital = 1_234_567;
        app.game.band.record_deal = Some(crate::game::band::RecordDeal {
            label_name: "Test Records".to_string(),
            label_tier: "Major".to_string(),
            advance: 0,
            royalty_rate: 0.12,
            albums_required: 2,
            albums_delivered: 0,
            market_reach: 80,
            unrecouped: 0,
            signed_week: 0,
            term_weeks: 0,
            label_id: Some(first_id),
        });

        app.screen = Screen::Labels { scroll: 0 };
        terminal.draw(|frame| draw(frame, &mut app)).unwrap();
        let text = buffer_text(&terminal);

        assert!(
            text.contains("Major") || text.contains("Independent") || text.contains("Boutique"),
            "labels should render their tier; got:\n{text}"
        );
        assert!(
            text.contains("your label"),
            "the player's own label should be tagged; got:\n{text}"
        );
        assert!(
            text.contains("$1,234,567"),
            "the player's own label should show its exact capital; got:\n{text}"
        );
    }

    /// L-UI-2 / design §H: the deal picker annotates each offer with the
    /// offering label's *live* health — its qualitative capital band (never a
    /// spied exact figure) and a distress flag — so "rising indie vs safe
    /// major" is legible at the table. The detail view adds the label's current
    /// standing (tier, reach, band). Rendered wide so the row's tail is in-buffer.
    #[test]
    fn deal_picker_annotates_offers_with_label_health() {
        use crate::data_loader::RecordLabel;
        use crate::game::world::{LabelStatus, PotentialDealOffer};

        let backend = TestBackend::new(110, 30);
        let mut terminal = Terminal::new(backend).expect("test backend");
        let mut app = app_on_main();

        // Two real seeded labels, forced to contrasting health: a flush, active
        // one and one bleeding red into distress.
        let ids: Vec<u32> = app.game.world.labels.keys().take(2).copied().collect();
        assert!(ids.len() >= 2, "worldgen should seed at least two labels");
        let flush_name = app.game.world.labels[&ids[0]].name.clone();
        let sick_name = app.game.world.labels[&ids[1]].name.clone();
        {
            let flush = app
                .game
                .world
                .labels
                .get_mut(&ids[0])
                .expect("label present");
            flush.capital = 5_000_000; // past the "flush" floor
            flush.status = LabelStatus::Active;
            flush.market_reach = 82;
        }
        {
            let sick = app
                .game
                .world
                .labels
                .get_mut(&ids[1])
                .expect("label present");
            sick.capital = -50_000; // in the red
            sick.status = LabelStatus::Distressed;
            sick.market_reach = 21;
        }

        let week = app.game.week;
        let make_offer = |label_name: &str, advance: u32| PotentialDealOffer {
            label_name: label_name.to_string(),
            label_tier: "Independent".to_string(),
            advance,
            royalty_rate: 0.12,
            albums_required: 2,
            original_label_data: RecordLabel {
                name: label_name.to_string(),
                grade: "B".to_string(),
                market_reach: 55,
                financial_power: 50,
                artist_development: 55,
                creative_freedom: 60,
                royalty_rate: 12,
                advance_range: [10_000, 80_000],
                specialty_genres: Vec::new(),
                founded: 1975,
                reputation: "Respected".to_string(),
            },
            expires_week: Some(week + 4),
            term_weeks: 104,
            carry_forward_unrecouped: 0,
        };
        app.game.pending_deal_offers = vec![
            make_offer(&flush_name, 40_000),
            make_offer(&sick_name, 250_000),
        ];

        // List view: both offers carry their label's live capital band, and the
        // distressed one is flagged.
        app.screen = Screen::Deals {
            selected: 0,
            detail: false,
        };
        terminal.draw(|frame| draw(frame, &mut app)).unwrap();
        let list_text = buffer_text(&terminal);
        assert!(
            list_text.contains("flush"),
            "the healthy offer should show a 'flush' band; got:\n{list_text}"
        );
        assert!(
            list_text.contains("in the red"),
            "the struggling offer should show an 'in the red' band; got:\n{list_text}"
        );
        assert!(
            list_text.contains("distressed"),
            "a distressed label should be flagged on its offer row; got:\n{list_text}"
        );

        // Detail view of the flush offer: the live "current standing" block
        // renders the tier, reach, and band.
        app.screen = Screen::Deals {
            selected: 0,
            detail: true,
        };
        terminal.draw(|frame| draw(frame, &mut app)).unwrap();
        let detail_text = buffer_text(&terminal);
        assert!(
            detail_text.contains("Where they stand now"),
            "detail should show the live standing section; got:\n{detail_text}"
        );
        assert!(
            detail_text.contains("reach 82/100"),
            "detail should show the label's live reach; got:\n{detail_text}"
        );
        assert!(
            detail_text.contains("flush"),
            "detail should show the label's live capital band; got:\n{detail_text}"
        );
    }

    /// L-UI-2 / design §H: the charts modal shows which label a signed scene
    /// act records for, wiring `GameWorld::rival_act_label`. Rendered wide so
    /// the row's label suffix is in-buffer.
    #[test]
    fn charts_modal_shows_a_signed_scene_acts_label() {
        use crate::game::genre::MusicGenre;
        use crate::game::world::{ChartRegion, SceneBand};

        let backend = TestBackend::new(120, 30);
        let mut terminal = Terminal::new(backend).expect("test backend");
        let mut app = app_on_main();

        // A scene act signed to a named label, charting high on the UK board.
        let band_name = "Zzyzx Test Quartet".to_string();
        let label_name = "Testlabel Records".to_string();
        app.game.world.bands.push(SceneBand {
            name: band_name.clone(),
            fame: 55,
            peak_fame: 55,
            latest_release: "Kicks".to_string(),
            genre: MusicGenre::ALL[0].clone(),
            label: Some(label_name.clone()),
            momentum: 1,
        });
        app.game.world.submit_chart_entry(
            ChartRegion::Uk,
            crate::game::music::ReleaseType::Album,
            "Kicks".to_string(),
            band_name.clone(),
            false,
            900,
        );

        app.screen = Screen::Charts {
            region: ChartRegion::Uk,
            format: crate::game::music::ReleaseType::Album,
            scroll: 0,
        };
        terminal.draw(|frame| draw(frame, &mut app)).unwrap();
        let text = buffer_text(&terminal);
        assert!(
            text.contains(&band_name),
            "the charting act should render; got:\n{text}"
        );
        assert!(
            text.contains(&label_name),
            "a signed scene act should show its label on its chart row; got:\n{text}"
        );
    }
}
