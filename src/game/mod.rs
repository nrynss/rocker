//! Rocker game simulation core.
//!
//! Exposes `Game` as the primary state machine, `GameAction` as the input
//! command set, and submodules for simulation subsystems (band, player, world, etc.).

mod actions;
pub mod band;
mod constants;
pub mod core;
mod economy;
pub mod events;
mod events_apply;
pub mod genre;
mod label_moves;
mod lifestyle;
pub mod music;
pub mod player;
mod rng;
mod shows;
#[cfg(test)]
mod sim; // Track D balance lab: bot-driven career sims, tests only.
pub mod timeline;
mod turn;
pub mod world;

#[cfg(test)]
mod tests;

pub use constants::{
    BREAK_WEEKS, GIG_HEALTH_GUARD, GIG_STRESS_GUARD, PRESSING_TIERS, STUDIO_STRESS_BLOCK,
    TOUR_HEALTH_GUARD, TOUR_STRESS_GUARD,
};
pub use core::{Game, GameAction, SupportTourOffer};
pub use shows::{ShowReport, TourReport};
