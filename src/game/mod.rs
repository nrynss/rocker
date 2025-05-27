pub mod band;
pub mod events;
pub mod music;
pub mod player;
pub mod timeline;
pub mod world;

use crate::data::constants;
use crate::data_loader::GameDataFiles;
use band::Band;
use events::EventManager;
use player::Player;
use rand::Rng;
use timeline::MusicTimeline;
use world::GameWorld;

#[derive(Debug, Clone)]
pub enum GameAction {
    LazeAround,
    WriteSongs,
    Practice,
    RecordSingle,
    RecordAlbum,
    Gig,
    GoOnTour,
    TakeBreak,
    VisitDoctor,
    Quit,
}

pub struct Game {
    pub player: Player,
    pub band: Band,
    pub world: GameWorld,
    pub events: EventManager,
    pub timeline: MusicTimeline,
    pub data_files: GameDataFiles,
    pub week: u32,
    pub game_over: bool,
}

impl Game {
    pub fn new() -> Result<Self, Box<dyn std::error::Error>> {
        // Validate all data files exist first
        GameDataFiles::validate_data_files()?;

        let data_files = GameDataFiles::load()?;
        Ok(Self {
            player: Player::default(),
            band: Band::default(),
            world: GameWorld::new(&data_files),
            events: EventManager::new(),
            timeline: MusicTimeline::new(&data_files),
            data_files,
            week: 1,
            game_over: false,
        })
    }

    pub fn initialize_player(&mut self, player_name: &str, band_name: &str) {
        self.player.name = player_name.to_string();
        self.band.name = band_name.to_string();
        self.player.money = 500; // Starting cash in 1970

        // Generate band members with names from data files
        self.band.members = vec![
            band::BandMember {
                name: self.data_files.random_band_member_name(),
                instrument: band::Instrument::Guitar,
                skill: 25,
                loyalty: 75,
                drug_problem: false,
            },
            band::BandMember {
                name: self.data_files.random_band_member_name(),
                instrument: band::Instrument::Bass,
                skill: 20,
                loyalty: 80,
                drug_problem: false,
            },
            band::BandMember {
                name: self.data_files.random_band_member_name(),
                instrument: band::Instrument::Drums,
                skill: 30,
                loyalty: 70,
                drug_problem: false,
            },
        ];
    }

    pub fn process_turn(&mut self, action: GameAction) -> Result<bool, String> {
        if self.game_over {
            return Ok(false);
        }

        // Process the chosen action
        self.execute_action(action)?;

        // Advance time - check if we need to advance to next year
        self.week += 1;
        if self.week % constants::WEEKS_PER_YEAR == 0 {
            self.timeline.advance_year();
        }

        // Process random events
        if let Some(event) = self.events.try_trigger_event(self.week) {
            self.apply_random_event(event)?;
        }

        // Apply weekly health decay
        self.player.weekly_health_decay();

        // Check for historical events
        if let Some(historical_event) = self.timeline.should_trigger_historical_event() {
            self.apply_historical_event(&historical_event)?;
        }

        // Update world state
        self.world.update_week(&self.timeline);

        // Check game over conditions
        self.check_game_over();

        Ok(!self.game_over)
    }

