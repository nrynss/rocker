use ratatui::DefaultTerminal;
use ratatui::crossterm::event::{self, Event, KeyCode, KeyEvent, KeyEventKind, KeyModifiers};

use crate::data::constants;
use crate::game::music::{DistributionChannel, ReleaseType};
use crate::game::{
    BREAK_WEEKS, GIG_HEALTH_GUARD, GIG_STRESS_GUARD, Game, GameAction, PRESSING_TIERS,
    STUDIO_STRESS_BLOCK, TOUR_HEALTH_GUARD, TOUR_STRESS_GUARD, TourRig,
};

use super::render;

pub const SAVE_FILE_DEFAULT: &str = "rocker.sav";
const LOG_CAPACITY: usize = 200;

pub enum LogKind {
    /// Produced by game logic (weekly events, action results).
    Game,
    /// UI chrome: hints, confirmations.
    Ui,
    /// A rejected action or failed save/load.
    Error,
}

pub struct LogEntry {
    pub kind: LogKind,
    pub text: String,
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum SetupField {
    Name,
    BandName,
    Genre,
}

#[derive(Clone, Copy)]
pub enum FileMode {
    Save,
    Load,
}

pub enum Screen {
    Setup {
        field: SetupField,
    },
    Main,
    Deals {
        selected: usize,
        detail: bool,
    },
    SupportOffer,
    Charts {
        region: crate::game::world::ChartRegion,
        scroll: usize,
    },
    MarketingRelease {
        selected: usize,
    },
    MarketingCampaign {
        release_id: u32,
        release_name: String,
        selected: usize,
    },
    File {
        mode: FileMode,
        input: String,
    },
    GameOver,
    VenuePicker {
        selected: usize,
    },
    RegionPicker {
        selected: usize,
    },
    /// The rig + length picker reached after choosing a region: shows an
    /// itemized quote live as the player changes selection (design §A, M1).
    TourBookingPicker {
        region_index: usize,
        rig: TourRig,
        weeks: u8,
    },
    PressingPicker {
        release_type: ReleaseType,
        selected: usize,
        /// Distribution channel choice alongside the pressing run (design
        /// §E-3, M6) — meaningful only while unsigned; signed releases never
        /// reach this screen (`open_pressing_picker` dispatches straight
        /// through).
        channel: DistributionChannel,
    },
    TourReport {
        scroll: usize,
    },
    LifestylePicker {
        selected: usize,
    },
    /// Which sold-out/low-stock release to re-press (design §E-1 indie
    /// half, M6).
    RePressPicker {
        selected: usize,
    },
    /// The pressing-tier choice for a re-press, once the release is picked.
    RePressTierPicker {
        release_id: u32,
        selected: usize,
    },
}

/// What a main-menu row does when activated.
#[derive(Clone)]
pub enum MenuKind {
    Action(GameAction),
    Deals,
    SupportTour,
    Charts,
    TourReport,
    Marketing,
    Save,
    Load,
    Quit,
    Gig,
    GoOnTour,
    RecordSingle,
    RecordAlbum,
    Lifestyle,
    /// Open the re-press picker (design §E-1 indie half, M6).
    RePress,
}

pub struct MenuEntry {
    pub hotkey: char,
    pub label: &'static str,
    pub detail: String,
    pub enabled: bool,
    pub kind: MenuKind,
}

/// A release that can be marketed, in the order shown to the player.
pub struct MarketingTarget {
    pub id: u32,
    pub name: String,
    pub pending: bool,
    pub buzz: u8,
}

pub struct App {
    pub game: Game,
    pub screen: Screen,
    pub log: Vec<LogEntry>,
    pub menu_selected: usize,
    pub name_input: String,
    pub band_input: String,
    pub genre_selected: usize,
    pub(crate) should_exit: bool,
}

impl App {
    pub fn new(game: Game) -> Self {
        Self {
            game,
            screen: Screen::Setup {
                field: SetupField::Name,
            },
            log: Vec::new(),
            menu_selected: 0,
            name_input: String::new(),
            band_input: String::new(),
            genre_selected: 0,
            should_exit: false,
        }
    }

