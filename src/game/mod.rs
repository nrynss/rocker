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
pub mod music;
pub mod player;
mod rng;
#[cfg(test)]
mod sim; // Track D balance lab: bot-driven career sims, tests only.
pub mod timeline;
mod turn;
pub mod world;

#[cfg(test)]
mod tests;

pub use constants::{BREAK_WEEKS, PRESSING_TIERS};
pub use core::{Game, GameAction, SupportTourOffer};
