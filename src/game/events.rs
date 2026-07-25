//! Random incident cadence and weighted selection.
//!
//! The incident *content* lives in `data/incidents.json` (loaded and validated
//! in `data_loader.rs`); this module only decides *when* one fires and *which*
//! one. Application is in `events_apply.rs`. All rolls draw on the action
//! stream (design §F/§G).
//!
//! Since v0.9 §G there are **two** taps, in this order every week:
//!
//! 1. the **activity tap** — a modest per-week chance that whatever the band
//!    is in the middle of goes wrong (or right) in a way only that activity
//!    can: a van breaks down on tour, a take lands in the studio;
//! 2. the **general tap** — the ordinary weekly incident, unchanged.
//!
//! They share `last_event_week`, so a week fires at most one incident either
//! way: a four-week tour is a place things can happen, not a slot machine.
//! And the general tap's pool is the pre-§G pool exactly (activity-scoped
//! entries are filtered out of it in `eligible_incidents`), so a week the
//! band spends at home rolls precisely what it always rolled.

use rand::Rng;
use serde::{Deserialize, Serialize};

use crate::data_loader::{ActivityScope, Incident};

use super::constants::INCIDENT_WEEKLY_CHANCE_PERCENT;

/// Per-week chance that the activity tap fires (design §G — "[tune: ~12%/week]").
///
/// Lives here rather than in `constants.rs` for one reason: §G's tap is
/// entirely a property of this module — it is rolled here, gated here, and
/// shares `last_event_week` with the general cadence three lines up — and
/// keeping it beside `INCIDENT_WEEKLY_CHANCE_PERCENT`'s only consumer is
/// worth more than the symmetry of the constants file.
pub(super) const ACTIVITY_INCIDENT_WEEKLY_CHANCE_PERCENT: u32 = 12;

#[derive(Serialize, Deserialize, Debug)]
pub struct EventManager {
    pub last_event_week: u32,
    /// What the band spent *this* week doing, latched by the turn before the
    /// commitment can settle out from under the event tap (design §G).
    ///
    /// The taps run in `advance_week_events`, which is after a tour's final
    /// week has already cleared `ongoing_activity` — and a one-off gig is
    /// never an `OngoingActivity` at all. So the two places that know what
    /// the week was spent on say so here (`Game::note_activity_week`,
    /// `Game::note_gig_week`) and the tap takes it. Absent a latch the tap
    /// falls back to reading `ongoing_activity` directly, which covers every
    /// week of a commitment except the one that settles it.
    ///
    /// `#[serde(default)]` and always `None` between turns — it is consumed
    /// unconditionally by the tap, so it can never leak into the next week.
    #[serde(default)]
    pub pending_activity: Option<ActivityScope>,
}

impl EventManager {
    pub fn new() -> Self {
        Self {
            last_event_week: 0,
            pending_activity: None,
        }
    }

    /// Incidents are eligible **every** week now (design §F — cadence up; was
    /// every other week). Both taps ask this, which is what keeps them to one
    /// incident a week between them (§G).
    pub fn should_process_events(&self, current_week: u32) -> bool {
        current_week.saturating_sub(self.last_event_week) >= 1
    }

    /// Roll this week's incident gate on the action stream. Returns whether an
    /// incident should fire; the caller then picks and applies one. Marks the
    /// week so the weekly cadence holds. `last_event_week` is the only
    /// serialized state — the shape is unchanged from before the JSON move.
    pub fn try_trigger_event(&mut self, current_week: u32, rng: &mut impl Rng) -> bool {
        self.try_trigger_at(current_week, INCIDENT_WEEKLY_CHANCE_PERCENT, rng)
    }

    /// Roll the §G activity gate. Same shape and the same shared cooldown as
    /// the general gate, at its own modest chance. The caller has already
    /// checked that the activity's pool is non-empty, so an activity nothing
    /// can happen during costs no roll and — crucially — cannot burn the
    /// week's one incident on nothing.
    pub fn try_trigger_activity_event(&mut self, current_week: u32, rng: &mut impl Rng) -> bool {
        self.try_trigger_at(current_week, ACTIVITY_INCIDENT_WEEKLY_CHANCE_PERCENT, rng)
    }

    /// Shared gate: cooldown, one `0..100` roll, stamp the week on a hit.
    /// One draw per tap that gets as far as rolling — the general tap's draw
    /// order on a free week is therefore exactly what it was before §G.
    fn try_trigger_at(
        &mut self,
        current_week: u32,
        chance_percent: u32,
        rng: &mut impl Rng,
    ) -> bool {
        if !self.should_process_events(current_week) {
            return false;
        }
        if rng.gen_range(0..100u32) < chance_percent {
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
