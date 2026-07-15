//! Player weekly actions (split by concern). Methods remain on `Game`.

use super::super::constants::{self, *};
use super::super::player::LifestyleTier;
use super::super::*;

impl Game {
    pub(in crate::game) fn action_laze_around(&mut self) -> Result<(), String> {
        // v0.7 §B: a nicer place to crash adds to the rest-healing bonus.
        let bonus = self.player.lifestyle.rest_healing_bonus();
        self.player.stress = self
            .player
            .stress
            .saturating_sub(LAZE_STRESS_RELIEF + bonus);
        self.player.creativity =
            (self.player.creativity + LAZE_CREATIVITY_GAIN).min(constants::MAX_CREATIVITY);
        self.player.health = (self.player.health + bonus).min(constants::MAX_HEALTH);
        self.log("😴 You took it easy this week — stress down, mind wandering.");
        Ok(())
    }

    pub(in crate::game) fn action_take_break(&mut self) -> Result<(), String> {
        // v0.7 §B: the lifestyle's rest-healing bonus tops up the health gain.
        let bonus = self.player.lifestyle.rest_healing_bonus();
        self.player.stress = 0;
        self.player.happiness =
            (self.player.happiness + BREAK_HAPPINESS_GAIN).min(constants::MAX_HAPPINESS);
        self.player.creativity =
            (self.player.creativity + BREAK_CREATIVITY_GAIN).min(constants::MAX_CREATIVITY);
        self.player.health =
            (self.player.health + BREAK_HEALTH_GAIN + bonus).min(constants::MAX_HEALTH);
        self.week += BREAK_WEEKS - 1;
        self.log(format!(
            "🏖️ You disappeared for {} weeks — fully recharged and healthier for it.",
            BREAK_WEEKS
        ));
        Ok(())
    }

    pub(in crate::game) fn action_visit_doctor(&mut self) -> Result<(), String> {
        if !self.player.can_afford(constants::DOCTOR_VISIT_COST) {
            return Err(format!(
                "You need ${} to visit the doctor!",
                constants::DOCTOR_VISIT_COST
            ));
        }
        self.player.spend_money(constants::DOCTOR_VISIT_COST);
        self.player.health = (self.player.health + 20).min(constants::MAX_HEALTH);
        self.log(format!(
            "🩺 The doctor patched you up (+20 health, -${}).",
            constants::DOCTOR_VISIT_COST
        ));
        Ok(())
    }

    /// Move up, down, or get told you're already home (design §B). Always
    /// instant — no week is consumed (see the exemption in `turn.rs`).
    /// Moving is strictly the player's call; the only involuntary move is
    /// the broke eviction in `lifestyle.rs`.
    pub(in crate::game) fn action_change_lifestyle(
        &mut self,
        tier: LifestyleTier,
    ) -> Result<(), String> {
        let current = self.player.lifestyle;
        match tier.cmp(&current) {
            std::cmp::Ordering::Equal => Err(format!("You already live in a {}.", tier.label())),
            std::cmp::Ordering::Greater => {
                let cost = tier.move_up_cost() as i32;
                if !self.player.can_afford(cost) {
                    return Err(format!(
                        "Moving to a {} costs ${} up front (deposit + first week) — you can't cover it.",
                        tier.label(),
                        cost
                    ));
                }
                self.player.spend_money(cost);
                self.player.lifestyle = tier;
                self.player.happiness = (self.player.happiness
                    + constants::LIFESTYLE_MOVE_UP_HAPPINESS)
                    .min(constants::MAX_HAPPINESS);
                self.log(format!(
                    "🏡 You moved up to a {} — ${} up front, and it feels like the career's finally going somewhere.",
                    tier.label(),
                    cost
                ));
                Ok(())
            }
            std::cmp::Ordering::Less => {
                self.player.lifestyle = tier;
                self.player.happiness = self
                    .player
                    .happiness
                    .saturating_sub(constants::LIFESTYLE_MOVE_DOWN_HAPPINESS);
                self.log(format!(
                    "📦 You moved down to a {} — cheaper, but it stings giving up the old place.",
                    tier.label()
                ));
                Ok(())
            }
        }
    }
}
