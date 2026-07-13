//! Player weekly actions (split by concern). Methods remain on `Game`.

use crate::game::music::ReleaseType;
use rand::Rng;

use super::super::constants::{self, *};
use super::super::*;

impl Game {
    pub fn get_sorted_regions(&self) -> Vec<(String, String, String, u32, u8, u8)> {
        let mut result = Vec::new();
        let markets_data = &self.data_files.markets_data;

        let mut countries: Vec<String> = markets_data.markets.keys().cloned().collect();
        countries.sort();

        for country in countries {
            if let Some(c_market) = markets_data.markets.get(&country) {
                let mut regions: Vec<String> = c_market.regions.keys().cloned().collect();
                regions.sort();
                for r_key in regions {
                    if let Some(r_market) = c_market.regions.get(&r_key) {
                        let fame_req = if r_market.population < 3_000_000 {
                            25
                        } else if r_market.population < 7_000_000 {
                            35
                        } else if r_market.population < 10_000_000 {
                            45
                        } else if r_market.population < 15_000_000 {
                            55
                        } else {
                            70
                        };
                        result.push((
                            country.clone(),
                            r_key.clone(),
                            r_market.name.clone(),
                            r_market.population,
                            r_market.economic_strength,
                            fame_req,
                        ));
                    }
                }
            }
        }
        result
    }

    /// How famous live performance alone can make you. Every record in the
    /// catalog — including one still in its sales window — lifts the ceiling.
    fn live_fame_cap(&self) -> u8 {
        let mut singles = self.band.singles_released.len();
        let mut albums = self.band.albums_released.len();
        for release in &self.just_released_music {
            match release.release_type {
                ReleaseType::Single => singles += 1,
                ReleaseType::Album => albums += 1,
            }
        }
        (LIVE_FAME_BASE_CAP as usize
            + singles * LIVE_FAME_PER_SINGLE as usize
            + albums * LIVE_FAME_PER_ALBUM as usize)
            .min(constants::MAX_FAME as usize) as u8
    }

    pub(in crate::game) fn action_play_gig(&mut self, venue_index: usize) -> Result<(), String> {
        if self.player.energy < 30 {
            return Err("You're too tired to perform!".to_string());
        }
        if venue_index >= self.world.venues.len() {
            return Err("Invalid venue selected.".to_string());
        }
        let venue = &self.world.venues[venue_index];
        if venue.prestige > self.band.fame.saturating_add(20) {
            return Err(format!(
                "'{}' is out of your league! Get more famous first.",
                venue.name
            ));
        }

        self.player.energy -= 30;

        let era_modifier = self.timeline.get_gig_pay_modifier();
        let market_modifier = self.world.get_market_modifier();

        let attendance_ratio =
            ((self.band.fame as f32 + 10.0) / (venue.prestige as f32 + 10.0)).min(1.0);
        let attendance = (venue.capacity as f32 * attendance_ratio) as u32;

        let earnings =
            (venue.base_payment as f32 * attendance_ratio * market_modifier * era_modifier) as u32;

        let base_fame_gain = if venue.capacity <= 200 {
            1
        } else if venue.capacity <= 2000 {
            2
        } else {
            3
        };
        let fame_gain = if attendance_ratio < 0.5 {
            (base_fame_gain / 2).max(1)
        } else {
            base_fame_gain
        };

        let venue_ceiling = venue.prestige.saturating_add(VENUE_FAME_HEADROOM);
        let live_cap = self.live_fame_cap();
        let headroom = venue_ceiling.min(live_cap).saturating_sub(self.band.fame);
        let fame_gain = fame_gain.min(headroom);

        self.player.earn_money(earnings);
        self.band.fame = (self.band.fame + fame_gain).min(constants::MAX_FAME);
        if fame_gain > 0 {
            self.log(format!(
                "🎤 Played at '{}' — sold {}/{} tickets, earned ${}, fame +{}.",
                venue.name, attendance, venue.capacity, earnings, fame_gain
            ));
        } else if self.band.fame >= live_cap {
            self.log(format!(
                "🎤 Played at '{}' — sold {}/{} tickets, earned ${}. The buzz has peaked — without new records, word of mouth carries no further.",
                venue.name, attendance, venue.capacity, earnings
            ));
        } else {
            self.log(format!(
                "🎤 Played at '{}' — sold {}/{} tickets, earned ${}. The regulars know every word — you've outgrown this stage.",
                venue.name, attendance, venue.capacity, earnings
            ));
        }
        Ok(())
    }

