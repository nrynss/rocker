//! The weekly lifestyle tick: stress, happiness, creativity, the
//! lazing-streak health wear, and — since v0.7 — the lifestyle ladder's
//! rent, stat effects, image penalty, and broke eviction. See
//! `docs/DESIGN-v0.6-life-cycle.md` §A (the four bars) and
//! `docs/DESIGN-v0.7-money-cycle.md` §B (the lifestyle ladder). The
//! module finally earns its name.
//!
//! This is deliberately separate from `Player::weekly_health_decay`
//! (stress/20 health decay, dormant addiction terms), which stays put —
//! this tick only adds the *new* stat drift on top of it.

use super::constants::{self, *};
use super::player::LifestyleTier;
use super::*;

impl Game {
    /// Apply the weekly stat tick. Runs once per turn that consumes a
    /// week (see the single call-site in `turn.rs`). `action` is only
    /// consulted to track the lazing and writing streaks — health wear from
    /// turtling and creative fatigue kick in after several *consecutive* weeks.
    pub(super) fn update_lifestyle(&mut self, action: &GameAction) {
        if matches!(action, GameAction::LazeAround) {
            self.player.laze_streak += 1;
        } else {
            self.player.laze_streak = 0;
        }

        if matches!(action, GameAction::WriteSongs) {
            // writing_streak is incremented in action_write_songs; just keep it here
        } else {
            self.writing_streak = 0;
        }

        // --- v0.7 §B: rent comes out first, then the broke clock (which
        // may force an eviction) reads the post-rent balance. ---
        self.apply_lifestyle_upkeep();
        self.check_broke_eviction();

        // Stress bleeds off on its own, worse while broke — the lifestyle's
        // stress-release bonus adds on top of the base passive release.
        let stress_release = STRESS_PASSIVE_RELEASE + self.player.lifestyle.stress_release_bonus();
        self.player.stress = self.player.stress.saturating_sub(stress_release);
        if self.player.money < 0 {
            self.player.stress =
                (self.player.stress + BROKE_STRESS_PER_WEEK).min(constants::MAX_STRESS);
        }

        // Happiness sags with however stressed the week left you — the
        // lifestyle's happiness floor stops *this specific drain* from
        // pulling it any lower. If happiness is already at or below the
        // floor (an event or incident put it there — a different code
        // path), the drain still applies normally instead of propping it
        // back up: the floor guards against this drain, it doesn't heal.
        let happiness_drain = self.player.stress / HAPPINESS_STRESS_DIVISOR;
        let floor = self.player.lifestyle.happiness_floor();
        let happiness_before = self.player.happiness;
        let drained = happiness_before.saturating_sub(happiness_drain);
        self.player.happiness = if happiness_before > floor {
            drained.max(floor)
        } else {
            drained
        };

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

        self.check_lifestyle_image();
    }

    /// Weekly rent. Always charged in full, even into the red — that's
    /// what starts the broke-eviction clock (§B).
    fn apply_lifestyle_upkeep(&mut self) {
        let upkeep = self.player.lifestyle.upkeep_per_week();
        if upkeep > 0 {
            self.player.money -= upkeep as i32;
        }
    }

    /// The only involuntary move: two consecutive weeks in the red and
    /// the landlord downgrades you a tier, no matter how you got there
    /// (§B — Broke eviction).
    fn check_broke_eviction(&mut self) {
        if self.player.money < 0 {
            self.player.weeks_broke = self.player.weeks_broke.saturating_add(1);
        } else {
            self.player.weeks_broke = 0;
            return;
        }

        if self.player.weeks_broke < LIFESTYLE_EVICTION_WEEKS {
            return;
        }
        self.player.weeks_broke = 0;

        if let Some(lower) = self.player.lifestyle.down() {
            let from = self.player.lifestyle.label();
            self.player.lifestyle = lower;
            self.player.happiness = self
                .player
                .happiness
                .saturating_sub(LIFESTYLE_EVICTION_HAPPINESS);
            self.log(format!(
                "🏚️ Broke two weeks running — the landlord's had enough. Evicted from the {} to the {}.",
                from,
                lower.label()
            ));
        }
    }

    /// Fame ≥ 60 while living at Squat or Shared flat draws the tabloids:
    /// happiness −2/week, one news line the first week of the streak.
    /// A Mansion draws no penalty at any fame — rock'n'roll excess is
    /// allowed, the rent is the penalty (§B — Image).
    fn check_lifestyle_image(&mut self) {
        let low_rent = matches!(
            self.player.lifestyle,
            LifestyleTier::Squat | LifestyleTier::SharedFlat
        );
        if low_rent && self.band.fame >= LIFESTYLE_IMAGE_FAME_THRESHOLD {
            self.player.happiness = self
                .player
                .happiness
                .saturating_sub(LIFESTYLE_IMAGE_HAPPINESS_LOSS);
            self.player.tabloid_streak = self.player.tabloid_streak.saturating_add(1);
            if self.player.tabloid_streak == 1 {
                self.log(format!(
                    "📸 The tabloids can't believe a star of your fame still lives in a {} — it's not a good look.",
                    self.player.lifestyle.label().to_lowercase()
                ));
            }
        } else {
            self.player.tabloid_streak = 0;
        }
    }
}
