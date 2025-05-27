use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Player {
    pub name: String,
    pub money: i32,
    pub health: u8,            // 0-100
    pub energy: u8,            // 0-100
    pub stress: u8,            // 0-100
    pub drug_addiction: u8,    // 0-100
    pub alcohol_addiction: u8, // 0-100
}

impl Default for Player {
    fn default() -> Self {
        Self {
            name: String::new(),
            money: 0,
            health: 100,
            energy: 100,
            stress: 0,
            drug_addiction: 0,
            alcohol_addiction: 0,
        }
    }
}

impl Player {
    pub fn new(name: String) -> Self {
        Self {
            name,
            money: 500,
            health: 100,
            energy: 100,
            stress: 0,
            drug_addiction: 0,
            alcohol_addiction: 0,
        }
    }

    pub fn get_health_status(&self) -> &str {
        match self.health {
            90..=100 => "Excellent",
            70..=89 => "Good",
            50..=69 => "Fair",
            30..=49 => "Poor",
            10..=29 => "Very Poor",
            _ => "Critical",
        }
    }

    pub fn get_energy_status(&self) -> &str {
        match self.energy {
            80..=100 => "Full of energy",
            60..=79 => "Energetic",
            40..=59 => "Tired",
            20..=39 => "Exhausted",
            _ => "Dead tired",
        }
    }

    pub fn get_stress_status(&self) -> &str {
        match self.stress {
            0..=20 => "Relaxed",
            21..=40 => "Mild stress",
            41..=60 => "Stressed",
            61..=80 => "Very stressed",
            _ => "Burnout",
        }
    }

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
