//! The weekly lifestyle tick: stress, happiness, creativity, and the
//! lazing-streak health wear. See `docs/DESIGN-v0.6-life-cycle.md` §A —
//! the four bars are the decided design for the v0.6 cycle.
//!
//! This is deliberately separate from `Player::weekly_health_decay`
//! (stress/20 health decay, dormant addiction terms), which stays put —
//! this tick only adds the *new* stat drift on top of it.

use super::constants::{self, *};
use super::*;

impl Game {
    /// Apply the weekly stat tick. Runs once per turn that consumes a
    /// week (see the single call-site in `turn.rs`). `action` is only
    /// consulted to track the lazing streak — health wear from turtling
    /// only kicks in after several *consecutive* lazing weeks.
    pub(super) fn update_lifestyle(&mut self, action: &GameAction) {
        if matches!(action, GameAction::LazeAround) {
            self.player.laze_streak += 1;
        } else {
            self.player.laze_streak = 0;
        }

        // Stress bleeds off on its own, worse while broke.
        self.player.stress = self.player.stress.saturating_sub(STRESS_PASSIVE_RELEASE);
        if self.player.money < 0 {
            self.player.stress =
                (self.player.stress + BROKE_STRESS_PER_WEEK).min(constants::MAX_STRESS);
        }

        // Happiness sags with however stressed the week left you.
        let happiness_drain = self.player.stress / HAPPINESS_STRESS_DIVISOR;
        self.player.happiness = self.player.happiness.saturating_sub(happiness_drain);

        // Creativity only drains once stress is genuinely high.
        if self.player.stress > CREATIVITY_STRESS_THRESHOLD {
            let creativity_drain =
                (self.player.stress - CREATIVITY_STRESS_THRESHOLD) / CREATIVITY_STRESS_DIVISOR;
            self.player.creativity = self.player.creativity.saturating_sub(creativity_drain);
        }

        // Turtling is safe for months, not years: excessive lazing wears
        // on health, much slower than anything else in the tick.
        if self.player.laze_streak > LAZE_WEAR_THRESHOLD_WEEKS {
            self.player.health = self.player.health.saturating_sub(1);
            if self.player.laze_streak == LAZE_WEAR_THRESHOLD_WEEKS + 1 {
                self.log("🛋️ Weeks on the couch are starting to catch up with you.");
            }
        }
    }
}
