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
    /// Where the player lives (v0.7 §B — the lifestyle ladder). Old saves
    /// with no field default to `Squat`.
    #[serde(default)]
    pub lifestyle: LifestyleTier,
    /// Consecutive weeks money has been negative. Two in a row triggers a
    /// broke eviction (down one tier); resets the moment money is
    /// non-negative again (v0.7 §B).
    #[serde(default)]
    pub weeks_broke: u32,
    /// Consecutive weeks the fame-vs-lifestyle "image" penalty has held,
    /// so the tabloid news line fires once per streak rather than every
    /// week it continues (v0.7 §B).
    #[serde(default)]
    pub tabloid_streak: u32,
}

fn default_happiness() -> u8 {
    constants::DEFAULT_HAPPINESS
}

fn default_creativity() -> u8 {
    constants::DEFAULT_CREATIVITY
}

/// Where the player lives — the original 1989 game charged rent every
/// week and made your home part of who you were (v0.7 design §B).
/// Declaration order is the ladder order: comparisons (`<`, `>`) via the
/// derived `Ord` tell a move up from a move down.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize, Default)]
pub enum LifestyleTier {
    #[default]
    Squat,
    SharedFlat,
    CityApartment,
    Townhouse,
    Mansion,
}

impl LifestyleTier {
    /// One-shot happiness gain on a voluntary move up (§B).
    pub const MOVE_UP_HAPPINESS: u8 = constants::LIFESTYLE_MOVE_UP_HAPPINESS;
    /// One-shot happiness loss on a voluntary move down (§B).
    pub const MOVE_DOWN_HAPPINESS: u8 = constants::LIFESTYLE_MOVE_DOWN_HAPPINESS;

    /// Every tier, cheapest to grandest — the ladder order.
    pub const ALL: [LifestyleTier; 5] = [
        LifestyleTier::Squat,
        LifestyleTier::SharedFlat,
        LifestyleTier::CityApartment,
        LifestyleTier::Townhouse,
        LifestyleTier::Mansion,
    ];

    pub fn label(&self) -> &'static str {
        match self {
            LifestyleTier::Squat => "Squat",
            LifestyleTier::SharedFlat => "Shared flat",
            LifestyleTier::CityApartment => "City apartment",
            LifestyleTier::Townhouse => "Townhouse",
            LifestyleTier::Mansion => "Mansion",
        }
    }

    fn index(&self) -> usize {
        match self {
            LifestyleTier::Squat => 0,
            LifestyleTier::SharedFlat => 1,
            LifestyleTier::CityApartment => 2,
            LifestyleTier::Townhouse => 3,
            LifestyleTier::Mansion => 4,
        }
    }

    /// Weekly upkeep, deducted in the lifestyle tick (§B — tune table).
    pub fn upkeep_per_week(&self) -> u32 {
        constants::LIFESTYLE_UPKEEP_PER_WEEK[self.index()]
    }

    /// Added to `STRESS_PASSIVE_RELEASE` in the weekly tick.
    pub fn stress_release_bonus(&self) -> u8 {
        constants::LIFESTYLE_STRESS_RELEASE_BONUS[self.index()]
    }

    /// The weekly stress drain cannot pull happiness below this floor
    /// (event/incident losses still can).
    pub fn happiness_floor(&self) -> u8 {
        constants::LIFESTYLE_HAPPINESS_FLOOR[self.index()]
    }

    /// Added to the health/stress recovery of rest-type actions
    /// (`LazeAround`, `TakeBreak`).
    pub fn rest_healing_bonus(&self) -> u8 {
        constants::LIFESTYLE_REST_HEALING_BONUS[self.index()]
    }

    /// Total up-front cost to move up to *this* tier: the deposit plus
    /// the first week's rent (§B — Moving up).
    pub fn move_up_cost(&self) -> u32 {
        self.upkeep_per_week() * (constants::LIFESTYLE_MOVE_UP_DEPOSIT_WEEKS + 1)
    }

    /// The next tier down, or `None` already at the bottom (`Squat`).
    pub fn down(&self) -> Option<LifestyleTier> {
        self.index().checked_sub(1).map(|i| LifestyleTier::ALL[i])
    }
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
            lifestyle: LifestyleTier::default(),
            weeks_broke: 0,
            tabloid_streak: 0,
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
