use crate::game::Game;
use rand::{thread_rng, Rng};

#[derive(Debug, Clone)]
pub enum RandomEvent {
    DrugOffer,
    EquipmentIssue,
    BandMemberIssue,
    MediaEvent,
    HealthEvent,
    MoneyEvent,
    IndustryEvent,
}

pub struct EventManager {
    pub last_event_week: u32,
}

impl EventManager {
    pub fn new() -> Self {
        Self { last_event_week: 0 }
    }

    pub fn should_process_events(&self, current_week: u32) -> bool {
        current_week - self.last_event_week >= 2
    }

    pub fn try_trigger_event(&mut self, current_week: u32) -> Option<RandomEvent> {
        if !self.should_process_events(current_week) {
            return None;
        }

        let mut rng = thread_rng();

        // 30% chance of a random event each eligible week
        if rng.gen_range(0..100) < 30 {
            self.last_event_week = current_week;
            Some(self.generate_random_event(&mut rng))
        } else {
            None
        }
    }

    fn generate_random_event(&self, rng: &mut impl Rng) -> RandomEvent {
        let event_type = rng.gen_range(0..10);

        match event_type {
            0..=2 => RandomEvent::DrugOffer,
            3..=4 => RandomEvent::EquipmentIssue,
            5 => RandomEvent::BandMemberIssue,
            6 => RandomEvent::MediaEvent,
            7 => RandomEvent::HealthEvent,
            8 => RandomEvent::MoneyEvent,
            _ => RandomEvent::IndustryEvent,
        }
    }

    fn drug_offer_event(&self, game: &mut Game) -> Result<(), String> {
        let mut rng = thread_rng();
        let drug_types = ["cocaine", "heroin", "speed", "marijuana", "alcohol"];
        let drug = drug_types[rng.gen_range(0..drug_types.len())];

        // For now, just apply random effects
        // TODO: Add player choice system
        if rng.gen_bool(0.3) {
            // 30% chance player "accepts"
            match drug {
                "cocaine" | "speed" => {
                    game.player.energy = (game.player.energy + 30).min(100);
                    game.player.drug_addiction = (game.player.drug_addiction + 10).min(100);
                    game.player.health = game.player.health.saturating_sub(5);
                }
                "heroin" => {
                    game.player.stress = game.player.stress.saturating_sub(30);
                    game.player.drug_addiction = (game.player.drug_addiction + 20).min(100);
                    game.player.health = game.player.health.saturating_sub(15);
                }
                "marijuana" => {
                    game.player.stress = game.player.stress.saturating_sub(15);
                    game.player.drug_addiction = (game.player.drug_addiction + 5).min(100);
                    game.player.health = game.player.health.saturating_sub(2);
                }
                "alcohol" => {
                    game.player.stress = game.player.stress.saturating_sub(10);
                    game.player.alcohol_addiction = (game.player.alcohol_addiction + 8).min(100);
                    game.player.health = game.player.health.saturating_sub(3);
                }
                _ => {}
            }
        }

        Ok(())
    }

    fn equipment_event(&self, game: &mut Game) -> Result<(), String> {
        let mut rng = thread_rng();

        match rng.gen_range(0..3) {
            0 => {
                // Equipment breaks
                let repair_cost = rng.gen_range(50..200);
                game.player.money -= repair_cost;
            }
            1 => {
                // Found/given equipment
                game.band.skill = (game.band.skill + 5).min(100);
            }
            _ => {
                // Stolen equipment
                let loss = rng.gen_range(100..500);
                game.player.money -= loss;
                game.band.skill = game.band.skill.saturating_sub(3);
            }
        }

        Ok(())
    }

