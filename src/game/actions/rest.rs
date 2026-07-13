//! Player weekly actions (split by concern). Methods remain on `Game`.

use super::super::*;
impl Game {
    pub(in crate::game) fn action_laze_around(&mut self) -> Result<(), String> {
        self.player.energy = (self.player.energy + 20).min(constants::MAX_ENERGY);
        self.player.stress = self.player.stress.saturating_sub(10);
        self.log("😴 You took it easy this week — energy up, stress down.");
        Ok(())
    }

    pub(in crate::game) fn action_take_break(&mut self) -> Result<(), String> {
        self.player.energy = constants::MAX_ENERGY;
        self.player.stress = 0;
        self.player.health = (self.player.health + 30).min(constants::MAX_HEALTH);
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
}
