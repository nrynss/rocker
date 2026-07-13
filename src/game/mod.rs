mod actions;
pub mod band;
mod economy;
pub mod events;
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
        genre: world::MusicGenre,
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
mod tests {
    use super::*;

    fn test_game() -> Game {
        Game::new().expect("data files present")
    }

    fn test_release(id: u32, release_type: ReleaseType) -> Release {
        Release {
            id,
            name: format!("Test Release {id}"),
            release_type,
            release_quality: 50,
            week_released: 0,
            songs_involved_quality_avg: 50,
            active_marketing: Vec::new(),
            marketing_level_achieved: 0,
            initial_sales_score: 0,
            total_income_generated: 0,
            genre: None,
            copies_pressed: 0,
            copies_sold: 0,
        }
    }

    /// The biggest venue whose door policy admits the band right now.
    fn best_open_venue(game: &Game) -> usize {
        (0..game.world.venues.len())
            .filter(|&i| game.world.venues[i].prestige <= game.band.fame.saturating_add(20))
            .max_by_key(|&i| game.world.venues[i].capacity)
            .expect("at least one venue is always open")
    }

    #[test]
    fn gigging_alone_cannot_make_you_a_star() {
        let mut game = test_game();
        game.band.fame = 0;

        for _ in 0..300 {
            game.player.energy = 100;
            let venue = best_open_venue(&game);
            game.action_play_gig(venue).expect("gig should succeed");
        }

        assert_eq!(
            game.band.fame, LIVE_FAME_BASE_CAP,
            "with no records, live shows should stall at the base cap"
        );
    }

    #[test]
    fn records_raise_the_live_fame_cap() {
        let mut game = test_game();
        game.band.fame = LIVE_FAME_BASE_CAP;
        game.player.energy = 100;

        let venue = best_open_venue(&game);
        game.action_play_gig(venue).expect("gig should succeed");
        assert_eq!(
            game.band.fame, LIVE_FAME_BASE_CAP,
            "at the cap, another gig adds nothing"
        );

        game.band
            .albums_released
            .push(test_release(1, ReleaseType::Album));
        game.band
            .singles_released
            .push(test_release(2, ReleaseType::Single));
        game.player.energy = 100;
        game.action_play_gig(venue).expect("gig should succeed");
        assert!(
            game.band.fame > LIVE_FAME_BASE_CAP,
            "records should lift the live ceiling"
        );
    }

    #[test]
    fn an_outgrown_venue_adds_no_fame() {
        let mut game = test_game();
        for id in 0..6 {
            game.band
                .albums_released
                .push(test_release(id, ReleaseType::Album));
        }
        game.band.fame = 30; // past the pub's ceiling of prestige 10 + headroom 15
        game.player.energy = 100;

        let smallest = (0..game.world.venues.len())
            .min_by_key(|&i| game.world.venues[i].capacity)
            .expect("venues exist");
        game.action_play_gig(smallest).expect("gig should succeed");

        assert_eq!(game.band.fame, 30, "an outgrown stage draws no new fans");
    }

    fn test_deal(market_reach: u8, royalty_rate: f32) -> band::RecordDeal {
        band::RecordDeal {
            label_name: "Test Records".to_string(),
            label_tier: "Major".to_string(),
            advance: 0,
            royalty_rate,
            albums_required: 2,
            albums_delivered: 0,
            market_reach,
        }
    }

    #[test]
    fn unknown_indie_acts_reach_almost_nobody() {
        let mut game = test_game();
        game.band.record_deal = None;
        let release = test_release(1, ReleaseType::Single);

        game.band.fame = 5;
        let (unknown, _, _) = game.calculate_release_outcome(300, &release);
        game.band.fame = 95;
        let (famous, _, _) = game.calculate_release_outcome(300, &release);

        assert!(
            famous > unknown * 3,
            "a famous indie act should reach a far larger audience: {unknown} vs {famous}"
        );
    }