    fn band_member_event(&self, game: &mut Game) -> Result<(), String> {
        let mut rng = thread_rng();

        if !game.band.members.is_empty() {
            let member_idx = rng.gen_range(0..game.band.members.len());
            let member = &mut game.band.members[member_idx];

            match rng.gen_range(0..4) {
                0 => {
                    // Member gets better
                    member.skill = (member.skill + 5).min(100);
                    member.loyalty = (member.loyalty + 10).min(100);
                }
                1 => {
                    // Member has problems
                    member.loyalty = member.loyalty.saturating_sub(15);
                    if rng.gen_bool(0.3) {
                        member.drug_problem = true;
                    }
                }
                2 => {
                    // Member quits (if loyalty is low)
                    if member.loyalty < 30 {
                        // TODO: Implement member replacement system
                        member.loyalty = 0; // Mark as problematic for now
                    }
                }
                _ => {
                    // Member demands more money
                    let demand = rng.gen_range(100..300);
                    game.player.money -= demand;
                }
            }
        }

        Ok(())
    }

    fn media_event(&self, game: &mut Game) -> Result<(), String> {
        let mut rng = thread_rng();

        match rng.gen_range(0..3) {
            0 => {
                // Good press
                game.band.fame = (game.band.fame + rng.gen_range(3..8)).min(100);
                game.band.reputation.media_presence =
                    (game.band.reputation.media_presence + 5).min(100);
            }
            1 => {
                // Bad press
                game.band.fame = game.band.fame.saturating_sub(rng.gen_range(2..6));
                game.band.reputation.media_presence =
                    game.band.reputation.media_presence.saturating_sub(8);
            }
            _ => {
                // Scandal
                game.band.fame = game.band.fame.saturating_sub(rng.gen_range(5..15));
                game.player.stress = (game.player.stress + 20).min(100);
            }
        }

        Ok(())
    }

    fn health_event(&self, game: &mut Game) -> Result<(), String> {
        let mut rng = thread_rng();

        match rng.gen_range(0..3) {
            0 => {
                // Illness
                game.player.health = game.player.health.saturating_sub(rng.gen_range(10..25));
                game.player.energy = game.player.energy.saturating_sub(30);
            }
            1 => {
                // Injury during performance
                game.player.health = game.player.health.saturating_sub(rng.gen_range(5..15));
                game.band.skill = game.band.skill.saturating_sub(5);
            }
            _ => {
                // Mental health issues
                game.player.stress = (game.player.stress + rng.gen_range(15..30)).min(100);
                game.player.energy = game.player.energy.saturating_sub(20);
            }
        }

        Ok(())
    }

    fn money_event(&self, game: &mut Game) -> Result<(), String> {
        let mut rng = thread_rng();

        match rng.gen_range(0..4) {
            0 => {
                // Unexpected windfall
                let amount = rng.gen_range(200..1000);
                game.player.money += amount;
            }
            1 => {
                // Unexpected expense
                let amount = rng.gen_range(100..500);
                game.player.money -= amount;
            }
            2 => {
                // Royalty payment
                let royalties = game.band.total_releases() as i32 * rng.gen_range(10..50);
                game.player.money += royalties;
            }
            _ => {
                // Lawsuit or legal trouble
                let cost = rng.gen_range(500..2000);
                game.player.money -= cost;
                game.band.fame = game.band.fame.saturating_sub(5);
            }
        }

        Ok(())
    }

    fn industry_event(&self, game: &mut Game) -> Result<(), String> {
        let mut rng = thread_rng();

        match rng.gen_range(0..3) {
            0 => {
                // Record label interest
                if !game.band.has_record_deal() && game.band.fame > 30 {
                    // TODO: Implement record deal offer system
                    game.band.fame = (game.band.fame + 5).min(100);
                }
            }
            1 => {
                // Festival invitation
                if game.band.fame > 20 {
                    let payment = rng.gen_range(500..2000);
                    game.player.money += payment;
                    game.band.fame = (game.band.fame + 3).min(100);
                }
            }
            _ => {
                // Industry changes affecting all bands
                // This could affect the global market conditions
            }
        }

        Ok(())
    }
}
