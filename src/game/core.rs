use std::fs::File;
use std::io::{Read, Write};

use rand::SeedableRng;
use rand::rngs::StdRng;
use serde::{Deserialize, Serialize};

use crate::data_loader::GameDataFiles;
use crate::game::actions::TourRig;
use crate::game::band::{self, Band};
use crate::game::constants;
use crate::game::events::EventManager;
use crate::game::genre;
use crate::game::music::{MarketingCampaignType, Release};
use crate::game::player::{LifestyleTier, Player};
use crate::game::shows::TourReport;
use crate::game::timeline::MusicTimeline;
use crate::game::world::{GameWorld, PotentialDealOffer};

#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum GameAction {
    LazeAround,
    WriteSongs,
    Practice,
    RecordSingle {
        pressing: Option<usize>,
    },
    RecordAlbum {
        pressing: Option<usize>,
    },
    Gig(usize),
    /// Region index, chosen rig, and tour length in weeks — all explicit
    /// player choices, quoted before booking (design §A, M1). Fame never
    /// selects any of these; it only gates which are available.
    GoOnTour(usize, TourRig, u8),
    TakeBreak,
    VisitDoctor,
    AcceptDeal(usize),
    RejectDeal(usize),
    AcceptSupportTour,
    DeclineSupportTour,
    StartMarketingCampaign(u32, MarketingCampaignType), // release_id, campaign_type
    /// Move to a different lifestyle tier — always the player's call,
    /// instant, no week consumed (v0.7 design §B).
    ChangeLifestyle(LifestyleTier),
    Quit,
}

/// An invitation from a bigger act to open on their tour.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SupportTourOffer {
    pub host_band: String,
    pub host_fame: u8,
    pub weeks: u32,
    pub pay: u32,
    pub fame_gain: u8,
    pub expires_week: u32,
}

/// The one deliberate use of ambient entropy: choosing a world seed when
/// ROCKER_SEED doesn't dictate one. Every roll after this derives from it.
fn default_seed() -> u64 {
    rand::random::<u64>()
}

#[derive(Serialize, Deserialize)]
pub struct Game {
    #[serde(default = "default_seed")]
    pub world_seed: u64,
    pub player: Player,
    pub band: Band,
    pub world: GameWorld,
    pub events: EventManager,
    pub timeline: MusicTimeline,
    #[serde(skip)]
    pub data_files: GameDataFiles,
    pub pending_deal_offers: Vec<PotentialDealOffer>,
    #[serde(default)]
    pub pending_support_offer: Option<SupportTourOffer>,
    #[serde(default)]
    pub regional_fame: std::collections::HashMap<String, u8>,
    /// Consecutive weeks with no public activity (no shows, nothing on sale).
    #[serde(default)]
    pub idle_streak: u32,
    /// Consecutive weeks fame has actually been decaying (past grace). The
    /// ramp (−1, −2, −3, −4, then −5 flat) is keyed to this clock, not to
    /// weeks-past-grace, so a shrinking grace tier mid-decline cannot skip
    /// steps (§C — The ramp). Resets with `idle_streak`.
    #[serde(default)]
    pub decay_streak: u32,
    /// The last era-fit verdict the press reported on the band's genre
    /// (-1 cold, 0 unremarkable, +1 hot) — the news speaks only on change.
    #[serde(default)]
    pub genre_trend_reported: i8,
    /// Consecutive weeks spent writing songs (v0.6 §A).
    #[serde(default)]
    pub writing_streak: u32,
    pub week: u32,
    pub game_over: bool,
    pub next_song_id: u32,
    pub next_release_id: u32,
    pub just_released_music: Vec<Release>, // Stores releases for their initial sales window
    /// The most recent gig or tour's per-show report (design §B — the tour
    /// report). A one-off gig produces the same single-row report a tour
    /// would. Serde-defaulted so old saves load with no report on hand.
    #[serde(default)]
    pub last_tour_report: Option<TourReport>,
    /// Messages produced while processing the last turn, drained by the UI.
    #[serde(skip)]
    pub turn_log: Vec<String>,
    /// Whether the band has reached rockstar status. Set once when thresholds
    /// are met; the game continues indefinitely after.
    #[serde(default)]
    pub rockstar_achieved: bool,
}

impl Game {
    pub fn new() -> Result<Self, Box<dyn std::error::Error>> {
        GameDataFiles::validate_data_files()?;
        let data_files = GameDataFiles::load()?;

        let world_seed = std::env::var("ROCKER_SEED")
            .ok()
            .and_then(|s| s.parse::<u64>().ok())
            .unwrap_or_else(default_seed);

        let mut init_rng = StdRng::seed_from_u64(world_seed);
        let world = GameWorld::new(&data_files, &mut init_rng);

        let mut turn_log = Vec::new();
        turn_log.push(format!("🌱 World Seed: {}", world_seed));

        Ok(Self {
            world_seed,
            player: Player::default(),
            band: Band::default(),
            world,
            events: EventManager::new(),
            timeline: MusicTimeline::new(&data_files),
            data_files,
            pending_deal_offers: Vec::new(),
            pending_support_offer: None,
            regional_fame: std::collections::HashMap::new(),
            idle_streak: 0,
            decay_streak: 0,
            genre_trend_reported: 0,
            writing_streak: 0,
            week: 1,
            game_over: false,
            next_song_id: 0,
            next_release_id: 0,
            just_released_music: Vec::new(),
            last_tour_report: None,
            turn_log,
            rockstar_achieved: false,
        })
    }

    pub(super) fn log(&mut self, message: impl Into<String>) {
        self.turn_log.push(message.into());
    }

    pub fn take_turn_log(&mut self) -> Vec<String> {
        std::mem::take(&mut self.turn_log)
    }

    pub fn initialize_player(
        &mut self,
        player_name: &str,
        band_name: &str,
        genre: genre::MusicGenre,
    ) {
        self.player.name = player_name.to_string();
        self.band.name = band_name.to_string();
        self.band.genre = genre;
        self.player.money = 500; // Starting cash in 1970

        // Bandmates are part of the seed's identity, like the scene itself.
        let mut rng = self.action_rng_for_week(constants::SETUP_STREAM_WEEK);
        self.band.members = vec![
            band::BandMember {
                name: self.data_files.random_band_member_name(&mut rng),
                instrument: band::Instrument::Guitar,
                skill: 25,
                loyalty: 75,
                drug_problem: false,
            },
            band::BandMember {
                name: self.data_files.random_band_member_name(&mut rng),
                instrument: band::Instrument::Bass,
                skill: 20,
                loyalty: 80,
                drug_problem: false,
            },
            band::BandMember {
                name: self.data_files.random_band_member_name(&mut rng),
                instrument: band::Instrument::Drums,
                skill: 30,
                loyalty: 70,
                drug_problem: false,
            },
        ];
    }

    pub fn save_game(&self, file_path: &str) -> Result<(), Box<dyn std::error::Error>> {
        let json_string = serde_json::to_string_pretty(self)?;
        let mut file = File::create(file_path)?;
        file.write_all(json_string.as_bytes())?;
        Ok(())
    }

    pub fn load_game(file_path: &str) -> Result<Self, Box<dyn std::error::Error>> {
        let mut file = File::open(file_path)?;
        let mut json_string = String::new();
        file.read_to_string(&mut json_string)?;

        let mut loaded_game: Game = serde_json::from_str(&json_string)?;

        loaded_game.data_files = GameDataFiles::load()?;

        Ok(loaded_game)
    }
}