    #[test]
    fn label_out_earns_indie_at_low_fame_but_not_at_high_fame() {
        let mut game = test_game();
        game.band.fame = 10;
        let release = test_release(1, ReleaseType::Single);

        game.band.record_deal = None;
        let (indie_low, _, _) = game.calculate_release_outcome(300, &release);
        game.band.record_deal = Some(test_deal(90, 0.12));
        let (label_low, _, _) = game.calculate_release_outcome(300, &release);
        assert!(
            label_low > indie_low,
            "an unknown band should earn more through a label: label {label_low} vs indie {indie_low}"
        );

        game.band.fame = 95;
        game.band.record_deal = None;
        let (indie_high, _, _) = game.calculate_release_outcome(300, &release);
        assert!(
            indie_high > label_low * 2,
            "a superstar keeping everything should out-earn a royalty slice: indie {indie_high} vs label {label_low}"
        );
    }

    #[test]
    fn pressing_costs_fall_on_indies_and_labels_press_for_you() {
        let mut game = test_game();
        game.band.record_deal = None;

        let garage = game.pressing_cost(&ReleaseType::Album, PRESSING_TIERS[0].1);
        let national = game.pressing_cost(&ReleaseType::Album, PRESSING_TIERS[3].1);
        assert!(garage > 0, "an indie band pays to press its own records");
        assert!(
            national > garage * 10,
            "a national run costs far more than a garage run: {garage} vs {national}"
        );
        let (copies, cost) = game
            .plan_pressing(&ReleaseType::Album, Some(0))
            .expect("tier 0 exists");
        assert_eq!(copies, PRESSING_TIERS[0].1);
        assert_eq!(cost, garage);

        game.band.record_deal = Some(test_deal(70, 0.10));
        game.band.fame = 40;
        let (label_copies, label_cost) = game
            .plan_pressing(&ReleaseType::Album, None)
            .expect("the label always presses");
        assert_eq!(label_cost, 0, "the label covers pressing when signed");
        assert_eq!(
            label_copies,
            70 * LABEL_PRESSING_PER_REACH + 40 * LABEL_PRESSING_PER_FAME,
            "the run scales with the label's network and the band's name"
        );
    }

    #[test]
    fn a_pressing_can_sell_out() {
        let mut game = test_game();
        game.band.record_deal = None;
        game.band.fame = 60;

        let mut release = test_release(1, ReleaseType::Single);
        release.copies_pressed = 500;
        let (income, units, sold_out) = game.calculate_release_outcome(400, &release);
        assert!(sold_out, "demand should outstrip a garage run");
        assert_eq!(units, 500);
        assert_eq!(income, 500 * INDIE_INCOME_PER_COPY);

        release.copies_pressed = 50_000;
        let (_, units_uncapped, sold_out) = game.calculate_release_outcome(400, &release);
        assert!(!sold_out);
        assert!(units_uncapped > 500, "a bigger run keeps selling");
    }

    #[test]
    fn signed_bands_do_not_run_their_own_marketing() {
        let mut game = test_game();
        game.just_released_music
            .push(test_release(7, ReleaseType::Single));
        game.band.record_deal = Some(test_deal(60, 0.12));

        let err = game
            .action_start_marketing_campaign(7, MarketingCampaignType::BasicPress)
            .unwrap_err();
        assert!(err.contains("job"), "unexpected error: {err}");
    }

    #[test]
    fn idle_weeks_erode_fame_after_a_grace_week() {
        let mut game = test_game();
        game.band.fame = 30;

        game.update_public_visibility(&GameAction::LazeAround, 1);
        assert_eq!(game.band.fame, 30, "the first quiet week is forgiven");

        game.update_public_visibility(&GameAction::LazeAround, 1);
        game.update_public_visibility(&GameAction::LazeAround, 1);
        assert_eq!(
            game.band.fame, 28,
            "every idle week past the grace costs fame"
        );

        game.update_public_visibility(&GameAction::Gig(0), 1);
        assert_eq!(game.idle_streak, 0, "a show resets the idle streak");
    }

    #[test]
    fn a_release_on_the_shelves_keeps_the_band_visible() {
        let mut game = test_game();
        game.band.fame = 30;
        game.just_released_music
            .push(test_release(1, ReleaseType::Single));

        game.update_public_visibility(&GameAction::LazeAround, 5);

        assert_eq!(
            game.band.fame, 30,
            "a record in its sales window counts as visibility"
        );
        assert_eq!(game.idle_streak, 0);
    }