    fn execute_action(&mut self, action: GameAction) -> Result<(), String> {
        match action {
            GameAction::LazeAround => {
                self.player.energy = (self.player.energy + 20).min(constants::MAX_ENERGY);
                self.player.stress = (self.player.stress.saturating_sub(10)).max(0);
            }
            GameAction::WriteSongs => {
                if self.player.energy < 20 {
                    return Err("You're too tired to write songs!".to_string());
                }
                self.player.energy -= 20;
                let songs_written = rand::thread_rng().gen_range(1..=3);
                self.band.unreleased_songs += songs_written;

                // Songs are influenced by current era
                let era_bonus = if self.timeline.get_innovation_bonus() > 70 {
                    1
                } else {
                    0
                };
                self.band.unreleased_songs += era_bonus;
            }
            GameAction::Practice => {
                if self.player.energy < 15 {
                    return Err("You're too tired to practice!".to_string());
                }
                self.player.energy -= 15;
                self.band.skill = (self.band.skill + 2).min(constants::MAX_SKILL);
            }
            GameAction::RecordSingle => {
                if !self.band.can_record_single() {
                    return Err("You need to write songs first!".to_string());
                }
                let cost = ((constants::SINGLE_RECORDING_COST as f32)
                    * self.timeline.get_recording_cost_modifier())
                    as i32;
                if !self.player.can_afford(cost) {
                    return Err(format!("You need at least ${} to record a single!", cost));
                }
                self.player.spend_money(cost);
                self.band.unreleased_songs -= 1;
                self.band.singles += 1;
            }
            GameAction::RecordAlbum => {
                if !self.band.can_record_album() {
                    return Err(format!(
                        "You need at least {} songs to record an album!",
                        constants::MIN_ALBUM_SONGS
                    ));
                }
                let cost = ((constants::ALBUM_RECORDING_BASE_COST as f32)
                    * self.timeline.get_recording_cost_modifier())
                    as i32;
                if !self.player.can_afford(cost) {
                    return Err(format!("You need at least ${} to record an album!", cost));
                }
                self.player.spend_money(cost);
                self.band.unreleased_songs -= constants::MIN_ALBUM_SONGS;
                self.band.albums += 1;

                // Albums are more valuable in album-focused eras
                if self.timeline.is_album_era() {
                    self.band.fame = (self.band.fame + 3).min(constants::MAX_FAME);
                }
            }
            GameAction::Gig => {
                if self.player.energy < 30 {
                    return Err("You're too tired to perform!".to_string());
                }
                self.player.energy -= 30;
                let earnings = self.calculate_gig_earnings();
                self.player.earn_money(earnings);
                self.band.fame = (self.band.fame + 1).min(constants::MAX_FAME);
            }
            GameAction::GoOnTour => {
                if self.player.energy < 40 {
                    return Err("You're too tired to go on tour!".to_string());
                }
                if self.band.fame < 25 {
                    return Err("You need more fame before promoters will book a tour!".to_string());
                }
                if !self.player.can_afford(2000) {
                    return Err("You need at least $2000 to finance a tour!".to_string());
                }

                // Tour costs and duration based on fame level
                let (tour_cost, tour_earnings, tour_weeks, fame_gain) = if self.band.fame >= 70 {
                    // International tour
                    (5000, 15000 + (self.band.fame as i32 * 200), 4, 8)
                } else if self.band.fame >= 50 {
                    // National tour
                    (3000, 8000 + (self.band.fame as i32 * 100), 3, 5)
                } else {
                    // Regional tour
                    (2000, 4000 + (self.band.fame as i32 * 50), 2, 3)
                };

                // Apply era and market modifiers
                let era_modifier = self.timeline.get_gig_pay_modifier();
                let market_modifier = self.world.get_market_modifier();
                let final_earnings =
                    ((tour_earnings as f32) * era_modifier * market_modifier) as i32;

                self.player.spend_money(tour_cost);
                self.player.earn_money(final_earnings as u32);
                self.player.energy -= 40;
                self.player.stress = (self.player.stress + 30).min(constants::MAX_STRESS);
                self.band.fame = (self.band.fame + fame_gain).min(constants::MAX_FAME);

                // Advance time for tour duration
                self.week += tour_weeks;

                // Random tour events
                let mut rng = rand::thread_rng();
                if rng.gen_bool(0.3) {
                    // Positive tour event
                    self.band.fame = (self.band.fame + 2).min(constants::MAX_FAME);
                } else if rng.gen_bool(0.15) {
                    // Negative tour event
                    self.player.health = self.player.health.saturating_sub(10);
                }
            }
            GameAction::TakeBreak => {
                self.player.energy = constants::MAX_ENERGY;
                self.player.stress = 0;
                self.player.health = (self.player.health + 10).min(constants::MAX_HEALTH);
            }
            GameAction::VisitDoctor => {
                if !self.player.can_afford(constants::DOCTOR_VISIT_COST) {
                    return Err(format!(
                        "You need ${} to visit the doctor!",
                        constants::DOCTOR_VISIT_COST
                    ));
                }
                self.player.spend_money(constants::DOCTOR_VISIT_COST);
                self.player.health = (self.player.health + 20).min(constants::MAX_HEALTH);
            }
            GameAction::Quit => {
                self.game_over = true;
            }
        }
        Ok(())
    }

