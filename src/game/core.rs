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
use crate::game::music::{DistributionChannel, MarketingCampaignType, Release};
use crate::game::player::{LifestyleTier, Player};
use crate::game::shows::{ShowReport, TourReport};
use crate::game::studios::RecordingStudio;
use crate::game::timeline::MusicTimeline;
use crate::game::world::{GameWorld, PotentialDealOffer};

#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum GameAction {
    LazeAround,
    WriteSongs,
    Practice,
    /// Song count and studio are explicit player choices, quoted before
    /// booking (design §A/§C, S-RECORD): 2-3 songs for a single, 8-14 for an
    /// album, in one of the five rooms. Fame never selects either; it only
    /// gates which studios are available.
    ///
    /// `pressing` stays for now — recording still presses and distributes, as
    /// it always has. §D (S-MASTER) is the task that splits recording from
    /// pressing and takes this field with it.
    RecordSingle {
        songs: u8,
        studio: RecordingStudio,
        pressing: Option<usize>,
    },
    RecordAlbum {
        songs: u8,
        studio: RecordingStudio,
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
    /// Re-press an already-released, sold-out (or low-stock) record — the
    /// indie half of §E-1 (M6). Instant, no week consumed (`turn.rs`);
    /// unavailable to a signed act, whose label restocks on its own
    /// (`economy::label_auto_repress`, M5).
    RePress {
        release_id: u32,
        pressing: Option<usize>,
    },
    /// Walk out of whatever the band is in the middle of (design §F —
    /// Abandoning): you keep what already resolved, you forfeit the rest of
    /// the commitment and the money still on the table. Instant — no week
    /// consumed, since the week you walk out of is yours to spend.
    AbandonActivity,
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

/// A multi-week commitment the band is in the middle of (design §F — "the
/// calendar never skips").
///
/// Before v0.9 three actions reached into `Game::week` and added weeks
/// behind `process_turn`'s back, so a four-week tour was four weeks of rent
/// and zero weeks of everything else. Now `week` moves in exactly one place
/// and a commitment is *state*: starting one charges its up-front cost and
/// resolves its first week, every turn after that resolves one more (the
/// submitted action is ignored), and the last one settles and clears.
///
/// **S-MASTER seam.** `Recording { .. }` — a booked studio project (§B/§D)
/// — is the fourth arm of this enum. Everything it needs is already here:
/// add the variant, a `resolve_*_week` arm in `actions/studio.rs`, and the
/// three `match` arms in this file's accessors; `turn.rs` needs no change.
#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum OngoingActivity {
    Tour(TourInProgress),
    SupportTour(SupportTourInProgress),
    Break(BreakInProgress),
}

/// A headline tour in progress (design §A + §F). Everything the tour's
/// economics depend on is resolved once, at booking, by `tour_pot` — the
/// cost, the pot, the per-show share, the fame and regional-fame gains —
/// and frozen here. The split spreads those numbers over the weeks; it
/// never recomputes them, which is what keeps §F economically neutral.
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct TourInProgress {
    /// Index into `get_sorted_regions()`, kept so the turn can hand the UI
    /// back the booking that started this.
    pub region_index: usize,
    pub rig: TourRig,
    pub region_name: String,
    pub country_key: String,
    pub weeks_total: u8,
    /// Weeks already resolved. `weeks_done == weeks_total` never persists:
    /// the week that reaches the total also settles and clears the tour.
    pub weeks_done: u8,
    /// The week the bus left. Keys the activity RNG stream (`rng.rs`), so
    /// where the tour's nights fall on the calendar can't change what they
    /// roll.
    pub booking_week: u32,

    // --- the resolved pot: `tour_pot`'s output, frozen at booking ---
    pub cost: i32,
    pub shows_total: u32,
    pub synth_capacity: u32,
    /// `total_potential_gross / shows_total` — the centre of every night's
    /// take. Frozen so the money a tour pays out cannot drift with the
    /// world while the tour is running.
    pub per_show_share: f32,
    pub base_fill_ratio: f32,
    pub fame_gain: u8,
    pub regional_fame_key: String,
    pub regional_fame_current: u8,
    pub regional_fame_gain_base: u16,

    // --- the road: what the tour carries from night to night ---
    pub era_genre_modifier: f32,
    /// The band's condition when the bus left. A tour's receptions are
    /// judged on the shape the band was in at the gate: the rent, a label
    /// memo and a chart slide back home don't change how Tuesday's set went
    /// down. Freezing it is also what makes the split *provably* neutral —
    /// these are the exact numbers the pre-split whole-tour loop read, which
    /// it could take for granted because no week ever passed underneath it.
    pub condition_stress: u8,
    pub condition_health: u8,
    /// Creativity as the road sees it: seeded from the player at booking and
    /// moved only by what happens on stage (§B — great and transcendent
    /// nights), exactly as the pre-split loop's live read did.
    pub road_creativity: u8,
    /// Word of mouth, carried across the week boundary (§B — Momentum).
    /// This is the reason the tour's state has to live here at all.
    pub momentum: f32,
    /// Nights resolved so far, in order — the tour report under construction.
    pub rows: Vec<ShowReport>,
}

