//! Tuning knobs and balance constants for the game simulation.
//!
//! Gameplay dials and RNG stream salts live here (formerly the top of
//! `game/mod.rs`). Data-level caps and costs (`MAX_HEALTH`, `WEEKS_PER_YEAR`,
//! recording prices, win thresholds, …) are re-exported from
//! [`crate::data::constants`] so callers can use one module:
//!
//! ```ignore
//! use crate::game::constants::{self, *};
//! // bare: LIVE_FAME_BASE_CAP, PRESSING_TIERS
//! // path: constants::MAX_FAME, constants::ACTION_STREAM_SALT
//! ```

pub use crate::data::constants::*;

// Quality calculation constants
pub(super) const QUALITY_BASE_SONGWRITING: u8 = 30;
pub(super) const QUALITY_SONGWRITING_RANDOM_VARIATION: u8 = 10;
pub(super) const QUALITY_BASE_RECORDING: u8 = 30;
pub(super) const QUALITY_RECORDING_RANDOM_VARIATION: u8 = 10;

// Sales model constants
pub(super) const INITIAL_SALES_WINDOW_WEEKS: u32 = 4;
pub(super) const SALES_QUALITY_WEIGHT: f32 = 2.5;
pub(super) const SALES_MARKETING_WEIGHT: f32 = 1.8;
pub(super) const SALES_FAME_WEIGHT: f32 = 1.2;

// Unit economics: a sales score converts into copies people want to buy,
// bounded by how many copies actually exist.
pub(super) const UNITS_PER_SCORE_POINT: f32 = 10.0;
pub(super) const INDIE_INCOME_PER_COPY: u32 = 2;
pub(super) const LABEL_INCOME_PER_COPY: u32 = 3;

// Living sales tail: post-launch decay and influence [tune].
pub(super) const TAIL_DIVISOR_WEEKS_PER_STEP: u32 = 3;
pub(super) const TAIL_MARKETING_WEIGHT: f32 = 1.8;
pub(super) const TAIL_FAME_WEIGHT: f32 = 0.3;

// Pressing runs. Independents choose a run and pay setup plus per-copy
// costs; a label presses to the size of its network and your name.
pub const PRESSING_TIERS: [(&str, u32); 4] = [
    ("Garage run", 500),
    ("Club run", 2_000),
    ("Regional run", 10_000),
    ("National run", 50_000),
];
pub(super) const PRESSING_SETUP_SINGLE: f32 = 25.0;
pub(super) const PRESSING_SETUP_ALBUM: f32 = 100.0;
pub(super) const PRESSING_PER_COPY_SINGLE: f32 = 0.10;
pub(super) const PRESSING_PER_COPY_ALBUM: f32 = 0.50;
pub(super) const LABEL_PRESSING_PER_REACH: u32 = 100;
pub(super) const LABEL_PRESSING_PER_FAME: u32 = 50;

// Distribution model: how much of a release's potential audience you can
// actually reach. Labels bring their market_reach; independents are capped
// by their own fame.
pub(super) const INDIE_REACH_FLOOR: f32 = 0.15;

// Support tours: bigger acts occasionally want you as their opener.
pub(super) const SUPPORT_OFFER_MIN_FAME: u8 = 5;
pub(super) const SUPPORT_OFFER_FAME_GAP: u8 = 10;
pub(super) const SUPPORT_OFFER_CHANCE: f64 = 0.06;
pub(super) const SUPPORT_OFFER_LIFETIME_WEEKS: u32 = 3;

// Record deals stay on the table about a month — one scouting cycle — so
// a slate the player sits on clears just as labels next come looking, and
// ignoring an offer can never silence the deal stream for good.
pub(super) const DEAL_OFFER_LIFETIME_WEEKS: u32 = 4;

pub(super) const PLAYER_MARKET_IMPACT_THRESHOLD_SALES_SCORE: u32 = 600;
pub(super) const PLAYER_MARKET_IMPACT_GENRE_MOD_BONUS: f32 = 0.05;
pub(super) const PLAYER_MARKET_IMPACT_DEMAND_BONUS: u8 = 1;

// Live fame ceilings: a gig only reaches the crowd in the room, and without
// records word of mouth stalls. Gigs and tours raise fame no further than
// the smaller of the venue's ceiling and the catalog cap.
pub(super) const VENUE_FAME_HEADROOM: u8 = 15;
pub(super) const LIVE_FAME_BASE_CAP: u8 = 35;
pub(super) const LIVE_FAME_PER_SINGLE: u8 = 6;
pub(super) const LIVE_FAME_PER_ALBUM: u8 = 12;