    pub(in crate::game) fn action_go_on_tour(
        &mut self,
        region_index: usize,
        rng: &mut impl Rng,
    ) -> Result<(), String> {
        if self.player.energy < 40 {
            return Err("You're too tired to go on tour!".to_string());
        }

        let sorted_regions = self.get_sorted_regions();
        if region_index >= sorted_regions.len() {
            return Err("Invalid region selected.".to_string());
        }

        let (country_key, region_key, region_name, population, economic_strength, fame_req) =
            &sorted_regions[region_index];

        if self.band.fame < *fame_req {
            return Err(format!(
                "Your band needs at least {} fame to tour '{}'.",
                fame_req, region_name
            ));
        }

        let tier_name = if self.band.fame < 35 {
            "local"
        } else if self.band.fame < 60 {
            "regional"
        } else if self.band.fame < 80 {
            "national"
        } else {
            "international"
        };

        let touring_costs = self
            .data_files
            .markets_data
            .market_modifiers
            .touring_costs
            .get(tier_name)
            .ok_or_else(|| "Touring cost tier not found.".to_string())?;

        let country_travel_mult = match country_key.as_str() {
            "united_states" => 1.5,
            "united_kingdom" => 0.8,
            "europe" => 1.2,
            "japan" => 1.0,
            "australia" => 1.4,
            _ => 1.0,
        };

        let tour_cost = (touring_costs.base_cost_per_show as f32 * country_travel_mult) as i32;

        if !self.player.can_afford(tour_cost) {
            return Err(format!(
                "You need at least ${} to finance this tour!",
                tour_cost
            ));
        }

        let (tour_weeks, fame_gain) = if self.band.fame >= 80 {
            (4, 10)
        } else if self.band.fame >= 60 {
            (3, 6)
        } else if self.band.fame >= 35 {
            (2, 4)
        } else {
            (2, 3)
        };

        let regional_fame_key = format!("{}:{}", country_key, region_key);
        let regional_fame = *self.regional_fame.get(&regional_fame_key).unwrap_or(&0);

        let audience = (self.band.fame as f32 / 3.0) + (regional_fame as f32);
        let base_gross =
            (*population as f32).sqrt() * (*economic_strength as f32 / 100.0) * audience * 0.06;

        let era_modifier = self.timeline.get_gig_pay_modifier();
        let market_modifier = self.world.get_market_modifier();
        let final_earnings = (base_gross * era_modifier * market_modifier) as i32;

        self.player.spend_money(tour_cost);
        self.player.earn_money(final_earnings as u32);
        self.player.energy -= 40;
        self.player.stress = (self.player.stress + 30).min(constants::MAX_STRESS);
        // Tours are live shows too: fame stalls at the catalog cap.
        let fame_gain = fame_gain.min(self.live_fame_cap().saturating_sub(self.band.fame));
        self.band.fame += fame_gain;

        let regional_fame_gain = 10 + rng.gen_range(0..=5);
        let new_regional_fame = (regional_fame as u16 + regional_fame_gain as u16).min(100) as u8;
        self.regional_fame
            .insert(regional_fame_key.clone(), new_regional_fame);

        self.week += tour_weeks;
        self.log(format!(
            "🚌 Tour of {} ({}): grossed ${} against ${} in costs, fame +{}, regional fame {}% (+{}).",
            region_name, country_key.replace("_", " "), final_earnings, tour_cost, fame_gain, new_regional_fame, regional_fame_gain
        ));

        if rng.gen_bool(0.3) {
            let bonus = 2u8.min(self.live_fame_cap().saturating_sub(self.band.fame));
            if bonus > 0 {
                self.band.fame += bonus;
                self.log("🗣️ Word of your live show spreads — extra fame on the way home.");
            }
        } else if rng.gen_bool(0.15) {
            self.player.health = self.player.health.saturating_sub(10);
            self.log("🤒 The road took its toll — you came home run down.");
        }
        Ok(())
    }
}