    pub fn run(mut self, terminal: &mut DefaultTerminal) -> Result<(), Box<dyn std::error::Error>> {
        while !self.should_exit {
            terminal.draw(|frame| render::draw(frame, &mut self))?;
            if let Event::Key(key) = event::read()?
                && key.kind == KeyEventKind::Press
            {
                self.handle_key(key);
            }
        }
        Ok(())
    }

    // --- Menu model ---

    pub fn menu_entries(&self) -> Vec<MenuEntry> {
        let game = &self.game;
        let signed = game.band.current_deal().is_some();
        let single_cost = game.recording_cost(&ReleaseType::Single);
        let album_cost = game.recording_cost(&ReleaseType::Album);
        // The cheapest pressing run, for affordability checks. M6: also the
        // currently-selected distribution channel's fee (§E-3) — Ok(0) while
        // signed, and `plan_distribution` folds the fame-gate check in too,
        // but here we just want the floor cost, not the error.
        let (single_min, album_min) = if signed {
            (single_cost, album_cost)
        } else {
            let fee = game
                .plan_distribution(game.current_distribution_channel)
                .unwrap_or(0);
            (
                single_cost + game.pressing_cost(&ReleaseType::Single, PRESSING_TIERS[0].1) + fee,
                album_cost + game.pressing_cost(&ReleaseType::Album, PRESSING_TIERS[0].1) + fee,
            )
        };
        let songs = game.band.unreleased_songs.len();
        let offers = game.pending_deal_offers.len();
        let releases = game.just_released_music.len() + game.band.total_releases();
        // M6 (§E-1 indie half): releases eligible for a player-initiated
        // re-press right now — empty for a signed act (its label restocks
        // on its own).
        let repress_count = game.repressable_releases().len();

        let mut entries = vec![
            MenuEntry {
                hotkey: '1',
                label: "Laze Around",
                detail: "-stress, +creativity".into(),
                enabled: true,
                kind: MenuKind::Action(GameAction::LazeAround),
            },
            MenuEntry {
                hotkey: '2',
                label: "Write Songs",
                detail: if game.player.stress >= 90 {
                    "too stressed".into()
                } else {
                    "1-3 new songs".into()
                },
                enabled: game.player.stress < 90,
                kind: MenuKind::Action(GameAction::WriteSongs),
            },
            MenuEntry {
                hotkey: '3',
                label: "Practice",
                detail: if game.player.stress >= STUDIO_STRESS_BLOCK {
                    "too stressed".into()
                } else {
                    "+2 band skill".into()
                },
                enabled: game.player.stress < STUDIO_STRESS_BLOCK,
                kind: MenuKind::Action(GameAction::Practice),
            },
            MenuEntry {
                hotkey: '4',
                label: "Record Single",
                detail: if game.player.stress >= 90 {
                    "too stressed".into()
                } else if songs == 0 {
                    "no songs written".into()
                } else if signed {
                    format!("${} — label presses", single_cost)
                } else {
                    format!("${} + pressing", single_cost)
                },
                enabled: game.player.stress < 90
                    && game.band.can_record_single()
                    && game.player.can_afford(single_min),
                kind: MenuKind::RecordSingle,
            },
            MenuEntry {
                hotkey: '5',
                label: "Record Album",
                detail: if game.player.stress >= 90 {
                    "too stressed".into()
                } else if songs < constants::MIN_ALBUM_SONGS as usize {
                    format!("{}/{} songs", songs, constants::MIN_ALBUM_SONGS)
                } else if signed {
                    format!("${} — label presses", album_cost)
                } else {
                    format!("${} + pressing", album_cost)
                },
                enabled: game.player.stress < 90
                    && game.band.can_record_album()
                    && game.player.can_afford(album_min),
                kind: MenuKind::RecordAlbum,
            },
            MenuEntry {
                hotkey: '6',
                label: "Play a Gig",
                detail: if game.player.stress >= GIG_STRESS_GUARD {
                    "too stressed out".into()
                } else if game.player.health < GIG_HEALTH_GUARD {
                    "too unwell".into()
                } else {
                    "venue picker".into()
                },
                enabled: game.player.stress < GIG_STRESS_GUARD
                    && game.player.health >= GIG_HEALTH_GUARD,
                kind: MenuKind::Gig,
            },
            MenuEntry {
                hotkey: '7',
                label: "Go on Tour",
                detail: if game.band.fame < 25 {
                    "needs 25 fame".into()
                } else if game.player.stress >= TOUR_STRESS_GUARD {
                    "too stressed out".into()
                } else if game.player.health < TOUR_HEALTH_GUARD {
                    "too unwell".into()
                } else {
                    "region picker".into()
                },
                enabled: game.band.fame >= 25
                    && game.player.stress < TOUR_STRESS_GUARD
                    && game.player.health >= TOUR_HEALTH_GUARD,
                kind: MenuKind::GoOnTour,
            },
            MenuEntry {
                hotkey: 't',
                label: "Support Slot…",
                detail: match &game.pending_support_offer {
                    Some(offer) => format!("{} want you!", offer.host_band),
                    None => "no offers".into(),
                },
                enabled: game.pending_support_offer.is_some(),
                kind: MenuKind::SupportTour,
            },
            MenuEntry {
                hotkey: '8',
                label: "Take a Break",
                detail: format!("{} weeks off — full recovery", BREAK_WEEKS),
                enabled: true,
                kind: MenuKind::Action(GameAction::TakeBreak),
            },
            MenuEntry {
                hotkey: '9',
                label: "Visit Doctor",
                detail: format!("${}", constants::DOCTOR_VISIT_COST),
                enabled: game.player.can_afford(constants::DOCTOR_VISIT_COST),
                kind: MenuKind::Action(GameAction::VisitDoctor),
            },
            MenuEntry {
                hotkey: 'm',
                label: "Marketing…",
                detail: if signed {
                    "your label handles promo".into()
                } else if releases == 0 {
                    "nothing to promote".into()
                } else {
                    format!(
                        "{} release{}",
                        releases,
                        if releases == 1 { "" } else { "s" }
                    )
                },
                enabled: !signed && releases > 0,
                kind: MenuKind::Marketing,
            },
            MenuEntry {
                hotkey: 'v',
                label: "Deal Offers…",
                detail: if offers == 0 {
                    "none pending".into()
                } else {
                    format!("{} waiting!", offers)
                },
                enabled: offers > 0,
                kind: MenuKind::Deals,
            },
            MenuEntry {
                hotkey: 'c',
                label: "Charts…",
                detail: match game
                    .world
                    .regional_charts
                    .get(&crate::game::world::ChartRegion::Local)
                    .and_then(|entries| entries.iter().position(|e| e.is_player))
                {
                    Some(spot) => format!("you're at #{} Local!", spot + 1),
                    None => "regional Top 100s".into(),
                },
                enabled: true,
                kind: MenuKind::Charts,
            },
            MenuEntry {
                hotkey: 'r',
                label: "Tour Report…",
                detail: match &game.last_tour_report {
                    Some(report) => format!(
                        "{} show{} · avg {}",
                        report.rows.len(),
                        if report.rows.len() == 1 { "" } else { "s" },
                        report.avg_reception
                    ),
                    None => "no report yet".into(),
                },
                enabled: true,
                kind: MenuKind::TourReport,
            },
            MenuEntry {
                hotkey: 'h',
                label: "Lifestyle…",
                detail: format!(
                    "{} · ${}/wk",
                    game.player.lifestyle.label(),
                    game.player.lifestyle.upkeep_per_week()
                ),
                enabled: true,
                kind: MenuKind::Lifestyle,
            },
            MenuEntry {
                hotkey: 'p',
                label: "Re-press…",
                detail: if signed {
                    "your label restocks automatically".into()
                } else if repress_count == 0 {
                    "nothing low on stock".into()
                } else {
                    format!(
                        "{} release{} ready",
                        repress_count,
                        if repress_count == 1 { "" } else { "s" }
                    )
                },
                enabled: !signed && repress_count > 0,
                kind: MenuKind::RePress,
            },
            MenuEntry {
                hotkey: 's',
                label: "Save Game",
                detail: String::new(),
                enabled: true,
                kind: MenuKind::Save,
            },
            MenuEntry {
                hotkey: 'l',
                label: "Load Game",
                detail: String::new(),
                enabled: true,
                kind: MenuKind::Load,
            },
            MenuEntry {
                hotkey: 'q',
                label: "Quit",
                detail: String::new(),
                enabled: true,
                kind: MenuKind::Quit,
            },
        ];

        // Turn-consuming actions come first; keep indices stable for hotkeys.
        entries.shrink_to_fit();
        entries
    }

