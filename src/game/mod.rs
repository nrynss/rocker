//! Rocker game simulation core.
//!
//! Exposes `Game` as the primary state machine, `GameAction` as the input
//! command set, and submodules for simulation subsystems (band, player, world, etc.).

mod actions;
pub mod band;
mod constants;
mod economy;
pub mod events;
mod events_apply;
#[allow(clippy::module_inception)]
pub mod game;
pub mod genre;
pub mod music;
pub mod player;
#[cfg(test)]
mod sim; // Track D balance lab: bot-driven career sims, tests only.
pub mod timeline;
mod turn;
pub mod world;

#[cfg(test)]
mod tests;

pub use constants::{BREAK_WEEKS, PRESSING_TIERS};
pub use game::{Game, GameAction, SupportTourOffer};

// Re-exports for submodules and external crates
#[allow(unused_imports)]
pub use crate::data_loader::GameDataFiles;
#[allow(unused_imports)]
pub use crate::game::music::{
    ActiveMarketingCampaign, MarketingCampaignType, Release, ReleaseType,
};
#[allow(unused_imports)]
pub use band::Band;
#[allow(unused_imports)]
pub use events::EventManager;
#[allow(unused_imports)]
pub use player::Player;
#[allow(unused_imports)]
pub use rand::rngs::StdRng;
#[allow(unused_imports)]
pub use rand::{Rng, SeedableRng};
#[allow(unused_imports)]
pub use timeline::MusicTimeline;
#[allow(unused_imports)]
pub use world::{GameWorld, PotentialDealOffer};
