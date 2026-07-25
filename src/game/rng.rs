//! Deterministic RNG streams derived from `world_seed`.
//!
//! Everything after seed selection is reproducible: the same seed and the
//! same player choices replay the same career. Two **independent** streams
//! share one splitmix64 mixer and differ only in how the week key is formed:
//!
//! | Stream | Builder | Feeds |
//! |--------|---------|--------|
//! | **World** | `world_seed + week` | historical event *selection*, scene/`update_week` |
//! | **Action** | `(world_seed ^ ACTION_STREAM_SALT) + week` | player actions, random-event outcomes, deal rolls |
//! | **Activity** | `(world_seed ^ ACTIVITY_STREAM_SALT) + booking_week × stride + step` | one step of a multi-week commitment (v0.9 §F) |
//!
//! The activity stream is the odd one out: it is keyed by *step index*, not
//! by position in a running stream. See [`activity_step_rng`].
//!
//! Salts and the reserved setup week live in [`super::constants`] next to
//! the other tuning knobs (`ACTION_STREAM_SALT`, `SETUP_STREAM_WEEK`).
//! Streams are derived on demand and never stored — saves stay compatible
//! and a loaded game rolls exactly what the unsaved one would have.
//!
//! Draw order on each stream is sacred. Do not reorder `gen_*` calls; the
//! determinism tests are the contract.

use rand::SeedableRng;
use rand::rngs::StdRng;

use super::constants;
use super::core::Game;

/// Splitmix64 finalizer shared by both streams. `key` is the pre-mix material
/// (seed±week, already salted for the action stream).
fn mix_to_rng(mut key: u64) -> StdRng {
    key = key.wrapping_mul(0x9E3779B97F4A7C15);
    key = (key ^ (key >> 30)).wrapping_mul(0xBF58476D1CE4E5B8);
    key = (key ^ (key >> 27)).wrapping_mul(0x94D049BB133111EB);
    key ^= key >> 31;
    StdRng::seed_from_u64(key)
}

/// World-stream RNG for a calendar week: scene evolution and historical
/// event selection. Uncorrelated with the action stream (no salt).
pub(super) fn world_rng_for_week(world_seed: u64, week: u64) -> StdRng {
    mix_to_rng(world_seed.wrapping_add(week))
}

/// Action-stream seed material for a week (seed ⊕ salt, then + week).
fn action_stream_key(world_seed: u64, week: u64) -> u64 {
    (world_seed ^ constants::ACTION_STREAM_SALT).wrapping_add(week)
}

/// Action-stream RNG for a calendar week: every player-facing roll.
/// Salted so tour luck never mirrors the same week's scene news.
pub(super) fn action_stream_rng(world_seed: u64, week: u64) -> StdRng {
    mix_to_rng(action_stream_key(world_seed, week))
}

/// Activity-stream RNG for one *step* of a multi-week commitment — a single
/// tour night, the regional-fame settlement at the end of one (v0.9 §F).
///
/// Keyed by the week the commitment was booked plus the step's index within
/// it, deliberately **not** by position in a running stream. §F resolves a
/// tour five nights a week instead of twenty in one sitting, with the weekly
/// world tick running in between; a position-based stream would either have
/// to survive a save/load mid-tour or shift under whatever the world drew
/// that week. Keying by step means night 17 rolls the same numbers however
/// the calendar chopped the tour up — which is what makes the split
/// economically neutral, and what a `#[serde(default)]` activity can resume
/// into from `weeks_done` alone.
pub(super) fn activity_step_rng(world_seed: u64, booking_week: u64, step: u64) -> StdRng {
    mix_to_rng(
        (world_seed ^ constants::ACTIVITY_STREAM_SALT)
            .wrapping_add(booking_week.wrapping_mul(constants::ACTIVITY_WEEK_STRIDE))
            .wrapping_add(step),
    )
}

impl Game {
    /// Action-stream RNG for an arbitrary week (setup uses
    /// [`constants::SETUP_STREAM_WEEK`]).
    pub(super) fn action_rng_for_week(&self, week: u64) -> StdRng {
        action_stream_rng(self.world_seed, week)
    }

    /// Action-stream RNG for the current week. Turn-consuming actions advance
    /// the calendar so consecutive turns get fresh streams; same-week
    /// paperwork (e.g. rejecting two deals) rereads this stream, which is
    /// deterministic and harmless.
    pub(super) fn action_rng(&self) -> StdRng {
        action_stream_rng(self.world_seed, self.week as u64)
    }

    /// Activity-stream RNG for one step of the commitment booked in
    /// `booking_week` (see [`activity_step_rng`]).
    pub(super) fn activity_rng(&self, booking_week: u32, step: u32) -> StdRng {
        activity_step_rng(self.world_seed, booking_week as u64, step as u64)
    }
}