// A break clears the calendar for this many weeks (a full reset action).
pub const BREAK_WEEKS: u32 = 4;

// ============================================================================
// Fame gravity (v0.6 design §C — fully decided; only the comeback multiplier
// is [tune]). Fame is earned slowly and defended by staying in the picture;
// left alone it drifts back down toward a floor the band earned at its peak.
// ============================================================================

/// Comeback rule: while current fame is below the band's peak, every fame
/// *gain* is multiplied by this. Reclaiming ground is easier than the first
/// climb. Only this value is tune-able. (§C — Comeback)
pub(super) const FAME_COMEBACK_MULTIPLIER: u8 = 2;

/// The idle-decay ramp: −1 the first decay week, −2, −3, −4, then −5 every
/// week after — this is where it flattens out. (§C — The ramp)
pub(super) const FAME_RAMP_MAX_DECAY: u8 = 5;

/// Establishment rule: at or above this fame, an album/single released in the
/// recent window counts as staying in the picture. Below it, small acts must
/// keep physically showing up. (§C — Activity, rule 3)
pub(super) const ESTABLISHMENT_MIN_FAME: u8 = 60;
/// How recent a release must be to satisfy the establishment rule. (§C)
pub(super) const ESTABLISHMENT_RELEASE_WINDOW_WEEKS: u32 = 52;

/// The top floor (75) also requires this many *hits* — albums/singles that
/// charted at all (`peak_chart_position.is_some()`). (§C — Floors)
pub(super) const FAME_FLOOR_HITS_THRESHOLD: usize = 10;

/// Grace: consecutive quiet weeks before *any* decay begins, keyed by the
/// band's *current* fame. Each row is `(inclusive-upper-fame, grace weeks)`;
/// the first row whose bound the current fame falls under wins. (§C — Grace)
pub(super) const FAME_GRACE_TIERS: [(u8, u32); 7] = [
    (15, 2),       // 0–15   → 2 weeks
    (29, 4),       // 16–29  → 4 weeks
    (49, 8),       // 30–49  → 8 weeks
    (74, 13),      // 50–74  → 13 weeks (3 months)
    (89, 26),      // 75–89  → 26 weeks (6 months)
    (94, 39),      // 90–94  → 39 weeks (9 months)
    (u8::MAX, 52), // 95+    → 52 weeks (1 year)
];

/// Floors: fame never *decays* below these, keyed by the highest fame the band
/// ever reached (its peak). Each row is `(minimum-peak, floor)`, checked from
/// the top down so the highest matching peak wins. The 95+ row's 75 floor is
/// gated on `FAME_FLOOR_HITS_THRESHOLD` hits and applied in code. (§C — Floors)
pub(super) const FAME_FLOOR_TIERS: [(u8, u8); 7] = [
    (95, 70), // 95+ (→ 75 with ≥ 10 hits)
    (90, 60),
    (75, 45),
    (60, 30),
    (45, 15),
    (30, 10),
    (0, 0), // under 30
];
/// The elevated top floor once the 95+ peak is paired with enough hits. (§C)
pub(super) const FAME_FLOOR_LEGEND: u8 = 75;

// Era-genre fit: past these bounds the era clearly loves or has abandoned
// the band's sound, and the press says so — once per swing, not every week.
// (A genre missing from an era's table reads as out of fashion at 0.85.)
pub(super) const GENRE_TREND_HOT: f32 = 1.15;
pub(super) const GENRE_TREND_COLD: f32 = 0.85;

// ============================================================================
// Label single-cuts (v0.6 design §C — label releases a single on its own
// volition when signed, the band is quiet, and they have un-singled tracks).
// ============================================================================

/// Probability per week (0.0–1.0) that a label releases a single from an
/// eligible album. (§C — Label single-cuts) [tune]
pub(super) const LABEL_CUT_CHANCE: f64 = 0.10;

/// Weeks the band must be quiet (idle_streak) before the label cuts a single.
/// (§C — Label single-cuts) [tune]
pub(super) const LABEL_CUT_IDLE_WEEKS: u32 = 3;

/// Weeks since any release before the label is allowed to cut a single.
/// (§C — Label single-cuts) [tune]
pub(super) const LABEL_CUT_RELEASE_COOLDOWN_WEEKS: u32 = 6;

