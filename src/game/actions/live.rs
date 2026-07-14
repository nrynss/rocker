//! Player weekly actions (split by concern). Methods remain on `Game`.
//!
//! Gigs and tours resolve through the per-show engine (`shows.rs`, design
//! §B): every show gets its own reception roll and verdict, and a tour
//! carries momentum — word of mouth — from one stop to the next.

use crate::game::music::ReleaseType;
use crate::game::shows::{self, ShowVerdict};
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

    /// Apply a show's stat rewards (§A/§B): great and transcendent shows
    /// feed creativity, transcendent also lifts happiness on the spot.
    fn apply_show_verdict_rewards(&mut self, verdict: ShowVerdict) {
        match verdict {
            ShowVerdict::Great => {
                self.player.creativity = (self.player.creativity + GREAT_SHOW_CREATIVITY_GAIN)
                    .min(constants::MAX_CREATIVITY);
            }
            ShowVerdict::Transcendent => {
                self.player.creativity = (self.player.creativity
                    + TRANSCENDENT_SHOW_CREATIVITY_GAIN)
                    .min(constants::MAX_CREATIVITY);
                self.player.happiness = (self.player.happiness + TRANSCENDENT_SHOW_HAPPINESS_GAIN)
                    .min(constants::MAX_HAPPINESS);
            }
            ShowVerdict::Solid | ShowVerdict::Rough => {}
        }
    }

    /// A synthesized tour-stop venue's capacity, drawn from the region's
    /// population and economic strength (design §B — Box office). [tune]
    fn synth_tour_venue_capacity(population: u32, economic_strength: u8) -> u32 {
        let raw = (population as f32 / TOUR_VENUE_CAPACITY_POP_DIVISOR)
            * (economic_strength as f32 / 100.0);
        raw.clamp(
            TOUR_VENUE_CAPACITY_MIN as f32,
            TOUR_VENUE_CAPACITY_MAX as f32,
        ) as u32
    }

    /// A synthesized tour-stop venue's name: flavor from the region/city
    /// name lists, since a tour doesn't book from the home scene's fixed
    /// five venues (design §B — Box office).
    fn synth_tour_venue_name(&self, region_name: &str, rng: &mut impl Rng) -> String {
        let venue_names = &self.data_files.venue_names;
        let city_names = &self.data_files.city_names;
        let venue = &venue_names[rng.gen_range(0..venue_names.len())];
        let city = &city_names[rng.gen_range(0..city_names.len())];
        format!("{venue} ({city}, {region_name})")
    }

    pub(in crate::game) fn action_play_gig(
        &mut self,
        venue_index: usize,
        rng: &mut impl Rng,
    ) -> Result<(), String> {
        if self.player.stress >= GIG_STRESS_GUARD {
            return Err("You're too stressed out to perform right now!".to_string());
        }
        if self.player.health < GIG_HEALTH_GUARD {
            return Err("You're too unwell to perform!".to_string());
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

        let era_modifier = self.timeline.get_gig_pay_modifier();
        let market_modifier = self.world.get_market_modifier();
        let era_genre_modifier = self
            .data_files
            .era_genre_modifier(self.timeline.get_current_year(), self.band.genre.aliases());

        let base_ratio = ((self.band.fame as f32 + 10.0) / (venue.prestige as f32 + 10.0)).min(1.0);

        let reception = shows::compute_reception(
            &self.band,
            self.player.stress,
            self.player.health,
            era_genre_modifier,
            self.player.creativity,
            rng,
        );
        let verdict = ShowVerdict::from_reception(reception);
        let attendance_factor = shows::reception_attendance_factor(reception);
        let attendance_ratio = (base_ratio * attendance_factor).clamp(0.0, 1.0);

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

        let venue_name = venue.name.clone();
        let capacity = venue.capacity;

        self.player.earn_money(earnings);
        self.band
            .gain_fame_capped(fame_gain, venue_ceiling.min(live_cap));

        self.player.stress = (self.player.stress + GIG_STRESS_COST).min(constants::MAX_STRESS);
        self.apply_show_verdict_rewards(verdict);

        self.last_tour_report = Some(TourReport::from_rows(
            vec![ShowReport {
                week: self.week,
                venue_name: venue_name.clone(),
                verdict: verdict.label().to_string(),
                reception,
                attendance,
                capacity,
                take: earnings,
            }],
            fame_gain,
        ));

        if fame_gain > 0 {
            self.log(format!(
                "🎤 Played '{}' — a {} night, sold {}/{} tickets, earned ${}, fame +{}.",
                venue_name,
                verdict.label(),
                attendance,
                capacity,
                earnings,
                fame_gain
            ));
        } else if self.band.fame >= live_cap {
            self.log(format!(
                "🎤 Played '{}' — a {} night, sold {}/{} tickets, earned ${}. The buzz has peaked — without new records, word of mouth carries no further.",
                venue_name, verdict.label(), attendance, capacity, earnings
            ));
        } else {
            self.log(format!(
                "🎤 Played '{}' — a {} night, sold {}/{} tickets, earned ${}. The regulars know every word — you've outgrown this stage.",
                venue_name, verdict.label(), attendance, capacity, earnings
            ));
        }
        Ok(())
    }

    pub(in crate::game) fn action_go_on_tour(
        &mut self,
        region_index: usize,
        rng: &mut impl Rng,
    ) -> Result<(), String> {
        if self.player.stress >= TOUR_STRESS_GUARD {
            return Err("You're too stressed out to go on tour!".to_string());
        }
        if self.player.health < TOUR_HEALTH_GUARD {
            return Err("You're too unwell to go on tour!".to_string());
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
        // The whole-tour pot: exactly the pre-v0.6 formula. Per-show
        // resolution below redistributes this across shows — it does not
        // grow the total (design §B — Box office).
        let total_potential_gross = (base_gross * era_modifier * market_modifier).max(0.0);

        let era_genre_modifier = self
            .data_files
            .era_genre_modifier(self.timeline.get_current_year(), self.band.genre.aliases());

        let synth_capacity = Self::synth_tour_venue_capacity(*population, *economic_strength);
        let base_fill_ratio = ((self.band.fame as f32 + 10.0) / (*fame_req as f32 + 10.0)).min(1.0);

        let shows_total = tour_weeks * SHOWS_PER_TOUR_WEEK;
        let per_show_share = total_potential_gross / shows_total as f32;

        let mut momentum = MOMENTUM_START;
        let mut rows: Vec<ShowReport> = Vec::with_capacity(shows_total as usize);
        let mut gross_sum: u32 = 0;

        for show_idx in 0..shows_total {
            let venue_name = self.synth_tour_venue_name(region_name, rng);

            let reception = shows::compute_reception(
                &self.band,
                self.player.stress,
                self.player.health,
                era_genre_modifier,
                self.player.creativity,
                rng,
            );
            let verdict = ShowVerdict::from_reception(reception);
            let attendance_factor = shows::reception_attendance_factor(reception);
            // Word of mouth: momentum carries from show to show, so the
            // same night's own reception affects only its own attendance
            // fill, not its own take. Money is centered on the per-show
            // share regardless of venue size (design §B).
            let money_multiplier = attendance_factor * momentum;

            let fill_ratio = (base_fill_ratio * money_multiplier).clamp(0.0, 1.0);
            let attendance = (synth_capacity as f32 * fill_ratio).round() as u32;
            let take = (per_show_share * money_multiplier).max(0.0).round() as u32;

            self.apply_show_verdict_rewards(verdict);
            momentum = shows::apply_momentum_delta(momentum, verdict);

            gross_sum = gross_sum.saturating_add(take);

            rows.push(ShowReport {
                week: self.week + show_idx / SHOWS_PER_TOUR_WEEK,
                venue_name,
                verdict: verdict.label().to_string(),
                reception,
                attendance,
                capacity: synth_capacity,
                take,
            });
        }

        let report = TourReport::from_rows(rows, fame_gain);
        if report.went_very_well() {
            self.player.happiness = (self.player.happiness + TOUR_WENT_WELL_HAPPINESS_GAIN)
                .min(constants::MAX_HAPPINESS);
            self.player.creativity = (self.player.creativity + TOUR_WENT_WELL_CREATIVITY_GAIN)
                .min(constants::MAX_CREATIVITY);
        }

        // Touring wears harder than a night at home (§A): a flat cost per
        // tour week, replacing the old 15%-chance-of-a-big-health-hit.
        let tour_stress_cost = TOUR_STRESS_COST_PER_WEEK.saturating_mul(tour_weeks as u8);
        let tour_health_cost = TOUR_HEALTH_COST_PER_WEEK.saturating_mul(tour_weeks as u8);
        self.player.stress = (self.player.stress + tour_stress_cost).min(constants::MAX_STRESS);
        self.player.health = self.player.health.saturating_sub(tour_health_cost);

        self.player.spend_money(tour_cost);
        self.player.earn_money(gross_sum);

        let live_cap = self.live_fame_cap();
        let fame_gain = fame_gain.min(live_cap.saturating_sub(self.band.fame));
        self.band.gain_fame_capped(fame_gain, live_cap);

        let regional_fame_gain = 10 + rng.gen_range(0..=5);
        let new_regional_fame = (regional_fame as u16 + regional_fame_gain as u16).min(100) as u8;
        self.regional_fame
            .insert(regional_fame_key.clone(), new_regional_fame);

        self.week += tour_weeks;

        let avg_verdict = ShowVerdict::from_reception(report.avg_reception);
        self.log(format!(
            "🚌 Tour of {} ({}): {} shows, avg reception {} ({}) — grossed ${} against ${} in costs, fame +{}, regional fame {}% (+{}). Press R for the tour report.",
            region_name,
            country_key.replace("_", " "),
            shows_total,
            report.avg_reception,
            avg_verdict.label(),
            gross_sum,
            tour_cost,
            fame_gain,
            new_regional_fame,
            regional_fame_gain
        ));
        if report.went_very_well() {
            self.log("🌟 The tour went very well — spirits (and inspiration) are high.");
        }

        self.last_tour_report = Some(report);

        Ok(())
    }
}
