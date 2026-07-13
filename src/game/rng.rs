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
//!
//! Salts and the reserved setup week live in [`crate::game::constants`] next
//! to the other tuning knobs (`ACTION_STREAM_SALT`, `SETUP_STREAM_WEEK`).
//! Streams are derived on demand and never stored — saves stay compatible
//! and a loaded game rolls exactly what the unsaved one would have.
//!
//! Draw order on each stream is sacred. Do not reorder `gen_*` calls; the
//! determinism tests are the contract.

use rand::SeedableRng;
use rand::rngs::StdRng;

use crate::game::constants;
use crate::game::core::Game;

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

/// Action-stream RNG for a calendar week: every player-facing roll.
/// Salted so tour luck never mirrors the same week's scene news.
pub(super) fn action_rng_for_week(world_seed: u64, week: u64) -> StdRng {
    mix_to_rng((world_seed ^ constants::ACTION_STREAM_SALT).wrapping_add(week))
}

impl Game {
    /// Action-stream RNG for an arbitrary week (setup uses
    /// [`constants::SETUP_STREAM_WEEK`]).
    pub(super) fn action_rng_for_week(&self, week: u64) -> StdRng {
        action_rng_for_week(self.world_seed, week)
    }

    /// Action-stream RNG for the current week. Turn-consuming actions advance
    /// the calendar so consecutive turns get fresh streams; same-week
    /// paperwork (e.g. rejecting two deals) rereads this stream, which is
    /// deterministic and harmless.
    pub(super) fn action_rng(&self) -> StdRng {
        action_rng_for_week(self.world_seed, self.week as u64)
    }
}
