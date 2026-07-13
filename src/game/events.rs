use rand::{thread_rng, Rng};
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum RandomEvent {
    DrugOffer,
    EquipmentIssue,
    BandMemberIssue,
    MediaEvent,
    HealthEvent,
    MoneyEvent,
    IndustryEvent,
}

#[derive(Serialize, Deserialize, Debug)]
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
}