/// A support slot in progress (design §F). The guarantee is banked at
/// acceptance, as it always was; the wear and the exposure spread over the
/// weeks actually spent on someone else's stage.
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct SupportTourInProgress {
    pub host_band: String,
    pub weeks_total: u8,
    pub weeks_done: u8,
    /// Keys the activity RNG stream for the settlement roll (`rng.rs`).
    pub start_week: u32,
    pub pay: u32,
    pub fame_gain: u8,
}

/// Time off (design §F). The recovery lands when the band comes back, not
/// when it leaves — otherwise a break could be started, banked, and walked
/// out of the following week for a full recharge in one.
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct BreakInProgress {
    pub weeks_total: u8,
    pub weeks_done: u8,
}

/// The activity screen's read-only surface (§H). `allow(dead_code)`
/// because S-UI wires it up in wave 3 and this task must not touch the UI
/// files: every method here exists to answer one question the screen asks.
#[allow(dead_code)]
impl OngoingActivity {
    /// Total weeks the commitment runs for.
    pub fn weeks_total(&self) -> u8 {
        match self {
            OngoingActivity::Tour(tour) => tour.weeks_total,
            OngoingActivity::SupportTour(support) => support.weeks_total,
            OngoingActivity::Break(rest) => rest.weeks_total,
        }
    }

    /// Weeks already resolved (0 before the first one settles).
    pub fn weeks_done(&self) -> u8 {
        match self {
            OngoingActivity::Tour(tour) => tour.weeks_done,
            OngoingActivity::SupportTour(support) => support.weeks_done,
            OngoingActivity::Break(rest) => rest.weeks_done,
        }
    }

    /// The week currently being played, 1-based — the *n* of "week n of m".
    pub fn week_number(&self) -> u8 {
        self.weeks_done().saturating_add(1).min(self.weeks_total())
    }

    /// Whether the week about to resolve is the one that settles it.
    pub fn is_final_week(&self) -> bool {
        self.weeks_done().saturating_add(1) >= self.weeks_total()
    }