    fn calculate_gig_earnings(&self) -> u32 {
        let base_pay = 50u32;
        let fame_bonus = (self.band.fame as u32) / 10;
        let skill_bonus = (self.band.skill as u32) / 20;
        base_pay + fame_bonus + skill_bonus
    }

    fn check_game_over(&mut self) {
        if self.player.health <= 0 {
            self.game_over = true;
        }
        if self.player.money < 0 && self.band.fame < 10 {
            self.game_over = true;
        }
        // Win condition: become a rockstar
        if self.band.fame >= constants::ROCKSTAR_FAME_THRESHOLD
            && self.band.albums >= constants::ROCKSTAR_ALBUM_THRESHOLD
        {
            self.game_over = true;
        }
    }

    pub fn is_game_over(&self) -> bool {
        self.game_over
    }

    pub fn get_status_message(&self) -> String {
        if self.player.health <= 0 {
            "You died from poor health!".to_string()
        } else if self.player.money < 0 && self.band.fame < 10 {
            "You went broke and nobody knows who you are!".to_string()
        } else if self.band.fame >= constants::ROCKSTAR_FAME_THRESHOLD
            && self.band.albums >= constants::ROCKSTAR_ALBUM_THRESHOLD
        {
            "Congratulations! You're now a ROCKSTAR!".to_string()
        } else {
            "Game continues...".to_string()
        }
    }

    fn apply_random_event(&mut self, event: events::RandomEvent) -> Result<(), String> {
        use events::RandomEvent;
        let mut rng = rand::thread_rng();

        match event {
            RandomEvent::DrugOffer => {
                if rng.gen_bool(0.3) {
                    self.player.energy = (self.player.energy + 20).min(constants::MAX_ENERGY);
                    self.player.drug_addiction =
                        (self.player.drug_addiction + 10).min(constants::MAX_STRESS);
                    self.player.health = self.player.health.saturating_sub(5);
                }
            }
            RandomEvent::EquipmentIssue => {
                match rng.gen_range(0..3) {
                    0 => {
                        let repair_cost = rng.gen_range(
                            constants::EQUIPMENT_REPAIR_COST_RANGE.0
                                ..=constants::EQUIPMENT_REPAIR_COST_RANGE.1,
                        );
                        if self.player.can_afford(repair_cost) {
                            self.player.spend_money(repair_cost);
                        } else {
                            // Can't afford repair - equipment stays broken, skill decreases
                            self.band.skill = self.band.skill.saturating_sub(5);
                        }
                    }
                    1 => {
                        self.band.skill = (self.band.skill + 5).min(constants::MAX_SKILL);
                    }
                    _ => {
                        let loss = rng.gen_range(100..500);
                        if self.player.can_afford(loss) {
                            self.player.spend_money(loss);
                        } else {
                            // Lose all money if can't afford full loss
                            self.player.money = 0;
                        }
                        self.band.skill = self.band.skill.saturating_sub(3);
                    }
                }
            }
            RandomEvent::BandMemberIssue => {
                if !self.band.members.is_empty() {
                    let member_idx = rng.gen_range(0..self.band.members.len());
                    let member = &mut self.band.members[member_idx];

                    match rng.gen_range(0..4) {
                        0 => {
                            member.skill = (member.skill + 5).min(100);
                            member.loyalty = (member.loyalty + 10).min(100);
                        }
                        1 => {
                            member.loyalty = member.loyalty.saturating_sub(15);
                            if rng.gen_bool(0.3) {
                                member.drug_problem = true;
                            }
                        }
                        2 => {
                            if member.loyalty < 30 {
                                member.loyalty = 0;
                            }
                        }
                        _ => {
                            let demand = rng.gen_range(100..300);
                            self.player.money -= demand;
                        }
                    }
                }
            }
            RandomEvent::MediaEvent => match rng.gen_range(0..3) {
                0 => {
                    self.band.fame =
                        (self.band.fame + rng.gen_range(3..8)).min(constants::MAX_FAME);
                    self.band.reputation.media_presence =
                        (self.band.reputation.media_presence + 5).min(100);
                }
                1 => {
                    self.band.fame = self.band.fame.saturating_sub(rng.gen_range(2..6));
                    self.band.reputation.media_presence =
                        self.band.reputation.media_presence.saturating_sub(8);
                }
                _ => {
                    self.band.fame = self.band.fame.saturating_sub(rng.gen_range(5..15));
                    self.player.stress = (self.player.stress + 20).min(constants::MAX_STRESS);
                }
            },
            RandomEvent::HealthEvent => match rng.gen_range(0..3) {
                0 => {
                    self.player.health = self.player.health.saturating_sub(rng.gen_range(10..25));
                    self.player.energy = self.player.energy.saturating_sub(30);
                }
                1 => {
                    self.player.health = self.player.health.saturating_sub(rng.gen_range(5..15));
                    self.band.skill = self.band.skill.saturating_sub(5);
                }
                _ => {
                    self.player.stress =
                        (self.player.stress + rng.gen_range(15..30)).min(constants::MAX_STRESS);
                    self.player.energy = self.player.energy.saturating_sub(20);
                }
            },
            RandomEvent::MoneyEvent => {
                match rng.gen_range(0..4) {
                    0 => {
                        let amount = rng.gen_range(200..1000);
                        self.player.earn_money(amount as u32);
                    }
                    1 => {
                        let amount = rng.gen_range(100..500);
                        if self.player.can_afford(amount) {
                            self.player.spend_money(amount);
                        } else {
                            self.player.money = 0; // Lose all money
                        }
                    }
                    2 => {
                        let royalties = (self.band.total_releases() as i32) * rng.gen_range(10..50);
                        self.player.earn_money(royalties as u32);
                    }
                    _ => {
                        let cost = rng.gen_range(500..2000);
                        if self.player.can_afford(cost) {
                            self.player.spend_money(cost);
                        } else {
                            self.player.money = 0; // Bankruptcy
                        }
                        self.band.fame = self.band.fame.saturating_sub(5);
                    }
                }
            }
            RandomEvent::IndustryEvent => match rng.gen_range(0..3) {
                0 => {
                    if !self.band.has_record_deal() && self.band.fame > 30 {
                        self.band.fame = (self.band.fame + 5).min(constants::MAX_FAME);
                    }
                }
                1 => {
                    if self.band.fame > 20 {
                        let payment = rng.gen_range(500..2000);
                        self.player.earn_money(payment as u32);
                        self.band.fame = (self.band.fame + 3).min(constants::MAX_FAME);
                    }
                }
                _ => {}
            },
        }

        Ok(())
    }

