use ratatui::DefaultTerminal;
use ratatui::crossterm::event::{self, Event, KeyCode, KeyEvent, KeyEventKind, KeyModifiers};

use crate::data::constants;
use crate::game::music::{MarketingCampaignType, ReleaseType};
use crate::game::world::MusicGenre;
use crate::game::{BREAK_WEEKS, Game, GameAction, PRESSING_TIERS};

use super::render;

pub const SAVE_FILE_DEFAULT: &str = "rocker.sav";
const LOG_CAPACITY: usize = 200;
const INPUT_MAX_LEN: usize = 24;

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
    Setup { field: SetupField },
    Main,
    Deals { selected: usize, detail: bool },
    SupportOffer,
    MarketingRelease { selected: usize },
    MarketingCampaign { release_id: u32, release_name: String, selected: usize },
    File { mode: FileMode, input: String },
    GameOver,
    VenuePicker { selected: usize },
    RegionPicker { selected: usize },
    PressingPicker { release_type: ReleaseType, selected: usize },
}

/// What a main-menu row does when activated.
#[derive(Clone)]
pub enum MenuKind {
    Action(GameAction),
    Deals,
    SupportTour,
    Marketing,
    Save,
    Load,
    Quit,
    Gig,
    GoOnTour,
    RecordSingle,
    RecordAlbum,
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
    should_exit: bool,
}