    /// Short kind name for a header ("Tour", "Break").
    pub fn label(&self) -> &'static str {
        match self {
            OngoingActivity::Tour(_) => "Tour",
            OngoingActivity::SupportTour(_) => "Support tour",
            OngoingActivity::Break(_) => "Break",
        }
    }

    /// One line naming the specific commitment, for the activity screen
    /// (§H): "Van tour of California", "Opening for The Bright Lights".
    pub fn headline(&self) -> String {
        match self {
            OngoingActivity::Tour(tour) => {
                format!("{} of {}", tour.rig.label(), tour.region_name)
            }
            OngoingActivity::SupportTour(support) => {
                format!("Opening for {}", support.host_band)
            }
            OngoingActivity::Break(_) => "Off the grid".to_string(),
        }
    }

    /// "week 2 of 4" — the progress line the activity screen needs (§H).
    pub fn progress(&self) -> String {
        format!("week {} of {}", self.week_number(), self.weeks_total())
    }

    /// The nights resolved so far, for a tour in progress; empty otherwise.
    /// Lets the activity screen show the report as it is being written.
    pub fn rows_so_far(&self) -> &[ShowReport] {
        match self {
            OngoingActivity::Tour(tour) => &tour.rows,
            OngoingActivity::SupportTour(_) | OngoingActivity::Break(_) => &[],
        }
    }

    /// The action that started this, replayed for the turn machinery: fame
    /// visibility (§C) and the lifestyle streaks read the *action*, and a
    /// week spent on the road is a week on the road whether the player
    /// pressed the key that week or four weeks ago.
    pub(super) fn as_action(&self) -> GameAction {
        match self {
            OngoingActivity::Tour(tour) => {
                GameAction::GoOnTour(tour.region_index, tour.rig, tour.weeks_total)
            }
            OngoingActivity::SupportTour(_) => GameAction::AcceptSupportTour,
            OngoingActivity::Break(_) => GameAction::TakeBreak,
        }
    }
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
    /// The player's currently chosen indie distribution channel (design
    /// §E-3, M6). Not itself sales-authoritative — it's read at the moment
    /// an unsigned release goes out, to charge that release's fee and stamp
    /// its own frozen `Release::distribution_channel`, which is what sales
    /// math actually reads. Exists so the picker can remember a choice
    /// across sessions without adding a field to `GameAction::RecordSingle`/
    /// `RecordAlbum` (both exercised verbatim by the determinism tests).
    /// `#[serde(default)]` so pre-M6 saves start at Mail order & gigs —
    /// the exact reach the old, single indie formula already gave.
    #[serde(default)]
    pub current_distribution_channel: DistributionChannel,
    /// The last week the weekly sales pass
    /// (`process_music_releases_and_marketing`) resolved. The pass fires
    /// after EVERY action, but instant actions (marketing, lifestyle,
    /// re-press, deal responses) don't advance the calendar — without this
    /// guard each one re-sold a full catalog-tail week inside the same game
    /// week: bonus copies, income, recoupment paydown, and certification
    /// progress per instant action. `#[serde(default)]` (`None`) so an old
    /// save's first pass after load runs normally.
    #[serde(default)]
    pub(super) last_sales_pass_week: Option<u32>,
    /// The multi-week commitment the band is in the middle of, if any
    /// (design §F). While this is `Some`, `process_turn` ignores the
    /// submitted action and resolves one more week of it.
    /// `#[serde(default)]` so every pre-0.9 save loads with an empty
    /// calendar — and a save taken mid-tour resumes into the same week it
    /// was taken in, since the activity carries its own progress and keys
    /// its own RNG.
    #[serde(default)]
    pub ongoing_activity: Option<OngoingActivity>,
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
            current_distribution_channel: DistributionChannel::default(),
            last_sales_pass_week: None,
            ongoing_activity: None,
        })
    }

    /// The commitment in progress, for the activity screen (§H). `None`
    /// means the band is free and the ordinary action menu applies.
    /// `allow(dead_code)`: S-UI is the caller, and wires up in wave 3.
    #[allow(dead_code)]
    pub fn ongoing_activity(&self) -> Option<&OngoingActivity> {
        self.ongoing_activity.as_ref()
    }

    /// Whether the band is committed this week — the one question the UI has
    /// to ask before drawing a menu (§F/§H). A busy band gets "continue" and
    /// "abandon", not a list of choices.
    #[allow(dead_code)]
    pub fn is_busy(&self) -> bool {
        self.ongoing_activity.is_some()
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
        self.player.money = 1500; // Starting cash in 1970 (L-SCALE §B: ×3 with the rest of the economy)

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
        // M3 note: one-time migration for saves from before regional
        // charts — the old flat top-10 seeds the Local board, then stays
        // empty (design §C, `GameWorld::migrate_legacy_charts`).
        loaded_game.world.migrate_legacy_charts();

        Ok(loaded_game)
    }
}
