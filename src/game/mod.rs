mod actions;
pub mod band;
mod economy;
pub mod events;
mod events_apply;
pub mod genre;
pub mod music;
pub mod player;
#[cfg(test)]
mod sim; // Track D balance lab: bot-driven career sims, tests only.
pub mod timeline;
mod turn;
pub mod world;

use crate::data::constants;
use crate::data_loader::GameDataFiles;
use crate::game::music::*; // For Song, Release, ReleaseType, MarketingCampaignType, ActiveMarketingCampaign
use band::Band;
use events::EventManager;
use player::Player;
use rand::rngs::StdRng;
use rand::{Rng, SeedableRng};
use serde::{Deserialize, Serialize};
use std::fs::File;
use std::io::{Read, Write};
use timeline::MusicTimeline;
use world::{GameWorld, PotentialDealOffer};

// Quality calculation constants
const QUALITY_BASE_SONGWRITING: u8 = 30;
const QUALITY_SONGWRITING_MAX_BONUS_PLAYER_STATS: u8 = 25;
const QUALITY_SONGWRITING_RANDOM_VARIATION: u8 = 10;
const QUALITY_BASE_RECORDING: u8 = 30;
const QUALITY_RECORDING_MAX_BONUS_PLAYER_STATS: u8 = 20;
const QUALITY_RECORDING_RANDOM_VARIATION: u8 = 10;

// Sales model constants
const INITIAL_SALES_WINDOW_WEEKS: u32 = 4;
const SALES_QUALITY_WEIGHT: f32 = 2.5;
const SALES_MARKETING_WEIGHT: f32 = 1.8;
const SALES_FAME_WEIGHT: f32 = 1.2;

// Unit economics: a sales score converts into copies people want to buy,
// bounded by how many copies actually exist.
const UNITS_PER_SCORE_POINT: f32 = 10.0;
const INDIE_INCOME_PER_COPY: u32 = 2;
const LABEL_INCOME_PER_COPY: u32 = 3;

// Pressing runs. Independents choose a run and pay setup plus per-copy
// costs; a label presses to the size of its network and your name.
pub const PRESSING_TIERS: [(&str, u32); 4] = [
    ("Garage run", 500),
    ("Club run", 2_000),
    ("Regional run", 10_000),
    ("National run", 50_000),
];
const PRESSING_SETUP_SINGLE: f32 = 25.0;
const PRESSING_SETUP_ALBUM: f32 = 100.0;
const PRESSING_PER_COPY_SINGLE: f32 = 0.10;
const PRESSING_PER_COPY_ALBUM: f32 = 0.50;
const LABEL_PRESSING_PER_REACH: u32 = 100;
const LABEL_PRESSING_PER_FAME: u32 = 50;

// Distribution model: how much of a release's potential audience you can
// actually reach. Labels bring their market_reach; independents are capped
// by their own fame.
const INDIE_REACH_FLOOR: f32 = 0.15;

// Support tours: bigger acts occasionally want you as their opener.
const SUPPORT_OFFER_MIN_FAME: u8 = 5;
const SUPPORT_OFFER_FAME_GAP: u8 = 10;
const SUPPORT_OFFER_CHANCE: f64 = 0.06;
const SUPPORT_OFFER_LIFETIME_WEEKS: u32 = 3;

// Record deals stay on the table about a month — one scouting cycle — so
// a slate the player sits on clears just as labels next come looking, and
// ignoring an offer can never silence the deal stream for good.
const DEAL_OFFER_LIFETIME_WEEKS: u32 = 4;

const PLAYER_MARKET_IMPACT_THRESHOLD_SALES_SCORE: u32 = 600;
const PLAYER_MARKET_IMPACT_GENRE_MOD_BONUS: f32 = 0.05;
const PLAYER_MARKET_IMPACT_DEMAND_BONUS: u8 = 1;

// Live fame ceilings: a gig only reaches the crowd in the room, and without
// records word of mouth stalls. Gigs and tours raise fame no further than
// the smaller of the venue's ceiling and the catalog cap.
const VENUE_FAME_HEADROOM: u8 = 15;
const LIVE_FAME_BASE_CAP: u8 = 35;
const LIVE_FAME_PER_SINGLE: u8 = 6;
const LIVE_FAME_PER_ALBUM: u8 = 12;