    #[test]
    fn a_hit_release_enters_the_charts_and_a_flop_misses() {
        let mut game = test_game();
        game.initialize_player("Test", "The Tests", world::MusicGenre::Rock);
        // A crowded chart: ten scene records the player has to outsell.
        for i in 0..world::CHART_SIZE {
            game.world.submit_chart_entry(
                format!("Scene Filler {i}"),
                "Scene Band".into(),
                false,
                200,
            );
        }

        // A famous band drops a great record...
        game.band.fame = 80;
        let mut hit = test_release(1, ReleaseType::Single);
        hit.name = "Big Hit".to_string();
        hit.release_quality = 90;
        game.just_released_music.push(hit);
        game.week = INITIAL_SALES_WINDOW_WEEKS; // the sales window has closed
        game.process_music_releases_and_marketing();

        assert!(
            game.world
                .charts
                .iter()
                .any(|e| e.is_player && e.title == "Big Hit"),
            "a high-scoring release should land on the chart"
        );
        assert!(
            game.turn_log
                .iter()
                .any(|m| m.contains("enters the charts at #1")),
            "charting should be reported to the player"
        );

        // ...while a nobody's dud sinks without a trace.
        game.band.fame = 0;
        let mut flop = test_release(2, ReleaseType::Single);
        flop.name = "Total Flop".to_string();
        flop.release_quality = 1;
        flop.week_released = game.week;
        game.just_released_music.push(flop);
        game.week += INITIAL_SALES_WINDOW_WEEKS;
        game.process_music_releases_and_marketing();

        assert!(
            !game.world.charts.iter().any(|e| e.title == "Total Flop"),
            "a flop should not crack a crowded top 10"
        );
    }

    #[test]
    fn a_full_season_of_turns_never_panics() {
        let mut game = test_game();
        game.initialize_player("Test", "The Tests", world::MusicGenre::Rock);
        for i in 0..30 {
            let action = match i % 6 {
                0 => GameAction::WriteSongs,
                1 => GameAction::Gig(0),
                2 => GameAction::LazeAround,
                3 => GameAction::RecordSingle { pressing: Some(0) },
                4 => GameAction::Practice,
                _ => GameAction::TakeBreak,
            };
            // Rejected actions are fine; panics are not.
            let _ = game.process_turn(action);
            game.player.money = game.player.money.max(1_000);
            game.player.energy = 100;
            game.player.health = 100;
        }
    }

    #[test]
    fn a_break_is_a_real_break() {
        let mut game = test_game();
        let week_before = game.week;
        game.player.health = 50;
        game.player.energy = 5;

        game.action_take_break().expect("a break always works");

        assert_eq!(
            game.week,
            week_before + BREAK_WEEKS - 1,
            "the turn itself adds the final week"
        );
        assert_eq!(game.player.health, 80);
        assert_eq!(game.player.energy, constants::MAX_ENERGY);
    }

    #[test]
    fn failed_recording_does_not_eat_songs() {
        let mut game = test_game();
        game.band.unreleased_songs.push(music::Song {
            id: 0,
            name: "Keeper".to_string(),
            songwriting_quality: 50,
        });
        game.player.money = 0;

        let mut rng = StdRng::seed_from_u64(0);
        assert!(game.action_record_single(Some(0), &mut rng).is_err());
        assert_eq!(
            game.band.unreleased_songs.len(),
            1,
            "songs must survive a recording attempt the player cannot afford"
        );
    }

    #[test]
    fn accepting_a_support_tour_pays_and_advances_time() {
        let mut game = test_game();
        game.band.fame = 10;
        game.player.money = 500;
        game.player.energy = 100;
        game.pending_support_offer = Some(SupportTourOffer {
            host_band: "Big Stars".to_string(),
            host_fame: 60,
            weeks: 3,
            pay: 1000,
            fame_gain: 6,
            expires_week: 10,
        });
        let week_before = game.week;

        let mut rng = StdRng::seed_from_u64(0);
        game.action_accept_support_tour(&mut rng)
            .expect("offer should be acceptable");

        assert!(game.pending_support_offer.is_none());
        assert_eq!(game.player.money, 1500);
        assert_eq!(game.week, week_before + 3);
        assert!(game.band.fame >= 16, "fame should include the offered gain");
        assert_eq!(game.player.energy, 65);
    }

