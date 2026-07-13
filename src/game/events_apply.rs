//! Event outcomes: applies random and historical events to the game state.
//! Trigger selection lives in `events.rs`; week orchestration stays in `turn.rs`.

use super::constants;
use super::*;

impl Game {
    pub(super) fn apply_random_event(
        &mut self,
        event: events::RandomEvent,
        rng: &mut impl Rng,
    ) -> Result<(), String> {
        use events::RandomEvent;

        match event {
            RandomEvent::DrugOffer => {
                if rng.gen_bool(0.3) {
                    self.player.energy = (self.player.energy + 20).min(constants::MAX_ENERGY);
                    self.player.drug_addiction =
                        (self.player.drug_addiction + 10).min(constants::MAX_STRESS);
                    self.player.health = self.player.health.saturating_sub(5);
                    self.log(
                        "🍾 You partied with the wrong crowd — you're wired, but at what cost…",
                    );
                } else {
                    self.log("🚫 Someone offered you 'a little help' backstage. You passed.");
                }
            }
            RandomEvent::EquipmentIssue => match rng.gen_range(0..3) {
                0 => {
                    let repair_cost = rng.gen_range(
                        constants::EQUIPMENT_REPAIR_COST_RANGE.0
                            ..=constants::EQUIPMENT_REPAIR_COST_RANGE.1,
                    );
                    if self.player.can_afford(repair_cost) {
                        self.player.spend_money(repair_cost);
                        self.log(format!(
                            "🔧 Your amp blew mid-set — ${} in repairs.",
                            repair_cost
                        ));
                    } else {
                        self.band.skill = self.band.skill.saturating_sub(5);
                        self.log("🔧 Your amp blew and you can't afford repairs — the band sounds rougher.");
                    }
                }
                1 => {
                    self.band.skill = (self.band.skill + 5).min(constants::MAX_SKILL);
                    self.log("🎸 A pawn-shop find! New gear tightens up your sound (+5 skill).");
                }
                _ => {
                    let loss = rng.gen_range(100..500);
                    if self.player.can_afford(loss) {
                        self.player.spend_money(loss);
                        self.log(format!(
                            "🚨 Gear stolen from the van — ${} to replace it.",
                            loss
                        ));
                    } else {
                        self.player.money = 0;
                        self.log("🚨 Gear stolen from the van — it cleaned you out.");
                    }
                    self.band.skill = self.band.skill.saturating_sub(3);
                }
            },
            RandomEvent::BandMemberIssue => {
                if !self.band.members.is_empty() {
                    let member_idx = rng.gen_range(0..self.band.members.len());
                    let roll = rng.gen_range(0..4);
                    let develops_problem = roll == 1 && rng.gen_bool(0.3);
                    let demand = rng.gen_range(100..300);

                    let member = &mut self.band.members[member_idx];
                    let name = member.name.clone();
                    match roll {
                        0 => {
                            member.skill = (member.skill + 5).min(100);
                            member.loyalty = (member.loyalty + 10).min(100);
                            self.log(format!(
                                "🌟 {} has been woodshedding — sharper than ever.",
                                name
                            ));
                        }
                        1 => {
                            member.loyalty = member.loyalty.saturating_sub(15);
                            if develops_problem {
                                member.drug_problem = true;
                                self.log(format!(
                                    "😠 {} is unhappy with the band's direction — and partying way too hard.",
                                    name
                                ));
                            } else {
                                self.log(format!(
                                    "😠 {} is unhappy with the band's direction.",
                                    name
                                ));
                            }
                        }
                        2 => {
                            if member.loyalty < 30 {
                                member.loyalty = 0;
                                self.log(format!("🚪 {} is threatening to quit!", name));
                            }
                        }
                        _ => {
                            self.player.money -= demand;
                            self.log(format!(
                                "💸 {} demands a bigger cut — ${} to keep the peace.",
                                name, demand
                            ));
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
                    self.log("📰 A glowing review in the music press — your profile rises.");
                }
                1 => {
                    self.band.fame = self.band.fame.saturating_sub(rng.gen_range(2..6));
                    self.band.reputation.media_presence =
                        self.band.reputation.media_presence.saturating_sub(8);
                    self.log("📰 A critic tears your latest show apart. Ouch.");
                }
                _ => {
                    self.band.fame = self.band.fame.saturating_sub(rng.gen_range(5..15));
                    self.player.stress = (self.player.stress + 20).min(constants::MAX_STRESS);
                    self.log("🔥 SCANDAL! The tabloids are all over you — fame takes a hit.");
                }
            },
            RandomEvent::HealthEvent => match rng.gen_range(0..3) {
                0 => {
                    self.player.health = self.player.health.saturating_sub(rng.gen_range(10..25));
                    self.player.energy = self.player.energy.saturating_sub(30);
                    self.log("🤒 You've caught something nasty — health and energy suffer.");
                }
                1 => {
                    self.player.health = self.player.health.saturating_sub(rng.gen_range(5..15));
                    self.band.skill = self.band.skill.saturating_sub(5);
                    self.log("🤕 Stage dive gone wrong — you're hurt, and rehearsals suffer.");
                }
                _ => {
                    self.player.stress =
                        (self.player.stress + rng.gen_range(15..30)).min(constants::MAX_STRESS);
                    self.player.energy = self.player.energy.saturating_sub(20);
                    self.log("😰 The pressure is getting to you — stress climbs.");
                }
            },
            RandomEvent::MoneyEvent => {
                match rng.gen_range(0..4) {
                    0 => {
                        let amount = rng.gen_range(200..1000);
                        self.player.earn_money(amount as u32);
                        self.log(format!("💰 Unexpected windfall: ${}!", amount));
                    }
                    1 => {
                        let amount = rng.gen_range(100..500);
                        if self.player.can_afford(amount) {
                            self.player.spend_money(amount);
                        } else {
                            self.player.money = 0;
                        }
                        self.log(format!(
                            "💸 A surprise bill lands on the doormat: ${}.",
                            amount
                        ));
                    }
                    2 => {
                        // Simplified: Royalty for *all* past releases, not just current one.
                        let total_releases_count =
                            self.band.albums_released.len() + self.band.singles_released.len();
                        let royalties = (total_releases_count as i32) * rng.gen_range(10..50);
                        self.player.earn_money(royalties as u32);
                        if royalties > 0 {
                            self.log(format!("💵 A royalty check arrives: ${}.", royalties));
                        }
                    }
                    _ => {
                        let cost = rng.gen_range(500..2000);
                        if self.player.can_afford(cost) {
                            self.player.spend_money(cost);
                        } else {
                            self.player.money = 0;
                        }
                        self.band.fame = self.band.fame.saturating_sub(5);
                        self.log(format!(
                            "⚖️ Legal trouble costs you ${} and some reputation.",
                            cost
                        ));
                    }
                }
            }
            RandomEvent::IndustryEvent => match rng.gen_range(0..3) {
                0 if !self.band.has_record_deal() && self.band.fame > 30 => {
                    self.band.fame = (self.band.fame + 5).min(constants::MAX_FAME);
                    self.log("👀 A&R scouts were spotted at your show — industry buzz grows.");
                }
                1 if self.band.fame > 20 => {
                    let payment = rng.gen_range(500..2000);
                    self.player.earn_money(payment as u32);
                    self.band.fame = (self.band.fame + 3).min(constants::MAX_FAME);
                    self.log(format!(
                        "🎪 A festival slot opens up — ${} and more fans.",
                        payment
                    ));
                }
                _ => {}
            },
        }

        Ok(())
    }

    pub(super) fn apply_historical_event(
        &mut self,
        event: &str,
        rng: &mut impl Rng,
    ) -> Result<(), String> {
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
            _ => match rng.gen_range(0..3) {
                0 => self.band.fame = (self.band.fame + 1).min(constants::MAX_FAME),
                1 => self.player.money += rng.gen_range(50..200),
                _ => {
                    self.band.reputation.critical_acclaim =
                        (self.band.reputation.critical_acclaim + 1).min(100)
                }
            },
        }

        Ok(())
    }
}
