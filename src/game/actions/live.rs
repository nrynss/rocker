//! Player weekly actions (split by concern). Methods remain on `Game`.
//!
//! Gigs and tours resolve through the per-show engine (`shows.rs`, design
//! §B): every show gets its own reception roll and verdict, and a tour
//! carries momentum — word of mouth — from one stop to the next.

use crate::game::music::ReleaseType;
use crate::game::shows::{self, ShowVerdict};
use rand::Rng;
use serde::{Deserialize, Serialize};

use super::super::constants::{self, *};
use super::super::*;

/// The four tour rigs (design §A). Fame never re-prices a tour — it only
/// gates which rigs are selectable (`fame_gate`) and how many seats a tour
/// fills. Same region + rig + length quotes identically at any fame.
///
/// Not persisted (tour actions aren't saved mid-resolution), but `GameAction`
/// derives `Serialize`/`Deserialize` as a whole, so this must too.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum TourRig {
    Van,
    Bus,
    Truck,
    Full,
}

impl TourRig {
    /// In picker order, smallest rig to biggest.
    pub const ALL: [TourRig; 4] = [TourRig::Van, TourRig::Bus, TourRig::Truck, TourRig::Full];

    /// Index into the `TOUR_RIG_*` const tables — also this rig's position
    /// in `ALL`, for picker navigation.
    pub fn ordinal(self) -> usize {
        match self {
            TourRig::Van => 0,
            TourRig::Bus => 1,
            TourRig::Truck => 2,
            TourRig::Full => 3,
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            TourRig::Van => "Van tour",
            TourRig::Bus => "Tour bus",
            TourRig::Truck => "Truck & crew",
            TourRig::Full => "Full production",
        }
    }

    /// Fame required to select this rig at all (design §A table).
    pub fn fame_gate(self) -> u8 {
        TOUR_RIG_FAME_GATE[self.ordinal()]
    }

    pub fn cost_per_week(self) -> u32 {
        TOUR_RIG_COST_PER_WEEK[self.ordinal()]
    }

    pub fn capacity_mult(self) -> f32 {
        TOUR_RIG_CAPACITY_MULT[self.ordinal()]
    }

    /// (health lost, stress gained) per tour week.
    pub fn wear_per_week(self) -> (u8, u8) {
        (
            TOUR_RIG_HEALTH_COST_PER_WEEK[self.ordinal()],
            TOUR_RIG_STRESS_COST_PER_WEEK[self.ordinal()],
        )
    }

    /// Key into `markets.json`'s `touring_costs` map — re-keyed from the old
    /// fame-tier keys (local/regional/national/international) to the rigs
    /// (design §A).
    fn markets_key(self) -> &'static str {
        match self {
            TourRig::Van => "van_tour",
            TourRig::Bus => "tour_bus",
            TourRig::Truck => "truck_and_crew",
            TourRig::Full => "full_production",
        }
    }
}

/// The up-front quote a tour picker must show before booking (design §A):
/// itemized cost, weeks, shows, and a projected gross range computed from
/// the same formula the tour uses, at momentum 1.0, ± the reception spread
/// (`RECEPTION_ATTENDANCE_MIN_FACTOR`/`MAX_FACTOR`). `action_go_on_tour`
/// derives its charge from the same underlying numbers, so the quote can
/// never drift from what booking actually costs.
#[derive(Debug, Clone)]
pub struct TourQuote {
    pub rig: TourRig,
    pub weeks: u8,
    pub region_name: String,
    pub shows: u32,
    pub cost: i32,
    pub gross_low: u32,
    pub gross_high: u32,
    pub fame_gain: u8,
    pub regional_fame_gain_min: u8,
    pub regional_fame_gain_max: u8,
}