    #[test]
    fn declining_a_support_tour_clears_it() {
        let mut game = test_game();
        game.pending_support_offer = Some(SupportTourOffer {
            host_band: "Big Stars".to_string(),
            host_fame: 60,
            weeks: 2,
            pay: 500,
            fame_gain: 4,
            expires_week: 10,
        });

        game.action_decline_support_tour()
            .expect("decline should succeed");
        assert!(game.pending_support_offer.is_none());
        assert!(
            game.action_decline_support_tour().is_err(),
            "no offer left to decline"
        );
    }

    #[test]
    fn support_offers_arrive_when_bigger_acts_exist() {
        let mut game = test_game();
        game.band.fame = 20;
        // Guarantee at least one act big enough to headline over the player.
        game.world.bands[0].fame = 80;

        let mut offered = false;
        let mut rng = StdRng::seed_from_u64(1);
        for week in 1..=200 {
            game.week = week;
            game.update_support_tour_offer(&mut rng);
            if game.pending_support_offer.is_some() {
                offered = true;
                break;
            }
        }
        assert!(
            offered,
            "200 weeks alongside a big act should produce at least one offer"
        );

        let offer = game.pending_support_offer.as_ref().unwrap();
        assert!(offer.host_fame >= game.band.fame + SUPPORT_OFFER_FAME_GAP);
        assert!(offer.pay > 0);
    }

    #[test]
    fn a_release_riding_the_era_outsells_one_against_it() {
        let mut game = test_game();
        game.band.fame = 40;
        game.world.dynamic_genre_modifiers.clear(); // era taste is the only genre input

        let year = game.timeline.get_current_year();
        let era_fit =
            |genre: &world::MusicGenre| game.data_files.era_genre_modifier(year, genre.aliases());
        let hot = world::MusicGenre::ALL
            .iter()
            .max_by(|a, b| era_fit(a).total_cmp(&era_fit(b)))
            .expect("genres exist")
            .clone();
        let cold = world::MusicGenre::ALL
            .iter()
            .min_by(|a, b| era_fit(a).total_cmp(&era_fit(b)))
            .expect("genres exist")
            .clone();
        assert!(
            era_fit(&hot) > era_fit(&cold),
            "the era should actually have tastes"
        );

        let mut on_trend = test_release(1, ReleaseType::Single);
        on_trend.genre = Some(hot);
        let mut against_the_grain = test_release(2, ReleaseType::Single);
        against_the_grain.genre = Some(cold);

        assert!(
            game.calculate_release_sales_score(&on_trend)
                > game.calculate_release_sales_score(&against_the_grain),
            "identical records should sell by the era's tastes"
        );
    }

    #[test]
    fn bands_saved_before_genres_existed_load_as_rock() {
        assert_eq!(Band::default().genre, world::MusicGenre::Rock);

        // A pre-genre save is a Band JSON object with no "genre" key at all.
        let mut saved = serde_json::to_value(Band::default()).expect("bands serialize");
        saved
            .as_object_mut()
            .expect("a band serializes to an object")
            .remove("genre");
        let loaded: Band = serde_json::from_value(saved).expect("old saves must keep loading");
        assert_eq!(loaded.genre, world::MusicGenre::Rock);
    }

    #[test]
    fn the_press_calls_a_hot_genre_once_not_weekly() {
        let mut game = test_game();
        // Rock is the sound of 1970 in the era data — clearly hot.
        game.band.genre = world::MusicGenre::Rock;

        game.process_turn(GameAction::LazeAround)
            .expect("lazing always works");
        game.process_turn(GameAction::LazeAround)
            .expect("lazing always works");

        let mentions = game
            .turn_log
            .iter()
            .filter(|line| line.contains("right scene at the right time"))
            .count();
        assert_eq!(mentions, 1, "the trend is news once, not every week");
    }

    #[test]
    fn the_press_notices_a_genre_the_era_left_behind() {
        let mut game = test_game();
        // Punk is years ahead of 1970's tastes — out of fashion on day one.
        game.band.genre = world::MusicGenre::Punk;

        game.process_turn(GameAction::LazeAround)
            .expect("lazing always works");

        assert!(
            game.turn_log
                .iter()
                .any(|line| line.contains("chasing a different sound")),
            "an off-trend band should hear about it"
        );
    }