    pub fn marketing_targets(&self) -> Vec<MarketingTarget> {
        let mut targets: Vec<MarketingTarget> = self
            .game
            .just_released_music
            .iter()
            .map(|r| MarketingTarget {
                id: r.id,
                name: r.name.clone(),
                pending: true,
                buzz: r.marketing_level_achieved,
            })
            .collect();
        for r in self
            .game
            .band
            .singles_released
            .iter()
            .chain(self.game.band.albums_released.iter())
        {
            targets.push(MarketingTarget {
                id: r.id,
                name: r.name.clone(),
                pending: false,
                buzz: r.marketing_level_achieved,
            });
        }
        targets
    }

    /// The current standings for one charts tab — Worldwide is derived
    /// fresh (design §C), the rest are read straight off their stored
    /// board. Shared by the input handler (to clamp scrolling) and the
    /// modal renderer, so both agree on what "this tab" means.
    pub fn charts_region_entries(
        &self,
        region: crate::game::world::ChartRegion,
    ) -> Vec<crate::game::world::ChartEntry> {
        if region == crate::game::world::ChartRegion::Worldwide {
            self.game.world.worldwide_chart()
        } else {
            self.game
                .world
                .regional_charts
                .get(&region)
                .cloned()
                .unwrap_or_default()
        }
    }

