use serde::{Deserialize, Serialize};

use super::constants;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Player {
    pub name: String,
    pub money: i32,
    pub health: u8, // 0-100
    pub energy: u8, // 0-100 (v0.6: dormant — mechanic removed, field kept for save compat)
    pub stress: u8, // 0-100
    #[serde(default = "default_happiness")]
    pub happiness: u8, // 0-100 (v0.6 §A)
    #[serde(default = "default_creativity")]
    pub creativity: u8, // 0-100 (v0.6 §A)
    #[serde(default)]
    pub laze_streak: u32, // consecutive weeks spent lazing (v0.6 §A)
    pub drug_addiction: u8, // 0-100 (dormant — deferred to a later cycle)
    pub alcohol_addiction: u8, // 0-100 (dormant — deferred to a later cycle)
}

fn default_happiness() -> u8 {
    constants::DEFAULT_HAPPINESS
}

fn default_creativity() -> u8 {
    constants::DEFAULT_CREATIVITY
}

impl Default for Player {
    fn default() -> Self {
        Self {
            name: String::new(),
            money: 0,
            health: 100,
            energy: 100,
            stress: 0,
            happiness: constants::DEFAULT_HAPPINESS,
            creativity: constants::DEFAULT_CREATIVITY,
            laze_streak: 0,
            drug_addiction: 0,
            alcohol_addiction: 0,
        }
    }
}

impl Player {
    pub fn can_afford(&self, cost: i32) -> bool {
        self.money >= cost
    }

    pub fn spend_money(&mut self, amount: i32) -> bool {
        if self.can_afford(amount) {
            self.money -= amount;
            true
        } else {
            false
        }
    }

    pub fn earn_money(&mut self, amount: u32) {
        self.money += amount as i32;
    }

    pub fn is_addicted(&self) -> bool {
        self.drug_addiction > 50 || self.alcohol_addiction > 50
    }

    pub fn weekly_health_decay(&mut self) {
        // Health naturally decays based on stress and addictions
        let decay = (self.stress / 20) + (self.drug_addiction / 10) + (self.alcohol_addiction / 15);
        self.health = self.health.saturating_sub(decay);

        // Energy regenerates slightly each week
        self.energy = (self.energy + 10).min(100);
    }
}