    #[test]
    fn stale_support_offers_expire() {
        let mut game = test_game();
        game.pending_support_offer = Some(SupportTourOffer {
            host_band: "Big Stars".to_string(),
            host_fame: 60,
            weeks: 2,
            pay: 500,
            fame_gain: 4,
            expires_week: 3,
        });
        game.week = 5;

        game.update_support_tour_offer(&mut StdRng::seed_from_u64(0));
        assert!(game.pending_support_offer.is_none(), "offers should expire");
        assert!(
            game.turn_log
                .iter()
                .any(|m| m.contains("went to another band")),
            "expiry should be reported"
        );
    }

    /// A pending offer as `check_and_generate_deal_offers` would leave it.
    fn test_deal_offer(game: &Game, expires_week: Option<u32>) -> PotentialDealOffer {
        let label = game.data_files.get_record_labels_data().independent_labels[0].clone();
        PotentialDealOffer {
            label_name: label.name.clone(),
            label_tier: "Independent".to_string(),
            advance: 1_000,
            royalty_rate: 0.12,
            albums_required: 1,
            original_label_data: label,
            expires_week,
        }
    }

    #[test]
    fn ignored_deal_offers_expire_and_the_stream_resumes() {
        let mut game = test_game();
        game.pending_deal_offers = vec![test_deal_offer(&game, Some(8))];
        let unsigned_before = game
            .world
            .bands
            .iter()
            .filter(|b| b.label.is_none())
            .count();

        // Before the deadline the offer stays on the table.
        game.week = 7;
        game.check_and_generate_deal_offers(&mut StdRng::seed_from_u64(0));
        assert_eq!(
            game.pending_deal_offers.len(),
            1,
            "a live offer survives to its deadline"
        );

        // At the deadline it quietly leaves — with a line in the log...
        game.week = 8;
        game.check_and_generate_deal_offers(&mut StdRng::seed_from_u64(0));
        assert!(
            game.pending_deal_offers.is_empty(),
            "an ignored offer should expire"
        );
        let log = game.take_turn_log().join("\n");
        assert!(
            log.contains("interest has cooled"),
            "expiry is told in-fiction, got: {log}"
        );
        // ...and, unlike a rejection, nobody poaches the vacated deal.
        let unsigned_after = game
            .world
            .bands
            .iter()
            .filter(|b| b.label.is_none())
            .count();
        assert_eq!(
            unsigned_before, unsigned_after,
            "expiry must not hand the deal to a scene act"
        );

        // With the slate clear and a catalog worth scouting, the stream
        // resumes on the next 4-week beat instead of staying silent forever.
        game.band.fame = 30;
        game.band
            .singles_released
            .push(test_release(1, ReleaseType::Single));
        let mut resumed = false;
        for attempt in 0..80 {
            game.check_and_generate_deal_offers(&mut StdRng::seed_from_u64(attempt));
            if !game.pending_deal_offers.is_empty() {
                resumed = true;
                break;
            }
        }
        assert!(resumed, "new offers should arrive once the slate is clear");
        assert!(
            game.pending_deal_offers
                .iter()
                .all(|offer| offer.expires_week == Some(game.week + DEAL_OFFER_LIFETIME_WEEKS)),
            "fresh offers should carry a deadline"
        );
    }

    #[test]
    fn deal_offers_from_old_saves_never_expire() {
        // Offers already pending when an old save was written carry no
        // deadline; they stay on the table however late it gets.
        let mut game = test_game();
        game.pending_deal_offers = vec![test_deal_offer(&game, None)];
        game.week = 501;
        game.check_and_generate_deal_offers(&mut StdRng::seed_from_u64(0));
        assert_eq!(
            game.pending_deal_offers.len(),
            1,
            "legacy offers must never expire"
        );

        // And the on-disk shape old builds wrote — no expires_week key at
        // all — must deserialize to exactly that.
        let mut on_disk = serde_json::to_value(test_deal_offer(&game, Some(9))).unwrap();
        on_disk.as_object_mut().unwrap().remove("expires_week");
        let loaded: PotentialDealOffer = serde_json::from_value(on_disk).unwrap();
        assert_eq!(loaded.expires_week, None);
    }