    fn apply_historical_event(&mut self, event: &str) -> Result<(), String> {
        let mut rng = rand::thread_rng();

        // Apply effects based on the type of historical event
        match event {
            event if event.contains("Beatles") => {
                if self.band.dominant_genres_match(&["Rock", "Folk Rock"]) {
                    self.band.fame = (self.band.fame + 5).min(constants::MAX_FAME);
                    self.player.money += 200;
                }
            }
            event if event.contains("MTV") => {
                if self.timeline.get_image_importance() > 80 {
                    if self.band.reputation.media_presence > 60 {
                        self.band.fame = (self.band.fame + 10).min(constants::MAX_FAME);
                        let earnings = rng.gen_range(1000..3000);
                        self.player.money += earnings;
                    } else {
                        self.band.fame = self.band.fame.saturating_sub(5);
                    }
                }
            }
            event if event.contains("Grunge emerges") => {
                if self.band.dominant_genres_match(&["Grunge", "Alternative"]) {
                    self.band.fame = (self.band.fame + 12).min(constants::MAX_FAME);
                    let major_earnings = rng.gen_range(2000..5000);
                    self.player.money += major_earnings;
                } else if self
                    .band
                    .dominant_genres_match(&["Hair Metal", "Pop Metal"])
                {
                    self.band.fame = self.band.fame.saturating_sub(8);
                }
            }
            _ => {
                // Generic historical event
                match rng.gen_range(0..3) {
                    0 => self.band.fame = (self.band.fame + 1).min(constants::MAX_FAME),
                    1 => self.player.money += rng.gen_range(50..200),
                    _ => {
                        self.band.reputation.critical_acclaim =
                            (self.band.reputation.critical_acclaim + 1).min(100)
                    }
                }
            }
        }

        Ok(())
    }

    pub fn get_last_historical_event(&self) -> Option<String> {
        self.timeline.should_trigger_historical_event()
    }
}