/// The deterministic (no-rng) core of a tour booking: everything the quote
/// and the actual booking must agree on. Shared by `quote_tour` (read-only,
/// for the picker) and `action_go_on_tour` (which then layers per-show rng
/// on top) — the single source of truth that keeps the quote honest.
struct TourPot {
    country_key: String,
    region_name: String,
    fame_req: u8,
    cost: i32,
    shows_total: u32,
    synth_capacity: u32,
    total_potential_gross: f32,
    fame_gain: u8,
    regional_fame_gain_base: u16,
    regional_fame_key: String,
    regional_fame_current: u8,
}

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
    ///
    /// Also the fix for L12's open finding: stage time is how a live
    /// reputation is actually built. Every show but a rough one nudges
    /// `reputation.live_performance` up — solid gigs teach you something,
    /// great and transcendent ones teach you more — so a touring band's
    /// live reception keeps climbing instead of being fixed forever at
    /// character creation.
    fn apply_show_verdict_rewards(&mut self, verdict: ShowVerdict) {
        let live_reputation_gain = match verdict {
            ShowVerdict::Rough => 0,
            ShowVerdict::Solid => SOLID_SHOW_LIVE_REPUTATION_GAIN,
            ShowVerdict::Great => GREAT_SHOW_LIVE_REPUTATION_GAIN,
            ShowVerdict::Transcendent => TRANSCENDENT_SHOW_LIVE_REPUTATION_GAIN,
        };
        if live_reputation_gain > 0 {
            self.band.reputation.live_performance =
                (self.band.reputation.live_performance + live_reputation_gain).min(100);
        }

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

    /// Fame gate for a rig: fame never re-prices a tour, it only decides
    /// which rigs are on offer (design §A).
    pub fn rig_is_available(&self, rig: TourRig) -> bool {
        self.band.fame >= rig.fame_gate()
    }

    /// Fame gate for a tour length: 1-2 weeks are always open, 3 needs fame
    /// 40, 4 needs fame 60 (design §A). Out-of-range weeks are never
    /// available.
    pub fn tour_length_is_available(&self, weeks: u8) -> bool {
        if !(TOUR_LENGTH_MIN_WEEKS..=TOUR_LENGTH_MAX_WEEKS).contains(&weeks) {
            return false;
        }
        self.band.fame >= TOUR_LENGTH_FAME_GATE[(weeks - 1) as usize]
    }

    /// The deterministic core shared by `quote_tour` and `action_go_on_tour`
    /// (design §A): cost, shows, capacity, and the fame/regional-fame gains
    /// all come from here, so a quote can never drift from what booking
    /// actually charges. Validates region/rig/length gates; no rng, no
    /// mutation.
    fn tour_pot(&self, region_index: usize, rig: TourRig, weeks: u8) -> Result<TourPot, String> {
        if !(TOUR_LENGTH_MIN_WEEKS..=TOUR_LENGTH_MAX_WEEKS).contains(&weeks) {
            return Err(format!(
                "Tour length must be {}-{} weeks.",
                TOUR_LENGTH_MIN_WEEKS, TOUR_LENGTH_MAX_WEEKS
            ));
        }
        if !self.tour_length_is_available(weeks) {
            return Err(format!(
                "A {}-week tour needs at least {} fame.",
                weeks,
                TOUR_LENGTH_FAME_GATE[(weeks - 1) as usize]
            ));
        }
        if !self.rig_is_available(rig) {
            return Err(format!(
                "'{}' needs at least {} fame.",
                rig.label(),
                rig.fame_gate()
            ));
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

        let country_travel_mult = match country_key.as_str() {
            "united_states" => 1.5,
            "united_kingdom" => 0.8,
            "europe" => 1.2,
            "japan" => 1.0,
            "australia" => 1.4,
            _ => 1.0,
        };

        let touring_costs = self
            .data_files
            .markets_data
            .market_modifiers
            .touring_costs
            .get(rig.markets_key())
            .ok_or_else(|| "Touring cost data not found for this rig.".to_string())?;

        // Cost formula (design §A): rig cost/week × weeks × country travel
        // mult × the rig's own travel/equipment modifiers — the dead
        // `markets.json` fields finally do their job, now keyed by rig
        // instead of the old fame tier. Linear in weeks, by design.
        let cost = (rig.cost_per_week() as f32
            * weeks as f32
            * country_travel_mult
            * touring_costs.travel_cost_modifier
            * touring_costs.equipment_cost_modifier)
            .round() as i32;

        let regional_fame_key = format!("{}:{}", country_key, region_key);
        let regional_fame_current = *self.regional_fame.get(&regional_fame_key).unwrap_or(&0);

        let audience = (self.band.fame as f32 / 3.0) + (regional_fame_current as f32);
        let base_gross = (*population as f32).sqrt()
            * (*economic_strength as f32 / 100.0)
            * audience
            * TOUR_GROSS_COEFFICIENT;

        let era_modifier = self.timeline.get_gig_pay_modifier();
        let market_modifier = self.world.get_market_modifier();
        // The whole-tour pot: the pre-v0.6/v0.7 regional formula, scaled by
        // the rig's capacity multiplier (design §A — "a bigger rig books
        // bigger rooms, raising the gross ceiling"; the pot is "untouched
        // apart from the capacity multiplier"). The multiplier belongs on
        // the pot, not just the reported attendance: the per-show take is
        // this pot / shows, so without it a $8,000/wk full-production rig
        // would gross exactly what a $150/wk van does. A longer tour still
        // spreads the same pot across more shows — length buys fame, rig
        // buys gross.
        let total_potential_gross =
            (base_gross * era_modifier * market_modifier * rig.capacity_mult()).max(0.0);

        // Same multiplier on the synthesized venue so reported attendance
        // stays consistent with the bigger rooms the rig is playing (§A).
        let synth_capacity = (Self::synth_tour_venue_capacity(*population, *economic_strength)
            as f32
            * rig.capacity_mult())
        .round() as u32;

        let shows_total = weeks as u32 * SHOWS_PER_TOUR_WEEK;

        // Fame and regional-fame gains scale sublinearly with weeks (§A) —
        // see `TOUR_FAME_WEEKS_EXPONENT` for the curve and rationale.
        let weeks_scale = (weeks as f32).powf(TOUR_FAME_WEEKS_EXPONENT);
        let fame_gain = (TOUR_FAME_GAIN_BASE * weeks_scale).round() as u8;
        let regional_fame_gain_base = (TOUR_REGIONAL_FAME_GAIN_BASE * weeks_scale).round() as u16;

        Ok(TourPot {
            country_key: country_key.clone(),
            region_name: region_name.clone(),
            fame_req: *fame_req,
            cost,
            shows_total,
            synth_capacity,
            total_potential_gross,
            fame_gain,
            regional_fame_gain_base,
            regional_fame_key,
            regional_fame_current,
        })
    }

    /// The up-front quote (design §A): the tour picker must show this
    /// before booking. Pure — no rng, no mutation — so the UI can recompute
    /// it live as the player changes rig/length.
    pub fn quote_tour(
        &self,
        region_index: usize,
        rig: TourRig,
        weeks: u8,
    ) -> Result<TourQuote, String> {
        let pot = self.tour_pot(region_index, rig, weeks)?;

        // Projected gross range: the same whole-tour pot, at momentum 1.0,
        // spread by the reception-driven attendance factor alone (§A).
        let gross_low = (pot.total_potential_gross * RECEPTION_ATTENDANCE_MIN_FACTOR)
            .max(0.0)
            .round() as u32;
        let gross_high = (pot.total_potential_gross * RECEPTION_ATTENDANCE_MAX_FACTOR)
            .max(0.0)
            .round() as u32;

        Ok(TourQuote {
            rig,
            weeks,
            region_name: pot.region_name,
            shows: pot.shows_total,
            cost: pot.cost,
            gross_low,
            gross_high,
            fame_gain: pot.fame_gain,
            regional_fame_gain_min: pot.regional_fame_gain_base.min(u8::MAX as u16) as u8,
            regional_fame_gain_max: (pot.regional_fame_gain_base
                + TOUR_REGIONAL_FAME_GAIN_RNG_SPREAD as u16)
                .min(u8::MAX as u16) as u8,
        })
    }

    pub(in crate::game) fn action_go_on_tour(
        &mut self,
        region_index: usize,
        rig: TourRig,
        weeks: u8,
        rng: &mut impl Rng,
    ) -> Result<(), String> {
        if self.player.stress >= TOUR_STRESS_GUARD {
            return Err("You're too stressed out to go on tour!".to_string());
        }
        if self.player.health < TOUR_HEALTH_GUARD {
            return Err("You're too unwell to go on tour!".to_string());
        }

        let pot = self.tour_pot(region_index, rig, weeks)?;

        if !self.player.can_afford(pot.cost) {
            return Err(format!(
                "You need at least ${} to finance this tour!",
                pot.cost
            ));
        }

        let region_name = pot.region_name.clone();
        let era_genre_modifier = self
            .data_files
            .era_genre_modifier(self.timeline.get_current_year(), self.band.genre.aliases());

        let base_fill_ratio =
            ((self.band.fame as f32 + 10.0) / (pot.fame_req as f32 + 10.0)).min(1.0);

        let per_show_share = pot.total_potential_gross / pot.shows_total as f32;

        let mut momentum = MOMENTUM_START;
        let mut rows: Vec<ShowReport> = Vec::with_capacity(pot.shows_total as usize);
        let mut gross_sum: u32 = 0;

        for show_idx in 0..pot.shows_total {
            let venue_name = self.synth_tour_venue_name(&region_name, rng);

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
            let attendance = (pot.synth_capacity as f32 * fill_ratio).round() as u32;
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
                capacity: pot.synth_capacity,
                take,
            });
        }

        let report = TourReport::from_rows(rows, pot.fame_gain);
        if report.went_very_well() {
            self.player.happiness = (self.player.happiness + TOUR_WENT_WELL_HAPPINESS_GAIN)
                .min(constants::MAX_HAPPINESS);
            self.player.creativity = (self.player.creativity + TOUR_WENT_WELL_CREATIVITY_GAIN)
                .min(constants::MAX_CREATIVITY);
        }

        // Touring wears harder than a night at home (§A): the rig's own
        // wear table, replacing the flat cost-per-week the headline tour
        // used to charge every rig alike.
        let (rig_health_cost, rig_stress_cost) = rig.wear_per_week();
        let tour_stress_cost = rig_stress_cost.saturating_mul(weeks);
        let tour_health_cost = rig_health_cost.saturating_mul(weeks);
        self.player.stress = (self.player.stress + tour_stress_cost).min(constants::MAX_STRESS);
        self.player.health = self.player.health.saturating_sub(tour_health_cost);

        self.player.spend_money(pot.cost);
        self.player.earn_money(gross_sum);

        let live_cap = self.live_fame_cap();
        let fame_gain = pot.fame_gain.min(live_cap.saturating_sub(self.band.fame));
        self.band.gain_fame_capped(fame_gain, live_cap);

        let regional_fame_gain = pot.regional_fame_gain_base
            + rng.gen_range(0..=TOUR_REGIONAL_FAME_GAIN_RNG_SPREAD as u16);
        let new_regional_fame =
            (pot.regional_fame_current as u16 + regional_fame_gain).min(100) as u8;
        self.regional_fame
            .insert(pot.regional_fame_key.clone(), new_regional_fame);

        self.week += weeks as u32;

        let avg_verdict = ShowVerdict::from_reception(report.avg_reception);
        self.log(format!(
            "🚌 {} tour of {} ({}), {} weeks: {} shows, avg reception {} ({}) — grossed ${} against ${} in costs, fame +{}, regional fame {}% (+{}). Press R for the tour report.",
            rig.label(),
            region_name,
            pot.country_key.replace("_", " "),
            weeks,
            pot.shows_total,
            report.avg_reception,
            avg_verdict.label(),
            gross_sum,
            pot.cost,
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