    /// Track B's contract: a run is fully determined by its seed and the
    /// player's choices. Two games on the same seed, fed the same twenty
    /// turns, must land on the same week with the same money and fame and an
    /// identical week-by-week story; a different seed must tell a different
    /// one.
    #[test]
    fn same_seed_and_same_choices_replay_the_same_career() {
        fn scripted_run(seed: u64) -> (u32, i32, u8, String) {
            let mut game = super::sim::seeded_game(seed);
            // A representative career slice: writing, gigging, idling, one
            // club-run single, and a multi-week break (so the per-week RNG
            // keying survives calendar jumps).
            let script = [
                GameAction::WriteSongs,
                GameAction::Gig(0),
                GameAction::LazeAround,
                GameAction::WriteSongs,
                GameAction::LazeAround,
                GameAction::RecordSingle { pressing: Some(1) },
                GameAction::Gig(0),
                GameAction::LazeAround,
                GameAction::Gig(0),
                GameAction::TakeBreak,
                GameAction::WriteSongs,
                GameAction::WriteSongs,
                GameAction::Gig(0),
                GameAction::LazeAround,
                GameAction::WriteSongs,
                GameAction::LazeAround,
                GameAction::Gig(0),
                GameAction::LazeAround,
                GameAction::Gig(0),
                GameAction::LazeAround,
            ];
            let mut log: Vec<String> = Vec::new();
            for action in script {
                // A rejection is part of the story too — it must replay.
                if let Err(rejection) = game.process_turn(action) {
                    log.push(format!("[rejected] {rejection}"));
                }
                log.append(&mut game.take_turn_log());
            }
            (game.week, game.player.money, game.band.fame, log.join("\n"))
        }

        let (week_a, money_a, fame_a, story_a) = scripted_run(2025);
        let (week_b, money_b, fame_b, story_b) = scripted_run(2025);
        assert_eq!(week_a, week_b, "same seed, same calendar");
        assert_eq!(money_a, money_b, "same seed, same bank balance");
        assert_eq!(fame_a, fame_b, "same seed, same fame");
        assert_eq!(story_a, story_b, "same seed, same story, line for line");

        // The script must have exercised the seeded rolls for the proof to
        // mean anything: songs written and a single actually recorded.
        assert!(
            story_a.contains("🎼 Wrote"),
            "the script should write songs:\n{story_a}"
        );
        assert!(
            story_a.contains("🎙️ Recorded"),
            "the script should record a single:\n{story_a}"
        );

        let (_, _, _, story_c) = scripted_run(2026);
        assert_ne!(
            story_a, story_c,
            "a different seed must tell a different story"
        );
    }

    #[test]
    fn saves_from_v0_4_still_load() {
        // A real save written by the v0.4.0 binary (f8e5eb9): a 13-week
        // career with one single released and songs in the drawer. It
        // predates idle_streak, genre_trend_reported, band genre, pressing
        // runs, and offer expiry — loading must fill every gap in.
        let path = concat!(env!("CARGO_MANIFEST_DIR"), "/tests/fixtures/pre-0.5.sav");
        let mut game = Game::load_game(path).expect("a v0.4.0 save must keep loading");

        // What the old binary wrote survives the trip.
        assert_eq!(game.week, 13);
        assert_eq!(game.player.money, 1029);
        assert_eq!(game.band.fame, 3);
        assert_eq!(game.band.singles_released.len(), 1);

        // Fields born in the 0.5 cycle take their documented defaults.
        assert_eq!(game.idle_streak, 0, "no idle history: decay starts fresh");
        assert_eq!(game.genre_trend_reported, 0);
        assert_eq!(
            game.band.genre,
            world::MusicGenre::Rock,
            "pre-genre bands load as Rock"
        );
        let single = &game.band.singles_released[0];
        assert_eq!(single.copies_pressed, 0, "legacy releases stay uncapped");
        assert_eq!(single.copies_sold, 0);
        assert!(game.pending_deal_offers.is_empty());

        // And the loaded game is playable, not merely parseable.
        game.process_turn(GameAction::LazeAround)
            .expect("a loaded v0.4.0 game must take a turn");
        assert_eq!(game.week, 14);
    }
}