/// Maximum singles cut from any one album. (§C — Label single-cuts) [tune]
pub(super) const LABEL_CUT_MAX_PER_ALBUM: u32 = 2;

// --- L1: stat engine — the four bars (docs/DESIGN-v0.6-life-cycle.md §A) ---
// Weekly lifestyle tick (`lifestyle.rs`): stress bleeds off on its own,
// worse while broke; happiness and creativity sag once stress runs high;
// excessive lazing wears on health, but far slower than anything else.
pub(super) const STRESS_PASSIVE_RELEASE: u8 = 3;
pub(super) const BROKE_STRESS_PER_WEEK: u8 = 5;
pub(super) const HAPPINESS_STRESS_DIVISOR: u8 = 25;
pub(super) const CREATIVITY_STRESS_THRESHOLD: u8 = 40;
pub(super) const CREATIVITY_STRESS_DIVISOR: u8 = 20;
pub(super) const LAZE_WEAR_THRESHOLD_WEEKS: u32 = 4;

// Ceilings for the two new bars (health/stress already have MAX_HEALTH /
// MAX_STRESS from `data::constants`).
pub(super) const MAX_HAPPINESS: u8 = 100;
pub(super) const MAX_CREATIVITY: u8 = 100;

// Rest actions (`actions/rest.rs`): laze trades stress for a little
// creativity; a real break resets stress and gives happiness/creativity/
// health a proper boost.
pub(super) const LAZE_STRESS_RELIEF: u8 = 15;
pub(super) const LAZE_CREATIVITY_GAIN: u8 = 3;
pub(super) const BREAK_HAPPINESS_GAIN: u8 = 10;
pub(super) const BREAK_CREATIVITY_GAIN: u8 = 10;
pub(super) const BREAK_HEALTH_GAIN: u8 = 30;

// New-player defaults (`player.rs`) — the original 1989 game's starting
// mood/imagination, not maxed like health/energy.
pub(super) const DEFAULT_HAPPINESS: u8 = 60;
pub(super) const DEFAULT_CREATIVITY: u8 = 50;

// --- L2: writing & quality (docs/DESIGN-v0.6-life-cycle.md §A, Quality formulas) ---
// Writing and recording stress costs [tune], and the guard threshold.
pub(super) const WRITE_STRESS_COST: u8 = 5;
pub(super) const RECORD_STRESS_COST: u8 = 8;
pub(super) const STUDIO_STRESS_BLOCK: u8 = 90;
// Recording quality penalty threshold [tune].
pub(super) const RECORDING_STRESS_PENALTY_THRESHOLD: u8 = 70;
// Writing streak: fatigue kicks in on the 3rd+ consecutive week [tune].
pub(super) const WRITING_STREAK_FATIGUE: u32 = 3;
pub(super) const WRITING_FATIGUE_CREATIVITY_COST: u8 = 5;
// Writing under stress drains creativity: threshold and divisor [tune].
pub(super) const WRITING_STRESS_CREATIVITY_THRESHOLD: u8 = 50;
pub(super) const WRITING_STRESS_CREATIVITY_DIVISOR: u8 = 5;
// Happiness multiplier for quality: 0.8 + happiness / 500.0 (clamped 0.8–1.0).
// Constants for the formula: min 0.8, divisor 500.
pub(super) const HAPPINESS_QUALITY_MULTIPLIER_MIN: f32 = 0.8;
pub(super) const HAPPINESS_QUALITY_MULTIPLIER_SCALE: f32 = 500.0;
// Creativity divisor for songwriting bonus: creativity / 4 yields 0–25 bonus at creativity 0–100.
pub(super) const SONGWRITING_CREATIVITY_DIVISOR: u8 = 4;
// Recording quality penalty when stress > RECORDING_STRESS_PENALTY_THRESHOLD [tune].
pub(super) const RECORDING_STRESS_PENALTY: i8 = 10;

// Determinism salts — stream construction lives in `rng.rs`.
// ACTION_STREAM_SALT keeps the action stream uncorrelated with the world
// stream (π's fractional bits: arbitrary, fixed forever).
pub(super) const ACTION_STREAM_SALT: u64 = 0x243F_6A88_85A3_08D3;
// Setup rolls (bandmate names) use this reserved pre-game week so they
// never replay week 1's action stream.
pub(super) const SETUP_STREAM_WEEK: u64 = 0;
