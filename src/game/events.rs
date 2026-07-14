//! Random incident cadence and weighted selection.
//!
//! The incident *content* lives in `data/incidents.json` (loaded and validated
//! in `data_loader.rs`); this module only decides *when* one fires and *which*
//! one. Application is in `events_apply.rs`. All rolls draw on the action
//! stream (design §F/§G).

use rand::Rng;
use serde::{Deserialize, Serialize};

use crate::data_loader::Incident;

use super::constants::INCIDENT_WEEKLY_CHANCE_PERCENT;

#[derive(Serialize, Deserialize, Debug)]
pub struct EventManager {
    pub last_event_week: u32,
}

impl EventManager {
    pub fn new() -> Self {
        Self { last_event_week: 0 }
    }

    /// Incidents are eligible **every** week now (design §F — cadence up; was
    /// every other week).
    pub fn should_process_events(&self, current_week: u32) -> bool {
        current_week.saturating_sub(self.last_event_week) >= 1
    }

    /// Roll this week's incident gate on the action stream. Returns whether an
    /// incident should fire; the caller then picks and applies one. Marks the
    /// week so the weekly cadence holds. `last_event_week` is the only
    /// serialized state — the shape is unchanged from before the JSON move.
    pub fn try_trigger_event(&mut self, current_week: u32, rng: &mut impl Rng) -> bool {
        if !self.should_process_events(current_week) {
            return false;
        }
        if rng.gen_range(0..100u32) < INCIDENT_WEEKLY_CHANCE_PERCENT {
            self.last_event_week = current_week;
            true
        } else {
            false
        }
    }
}

/// Weighted pick among already-eligible incidents, rolled on the action stream.
/// Weights are guaranteed ≥ 1 by load-time validation, so a non-empty slice
/// always yields `Some`; `None` only when nothing is eligible.
pub fn weighted_pick<'a>(eligible: &[&'a Incident], rng: &mut impl Rng) -> Option<&'a Incident> {
    let total: u32 = eligible.iter().map(|inc| inc.weight).sum();
    if total == 0 {
        return None;
    }
    let mut roll = rng.gen_range(0..total);
    for inc in eligible {
        if roll < inc.weight {
            return Some(inc);
        }
        roll -= inc.weight;
    }
    eligible.last().copied()
}