// Fame fades when the band disappears from view: no shows, no tour, and
// nothing new on the shelves. One quiet week is forgiven.
const IDLE_GRACE_WEEKS: u32 = 1;
const IDLE_FAME_DECAY_PER_WEEK: u8 = 1;
pub const BREAK_WEEKS: u32 = 4;

// Era-genre fit: past these bounds the era clearly loves or has abandoned
// the band's sound, and the press says so — once per swing, not every week.
// (A genre missing from an era's table reads as out of fashion at 0.85.)
const GENRE_TREND_HOT: f32 = 1.15;
const GENRE_TREND_COLD: f32 = 0.85;

// Determinism: everything after seed selection derives from world_seed, so a
// seed replays a whole career. Two independent streams share the splitmix64
// key derivation (see `advance_week_events` for the world stream): the world
// stream evolves the scene, the action stream feeds every roll the player's
// own actions make. Salting the action keys keeps the streams uncorrelated —
// your tour luck never mirrors next week's scene news.
const ACTION_STREAM_SALT: u64 = 0x243F_6A88_85A3_08D3; // π's fraction bits: arbitrary, fixed forever
// Setup rolls (bandmate names) draw from a reserved pre-game week, so they
// can never replay week 1's action stream.
const SETUP_STREAM_WEEK: u64 = 0;

#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum GameAction {
    LazeAround,
    WriteSongs,
    Practice,
    RecordSingle { pressing: Option<usize> },
    RecordAlbum { pressing: Option<usize> },
    Gig(usize),
    GoOnTour(usize),
    TakeBreak,
    VisitDoctor,
    AcceptDeal(usize),
    RejectDeal(usize),
    AcceptSupportTour,
    DeclineSupportTour,
    StartMarketingCampaign(u32, MarketingCampaignType), // release_id, campaign_type
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
    /// The last era-fit verdict the press reported on the band's genre
    /// (-1 cold, 0 unremarkable, +1 hot) — the news speaks only on change.
    #[serde(default)]
    pub genre_trend_reported: i8,
    pub week: u32,
    pub game_over: bool,
    pub next_song_id: u32,
    pub next_release_id: u32,
    pub just_released_music: Vec<Release>, // Stores releases for their initial sales window
    /// Messages produced while processing the last turn, drained by the UI.
    #[serde(skip)]
    pub turn_log: Vec<String>,
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
            genre_trend_reported: 0,
            week: 1,
            game_over: false,
            next_song_id: 0,
            next_release_id: 0,
            just_released_music: Vec::new(),
            turn_log,
        })
    }

    pub(super) fn log(&mut self, message: impl Into<String>) {
        self.turn_log.push(message.into());
    }

    pub fn take_turn_log(&mut self) -> Vec<String> {
        std::mem::take(&mut self.turn_log)
    }

    /// The action-stream RNG for a given week: the same splitmix64 key
    /// derivation the world stream uses in `advance_week_events`, applied to
    /// the salted seed (see `ACTION_STREAM_SALT`). Derived on demand, never
    /// stored — saves carry no RNG state, and a loaded game rolls exactly
    /// what the unsaved one would have.
    fn action_rng_for_week(&self, week: u64) -> StdRng {
        let mut key = (self.world_seed ^ ACTION_STREAM_SALT)
            .wrapping_add(week)
            .wrapping_mul(0x9E3779B97F4A7C15);
        key = (key ^ (key >> 30)).wrapping_mul(0xBF58476D1CE4E5B8);
        key = (key ^ (key >> 27)).wrapping_mul(0x94D049BB133111EB);
        key ^= key >> 31;
        StdRng::seed_from_u64(key)
    }

    /// Every roll made while resolving the current turn draws from this one
    /// stream, in order: the action itself, then the week's random event,
    /// then offer generation. Turn-consuming actions move the calendar, so
    /// consecutive turns get fresh streams; the rare same-week paperwork
    /// action (rejecting two deals in one sitting) rereads the week's stream,
    /// which is deterministic and harmless.
    pub(super) fn action_rng(&self) -> StdRng {
        self.action_rng_for_week(self.week as u64)
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
        let mut rng = self.action_rng_for_week(SETUP_STREAM_WEEK);
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

#[cfg(test)]
mod tests;