impl App {
    pub fn new(game: Game) -> Self {
        Self {
            game,
            screen: Screen::Setup { field: SetupField::Name },
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
        // The cheapest pressing run, for affordability checks.
        let (single_min, album_min) = if signed {
            (single_cost, album_cost)
        } else {
            (
                single_cost + game.pressing_cost(&ReleaseType::Single, PRESSING_TIERS[0].1),
                album_cost + game.pressing_cost(&ReleaseType::Album, PRESSING_TIERS[0].1),
            )
        };
        let songs = game.band.unreleased_songs.len();
        let offers = game.pending_deal_offers.len();
        let releases = game.just_released_music.len() + game.band.total_releases();

        let mut entries = vec![
            MenuEntry {
                hotkey: '1',
                label: "Laze Around",
                detail: "+energy, -stress".into(),
                enabled: true,
                kind: MenuKind::Action(GameAction::LazeAround),
            },
            MenuEntry {
                hotkey: '2',
                label: "Write Songs",
                detail: if game.player.energy < 20 { "too tired".into() } else { "1-3 new songs".into() },
                enabled: game.player.energy >= 20,
                kind: MenuKind::Action(GameAction::WriteSongs),
            },
            MenuEntry {
                hotkey: '3',
                label: "Practice",
                detail: if game.player.energy < 15 { "too tired".into() } else { "+2 band skill".into() },
                enabled: game.player.energy >= 15,
                kind: MenuKind::Action(GameAction::Practice),
            },
            MenuEntry {
                hotkey: '4',
                label: "Record Single",
                detail: if songs == 0 {
                    "no songs written".into()
                } else if signed {
                    format!("${} — label presses", single_cost)
                } else {
                    format!("${} + pressing", single_cost)
                },
                enabled: game.band.can_record_single() && game.player.can_afford(single_min),
                kind: MenuKind::RecordSingle,
            },
            MenuEntry {
                hotkey: '5',
                label: "Record Album",
                detail: if songs < constants::MIN_ALBUM_SONGS as usize {
                    format!("{}/{} songs", songs, constants::MIN_ALBUM_SONGS)
                } else if signed {
                    format!("${} — label presses", album_cost)
                } else {
                    format!("${} + pressing", album_cost)
                },
                enabled: game.band.can_record_album() && game.player.can_afford(album_min),
                kind: MenuKind::RecordAlbum,
            },
            MenuEntry {
                hotkey: '6',
                label: "Play a Gig",
                detail: if game.player.energy < 30 { "too tired".into() } else { "venue picker".into() },
                enabled: game.player.energy >= 30,
                kind: MenuKind::Gig,
            },
            MenuEntry {
                hotkey: '7',
                label: "Go on Tour",
                detail: if game.band.fame < 25 {
                    "needs 25 fame".into()
                } else if game.player.energy < 40 {
                    "too tired".into()
                } else {
                    "region picker".into()
                },
                enabled: game.band.fame >= 25 && game.player.energy >= 40,
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
                    format!("{} release{}", releases, if releases == 1 { "" } else { "s" })
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

    // --- Logging ---

    fn push_log(&mut self, kind: LogKind, text: impl Into<String>) {
        self.log.push(LogEntry { kind, text: text.into() });
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
    fn dispatch(&mut self, action: GameAction) {
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
            Screen::MarketingRelease { .. } => self.handle_marketing_release_key(key),
            Screen::MarketingCampaign { .. } => self.handle_marketing_campaign_key(key),
            Screen::File { .. } => self.handle_file_key(key),
            Screen::GameOver => self.should_exit = true,
            Screen::VenuePicker { .. } => self.handle_venue_picker_key(key),
            Screen::RegionPicker { .. } => self.handle_region_picker_key(key),
            Screen::PressingPicker { .. } => self.handle_pressing_picker_key(key),
        }
    }

    fn handle_setup_key(&mut self, key: KeyEvent) {
        let Screen::Setup { field } = self.screen else { return };

        if key.code == KeyCode::Esc {
            self.should_exit = true;
            return;
        }

        match field {
            SetupField::Name | SetupField::BandName => {
                let input = match field {
                    SetupField::Name => &mut self.name_input,
                    _ => &mut self.band_input,
                };
                match key.code {
                    KeyCode::Char(c) if input.len() < INPUT_MAX_LEN => input.push(c),
                    KeyCode::Backspace => {
                        input.pop();
                    }
                    KeyCode::Enter if !input.trim().is_empty() => {
                        let next = if field == SetupField::Name {
                            SetupField::BandName
                        } else {
                            SetupField::Genre
                        };
                        self.screen = Screen::Setup { field: next };
                    }
                    _ => {}
                }
            }
            SetupField::Genre => {
                let count = MusicGenre::ALL.len();
                match key.code {
                    KeyCode::Up | KeyCode::Char('k') => {
                        self.genre_selected = self.genre_selected.checked_sub(1).unwrap_or(count - 1);
                    }
                    KeyCode::Down | KeyCode::Char('j') => {
                        self.genre_selected = (self.genre_selected + 1) % count;
                    }
                    KeyCode::Enter => self.finish_setup(),
                    _ => {}
                }
            }
        }
    }

    /// Hand the chosen identity to the game and start playing.
    fn finish_setup(&mut self) {
        let name = self.name_input.trim().to_string();
        let band = self.band_input.trim().to_string();
        let genre = MusicGenre::ALL[self.genre_selected.min(MusicGenre::ALL.len() - 1)].clone();
        let genre_name = genre.name();
        self.game.initialize_player(&name, &band, genre);
        self.push_log(
            LogKind::Ui,
            format!(
                "Welcome to {}, {}. Make '{}' the biggest name in {}.",
                constants::STARTING_YEAR, name, band, genre_name
            ),
        );
        self.push_log(
            LogKind::Ui,
            "Tip: hotkeys act instantly — V reviews deal offers, M runs marketing.",
        );
        self.screen = Screen::Main;
    }

    fn handle_main_key(&mut self, key: KeyEvent) {
        let entries = self.menu_entries();
        match key.code {
            KeyCode::Up | KeyCode::Char('k') => {
                self.menu_selected = self.menu_selected.checked_sub(1).unwrap_or(entries.len() - 1);
            }
            KeyCode::Down | KeyCode::Char('j') => {
                self.menu_selected = (self.menu_selected + 1) % entries.len();
            }
            KeyCode::Enter => {
                let kind = entries[self.menu_selected].kind.clone();
                self.activate(kind);
            }
            KeyCode::Char(c) => {
                let c = c.to_ascii_lowercase();
                if let Some(entry) = entries.iter().find(|e| e.hotkey == c) {
                    let kind = entry.kind.clone();
                    self.activate(kind);
                }
            }
            _ => {}
        }
    }

    fn activate(&mut self, kind: MenuKind) {
        match kind {
            MenuKind::Action(action) => self.dispatch(action),
            MenuKind::RecordSingle => self.open_pressing_picker(ReleaseType::Single),
            MenuKind::RecordAlbum => self.open_pressing_picker(ReleaseType::Album),
            MenuKind::Deals => {
                if self.game.pending_deal_offers.is_empty() {
                    self.push_log(LogKind::Ui, "No deal offers on the table right now.");
                } else {
                    self.screen = Screen::Deals { selected: 0, detail: false };
                }
            }
            MenuKind::SupportTour => {
                if self.game.pending_support_offer.is_some() {
                    self.screen = Screen::SupportOffer;
                } else {
                    self.push_log(LogKind::Ui, "No support slots on offer — get noticed by the bigger acts first.");
                }
            }
            MenuKind::Marketing => {
                if self.game.band.current_deal().is_some() {
                    self.push_log(LogKind::Ui, "Promotion is your label's job — their people are already on it.");
                } else if self.marketing_targets().is_empty() {
                    self.push_log(LogKind::Ui, "Record something first — there's nothing to promote.");
                } else {
                    self.screen = Screen::MarketingRelease { selected: 0 };
                }
            }
            MenuKind::Gig => {
                if self.game.player.energy < 30 {
                    self.push_log(LogKind::Ui, "You're too tired to perform!");
                } else {
                    self.screen = Screen::VenuePicker { selected: 0 };
                }
            }
            MenuKind::GoOnTour => {
                if self.game.player.energy < 40 {
                    self.push_log(LogKind::Ui, "You're too tired to go on tour!");
                } else if self.game.band.fame < 25 {
                    self.push_log(LogKind::Ui, "You need more fame before promoters will book a tour!");
                } else {
                    self.screen = Screen::RegionPicker { selected: 0 };
                }
            }
            MenuKind::Save => {
                self.screen = Screen::File { mode: FileMode::Save, input: String::new() };
            }
            MenuKind::Load => {
                self.screen = Screen::File { mode: FileMode::Load, input: String::new() };
            }
            MenuKind::Quit => self.dispatch(GameAction::Quit),
        }
    }

    fn handle_deals_key(&mut self, key: KeyEvent) {
        let Screen::Deals { selected, detail } = self.screen else { return };
        let count = self.game.pending_deal_offers.len();
        if count == 0 {
            self.screen = Screen::Main;
            return;
        }

        match key.code {
            KeyCode::Esc => {
                self.screen = if detail {
                    Screen::Deals { selected, detail: false }
                } else {
                    Screen::Main
                };
            }
            KeyCode::Up | KeyCode::Char('k') if !detail => {
                let selected = selected.checked_sub(1).unwrap_or(count - 1);
                self.screen = Screen::Deals { selected, detail };
            }
            KeyCode::Down | KeyCode::Char('j') if !detail => {
                self.screen = Screen::Deals { selected: (selected + 1) % count, detail };
            }
            KeyCode::Enter if !detail => {
                self.screen = Screen::Deals { selected, detail: true };
            }
            KeyCode::Char('a') | KeyCode::Char('A') => {
                self.screen = Screen::Main;
                self.dispatch(GameAction::AcceptDeal(selected));
            }
            KeyCode::Char('r') | KeyCode::Char('R') => {
                self.dispatch(GameAction::RejectDeal(selected));
                let remaining = self.game.pending_deal_offers.len();
                self.screen = if remaining == 0 {
                    Screen::Main
                } else {
                    Screen::Deals { selected: selected.min(remaining - 1), detail: false }
                };
            }
            _ => {}
        }
    }

    fn handle_support_offer_key(&mut self, key: KeyEvent) {
        if self.game.pending_support_offer.is_none() {
            self.screen = Screen::Main;
            return;
        }
        match key.code {
            KeyCode::Esc => self.screen = Screen::Main,
            KeyCode::Char('a') | KeyCode::Char('A') | KeyCode::Enter => {
                self.screen = Screen::Main;
                self.dispatch(GameAction::AcceptSupportTour);
            }
            KeyCode::Char('r') | KeyCode::Char('R') => {
                self.screen = Screen::Main;
                self.dispatch(GameAction::DeclineSupportTour);
            }
            _ => {}
        }
    }

    fn handle_marketing_release_key(&mut self, key: KeyEvent) {
        let Screen::MarketingRelease { selected } = self.screen else { return };
        let targets = self.marketing_targets();
        if targets.is_empty() {
            self.screen = Screen::Main;
            return;
        }

        match key.code {
            KeyCode::Esc => self.screen = Screen::Main,
            KeyCode::Up | KeyCode::Char('k') => {
                let selected = selected.checked_sub(1).unwrap_or(targets.len() - 1);
                self.screen = Screen::MarketingRelease { selected };
            }
            KeyCode::Down | KeyCode::Char('j') => {
                self.screen = Screen::MarketingRelease { selected: (selected + 1) % targets.len() };
            }
            KeyCode::Enter => {
                let target = &targets[selected.min(targets.len() - 1)];
                self.screen = Screen::MarketingCampaign {
                    release_id: target.id,
                    release_name: target.name.clone(),
                    selected: 0,
                };
            }
            _ => {}
        }
    }

    fn handle_marketing_campaign_key(&mut self, key: KeyEvent) {
        let Screen::MarketingCampaign { release_id, selected, .. } = self.screen else { return };
        let count = MarketingCampaignType::ALL.len();

        match key.code {
            KeyCode::Esc => self.screen = Screen::MarketingRelease { selected: 0 },
            KeyCode::Up | KeyCode::Char('k') => {
                if let Screen::MarketingCampaign { selected, .. } = &mut self.screen {
                    *selected = selected.checked_sub(1).unwrap_or(count - 1);
                }
            }
            KeyCode::Down | KeyCode::Char('j') => {
                if let Screen::MarketingCampaign { selected, .. } = &mut self.screen {
                    *selected = (*selected + 1) % count;
                }
            }
            KeyCode::Enter => {
                let campaign = MarketingCampaignType::ALL[selected.min(count - 1)];
                self.screen = Screen::Main;
                self.dispatch(GameAction::StartMarketingCampaign(release_id, campaign));
            }
            _ => {}
        }
    }

    fn handle_file_key(&mut self, key: KeyEvent) {
        let Screen::File { mode, input } = &mut self.screen else { return };

        match key.code {
            KeyCode::Char(c) if input.len() < 40 => input.push(c),
            KeyCode::Backspace => {
                input.pop();
            }
            KeyCode::Esc => self.screen = Screen::Main,
            KeyCode::Enter => {
                let mode = *mode;
                let path = if input.trim().is_empty() {
                    SAVE_FILE_DEFAULT.to_string()
                } else {
                    input.trim().to_string()
                };
                self.screen = Screen::Main;
                match mode {
                    FileMode::Save => match self.game.save_game(&path) {
                        Ok(()) => self.push_log(LogKind::Ui, format!("💾 Game saved to {}.", path)),
                        Err(e) => self.push_log(LogKind::Error, format!("❌ Save failed: {}", e)),
                    },
                    FileMode::Load => match Game::load_game(&path) {
                        Ok(loaded) => {
                            self.game = loaded;
                            self.push_log(LogKind::Ui, format!("📂 Game loaded from {}.", path));
                            if self.game.is_game_over() {
                                self.screen = Screen::GameOver;
                            }
                        }
                        Err(e) => self.push_log(LogKind::Error, format!("❌ Load failed: {}", e)),
                    },
                }
            }
            _ => {}
        }
    }

    /// A signed band's label decides the run; an indie band picks one.
    fn open_pressing_picker(&mut self, release_type: ReleaseType) {
        if self.game.band.current_deal().is_some() {
            let action = match release_type {
                ReleaseType::Single => GameAction::RecordSingle { pressing: None },
                ReleaseType::Album => GameAction::RecordAlbum { pressing: None },
            };
            self.dispatch(action);
        } else {
            self.screen = Screen::PressingPicker { release_type, selected: 0 };
        }
    }

    fn handle_pressing_picker_key(&mut self, key: KeyEvent) {
        let Screen::PressingPicker { release_type, selected } = self.screen else { return };
        let count = PRESSING_TIERS.len();
        match key.code {
            KeyCode::Esc => self.screen = Screen::Main,
            KeyCode::Up | KeyCode::Char('k') => {
                let selected = selected.checked_sub(1).unwrap_or(count - 1);
                self.screen = Screen::PressingPicker { release_type, selected };
            }
            KeyCode::Down | KeyCode::Char('j') => {
                self.screen = Screen::PressingPicker { release_type, selected: (selected + 1) % count };
            }
            KeyCode::Enter => {
                self.screen = Screen::Main;
                let action = match release_type {
                    ReleaseType::Single => GameAction::RecordSingle { pressing: Some(selected) },
                    ReleaseType::Album => GameAction::RecordAlbum { pressing: Some(selected) },
                };
                self.dispatch(action);
            }
            _ => {}
        }
    }

    fn handle_venue_picker_key(&mut self, key: KeyEvent) {
        let Screen::VenuePicker { selected } = self.screen else { return };
        let count = self.game.world.venues.len();
        match key.code {
            KeyCode::Esc => self.screen = Screen::Main,
            KeyCode::Up | KeyCode::Char('k') => {
                let selected = selected.checked_sub(1).unwrap_or(count - 1);
                self.screen = Screen::VenuePicker { selected };
            }
            KeyCode::Down | KeyCode::Char('j') => {
                self.screen = Screen::VenuePicker { selected: (selected + 1) % count };
            }
            KeyCode::Enter => {
                let venue = &self.game.world.venues[selected];
                if venue.prestige > self.game.band.fame.saturating_add(20) {
                    self.push_log(LogKind::Error, format!("❌ '{}' is out of your league! Get more famous first.", venue.name));
                } else {
                    self.screen = Screen::Main;
                    self.dispatch(GameAction::Gig(selected));
                }
            }
            _ => {}
        }
    }

    fn handle_region_picker_key(&mut self, key: KeyEvent) {
        let Screen::RegionPicker { selected } = self.screen else { return };
        let sorted_regions = self.game.get_sorted_regions();
        let count = sorted_regions.len();
        match key.code {
            KeyCode::Esc => self.screen = Screen::Main,
            KeyCode::Up | KeyCode::Char('k') => {
                let selected = selected.checked_sub(1).unwrap_or(count - 1);
                self.screen = Screen::RegionPicker { selected };
            }
            KeyCode::Down | KeyCode::Char('j') => {
                self.screen = Screen::RegionPicker { selected: (selected + 1) % count };
            }
            KeyCode::Enter => {
                let (country_key, _, region_name, _, _, fame_req) = &sorted_regions[selected];
                if self.game.band.fame < *fame_req {
                    self.push_log(LogKind::Error, format!("❌ Your band needs at least {} fame to tour '{}'.", fame_req, region_name));
                } else {
                    // Check if player can afford the tour cost to give clear feedback
                    let tier_name = if self.game.band.fame < 35 {
                        "local"
                    } else if self.game.band.fame < 60 {
                        "regional"
                    } else if self.game.band.fame < 80 {
                        "national"
                    } else {
                        "international"
                    };
                    let country_travel_mult = match country_key.as_str() {
                        "united_states" => 1.5,
                        "united_kingdom" => 0.8,
                        "europe" => 1.2,
                        "japan" => 1.0,
                        "australia" => 1.4,
                        _ => 1.0,
                    };
                    if let Some(touring_costs) = self.game.data_files.markets_data.market_modifiers.touring_costs.get(tier_name) {
                        let cost = (touring_costs.base_cost_per_show as f32 * country_travel_mult) as i32;
                        if !self.game.player.can_afford(cost) {
                            self.push_log(LogKind::Error, format!("❌ You need ${} to finance this tour!", cost));
                            return;
                        }
                    }
                    self.screen = Screen::Main;
                    self.dispatch(GameAction::GoOnTour(selected));
                }
            }
            _ => {}
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn press(app: &mut App, code: KeyCode) {
        app.handle_setup_key(KeyEvent::new(code, KeyModifiers::empty()));
    }

    fn type_text(app: &mut App, text: &str) {
        for c in text.chars() {
            press(app, KeyCode::Char(c));
        }
    }

    #[test]
    fn setup_can_found_a_band_in_every_genre() {
        for (index, genre) in MusicGenre::ALL.iter().enumerate() {
            let mut app = App::new(Game::new().expect("data files present"));

            type_text(&mut app, "Ray");
            press(&mut app, KeyCode::Enter);
            type_text(&mut app, "The Rayguns");
            press(&mut app, KeyCode::Enter);
            for _ in 0..index {
                press(&mut app, KeyCode::Down);
            }
            press(&mut app, KeyCode::Enter);

            assert!(matches!(app.screen, Screen::Main), "setup should end on the main screen");
            assert_eq!(app.game.band.genre, *genre);
            assert_eq!(app.game.band.name, "The Rayguns");
        }
    }
}