    // --- Logging ---

    pub(crate) fn push_log(&mut self, kind: LogKind, text: impl Into<String>) {
        self.log.push(LogEntry {
            kind,
            text: text.into(),
        });
        if self.log.len() > LOG_CAPACITY {
            let excess = self.log.len() - LOG_CAPACITY;
            self.log.drain(..excess);
        }
    }

    fn drain_game_log(&mut self) {
        for message in self.game.take_turn_log() {
            self.push_log(LogKind::Game, message);
        }
    }

    /// Run a game action, surface its messages, and follow game-over state.
    pub(crate) fn dispatch(&mut self, action: GameAction) {
        if let Err(message) = self.game.process_turn(action) {
            self.push_log(LogKind::Error, format!("❌ {}", message));
        }
        self.drain_game_log();
        if self.game.is_game_over() {
            self.screen = Screen::GameOver;
        }
    }

    // --- Input handling ---

    fn handle_key(&mut self, key: KeyEvent) {
        if key.code == KeyCode::Char('c') && key.modifiers.contains(KeyModifiers::CONTROL) {
            self.should_exit = true;
            return;
        }

        match &self.screen {
            Screen::Setup { .. } => self.handle_setup_key(key),
            Screen::Main => self.handle_main_key(key),
            Screen::Deals { .. } => self.handle_deals_key(key),
            Screen::SupportOffer => self.handle_support_offer_key(key),
            Screen::Charts { .. } => self.handle_charts_key(key),
            Screen::MarketingRelease { .. } => self.handle_marketing_release_key(key),
            Screen::MarketingCampaign { .. } => self.handle_marketing_campaign_key(key),
            Screen::File { .. } => self.handle_file_key(key),
            Screen::GameOver => self.should_exit = true,
            Screen::VenuePicker { .. } => self.handle_venue_picker_key(key),
            Screen::RegionPicker { .. } => self.handle_region_picker_key(key),
            Screen::TourBookingPicker { .. } => self.handle_tour_booking_picker_key(key),
            Screen::PressingPicker { .. } => self.handle_pressing_picker_key(key),
            Screen::TourReport { .. } => self.handle_tour_report_key(key),
            Screen::LifestylePicker { .. } => self.handle_lifestyle_picker_key(key),
            Screen::RePressPicker { .. } => self.handle_repress_picker_key(key),
            Screen::RePressTierPicker { .. } => self.handle_repress_tier_picker_key(key),
        }
    }
}
