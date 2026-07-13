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
pub(super) const QUALITY_SONGWRITING_MAX_BONUS_PLAYER_STATS: u8 = 25;
pub(super) const QUALITY_SONGWRITING_RANDOM_VARIATION: u8 = 10;
pub(super) const QUALITY_BASE_RECORDING: u8 = 30;
pub(super) const QUALITY_RECORDING_MAX_BONUS_PLAYER_STATS: u8 = 20;
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

// Fame fades when the band disappears from view: no shows, no tour, and
// nothing new on the shelves. One quiet week is forgiven.
pub(super) const IDLE_GRACE_WEEKS: u32 = 1;
pub(super) const IDLE_FAME_DECAY_PER_WEEK: u8 = 1;
pub const BREAK_WEEKS: u32 = 4;

// Era-genre fit: past these bounds the era clearly loves or has abandoned
// the band's sound, and the press says so — once per swing, not every week.
// (A genre missing from an era's table reads as out of fashion at 0.85.)
pub(super) const GENRE_TREND_HOT: f32 = 1.15;
pub(super) const GENRE_TREND_COLD: f32 = 0.85;

// Determinism salts — stream construction lives in `rng.rs`.
// ACTION_STREAM_SALT keeps the action stream uncorrelated with the world
// stream (π's fractional bits: arbitrary, fixed forever).
pub(super) const ACTION_STREAM_SALT: u64 = 0x243F_6A88_85A3_08D3;
// Setup rolls (bandmate names) use this reserved pre-game week so they
// never replay week 1's action stream.
pub(super) const SETUP_STREAM_WEEK: u64 = 0;
