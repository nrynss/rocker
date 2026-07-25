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
/// Persisted since v0.9 §F: a tour is now `OngoingActivity::Tour` state that
/// survives a save taken mid-run, and `GameAction` carries it too.
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
    /// Regional fame already banked in this territory. It is the dominant
    /// term in the gross pot (`audience = fame/3 + regional_fame`), so it is
    /// the single number that explains why the same rig loses money on a
    /// first visit and prints money on a return — the picker shows it.
    pub regional_fame_current: u8,
}

impl TourQuote {
    /// Projected net at the pessimistic and optimistic ends of the gross
    /// range. §A allows booking a money-losing tour — it buys fame — but
    /// insists it be "a *decision*, never a surprise", and a cost beside a
    /// gross range is arithmetic the player shouldn't have to do at the
    /// picker.
    pub fn projected_net(&self) -> (i32, i32) {
        (
            self.gross_low as i32 - self.cost,
            self.gross_high as i32 - self.cost,
        )
    }
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
        // Report the fame actually applied (comeback doubling, caps), not
        // the raw pre-multiplier gain.
        let fame_applied = self
            .band
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
            fame_applied,
        ));

        if fame_applied > 0 {
            self.log(format!(
                "🎤 Played '{}' — a {} night, sold {}/{} tickets, earned ${}, fame +{}.",
                venue_name,
                verdict.label(),
                attendance,
                capacity,
                earnings,
                fame_applied
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

        // §G tap point (S-EVENTS): the night happened, so this week's
        // incident tap is the stage's. Draws no rng, so the show above rolls
        // exactly what it rolled before. One line; the rest lives in
        // `events_apply.rs`.
        self.note_gig_week();
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
        // would gross exactly what a $150/wk van does.
        //
        // T-LEN: ...and the pot scales with how many nights you actually
        // play, sublinearly (`TOUR_GROSS_WEEKS_EXPONENT`, normalized so a
        // one-week run keeps the coefficient's calibrated value). The pot
        // used to be flat in `weeks` while `shows_total` and the cost both
        // grew with it, so a longer tour split the same money across more
        // nights *and* paid more for the privilege — length was a strict
        // loss at every rig, not the trade-off §A describes. Now a fourth
        // week still earns less than the first (the territory saturates)
        // but it earns something.
        let weeks_gross_scale = (weeks as f32).powf(TOUR_GROSS_WEEKS_EXPONENT);
        let total_potential_gross =
            (base_gross * era_modifier * market_modifier * rig.capacity_mult() * weeks_gross_scale)
                .max(0.0);

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
            regional_fame_current: pot.regional_fame_current,
        })
    }

    /// Book a tour (design §A + §F). Everything the tour's economics depend
    /// on is resolved here, once, by `tour_pot`, and frozen into the
    /// `OngoingActivity` — the cost is charged up front, and the first of
    /// the tour's weeks resolves immediately. The remaining weeks are
    /// resolved one per turn by `turn.rs`; this method never touches
    /// `Game::week`.
    pub(in crate::game) fn action_go_on_tour(
        &mut self,
        region_index: usize,
        rig: TourRig,
        weeks: u8,
    ) -> Result<(), String> {
        if let Some(activity) = &self.ongoing_activity {
            return Err(format!(
                "You're already committed — {} ({}).",
                activity.headline(),
                activity.progress()
            ));
        }
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

        // §B: the bill is due at booking, in full — you cannot start a run
        // you can't finish. (Walking out later forfeits it; see §F.)
        self.player.spend_money(pot.cost);

        let era_genre_modifier = self
            .data_files
            .era_genre_modifier(self.timeline.get_current_year(), self.band.genre.aliases());

        let tour = TourInProgress {
            region_index,
            rig,
            region_name: pot.region_name.clone(),
            country_key: pot.country_key.clone(),
            weeks_total: weeks,
            weeks_done: 0,
            booking_week: self.week,
            cost: pot.cost,
            shows_total: pot.shows_total,
            synth_capacity: pot.synth_capacity,
            per_show_share: pot.total_potential_gross / pot.shows_total as f32,
            base_fill_ratio: ((self.band.fame as f32 + 10.0) / (pot.fame_req as f32 + 10.0))
                .min(1.0),
            fame_gain: pot.fame_gain,
            regional_fame_key: pot.regional_fame_key.clone(),
            regional_fame_current: pot.regional_fame_current,
            regional_fame_gain_base: pot.regional_fame_gain_base,
            era_genre_modifier,
            condition_stress: self.player.stress,
            condition_health: self.player.health,
            road_creativity: self.player.creativity,
            momentum: MOMENTUM_START,
            rows: Vec::with_capacity(pot.shows_total as usize),
        };

        self.log(format!(
            "🚌 {} of {} ({}) booked — {} weeks, {} shows, ${} up front.",
            rig.label(),
            pot.region_name,
            pot.country_key.replace("_", " "),
            weeks,
            pot.shows_total,
            pot.cost
        ));

        // The first week plays now; `turn.rs` drives the rest.
        self.resolve_tour_week(tour);
        Ok(())
    }

    /// Play one week of a tour in progress (design §F): `SHOWS_PER_TOUR_WEEK`
    /// nights, the week's takings, one week of the rig's wear. Momentum and
    /// the report rows live on the activity, so word of mouth carries across
    /// the week boundary exactly as it used to carry across the loop.
    ///
    /// The nights roll on the **activity stream** (`rng.rs`), one sub-stream
    /// per night index, not on the turn's action stream. That is what keeps
    /// the split neutral: night 17 rolls the same numbers whether the tour
    /// resolved five a week for four weeks or twenty in one sitting, and the
    /// weekly world tick that now runs in between can't shift it.
    pub(in crate::game) fn resolve_tour_week(&mut self, mut tour: TourInProgress) {
        let first_show = u32::from(tour.weeks_done) * SHOWS_PER_TOUR_WEEK;
        let last_show = (first_show + SHOWS_PER_TOUR_WEEK).min(tour.shows_total);
        let mut week_gross: u32 = 0;

        for show_idx in first_show..last_show {
            let mut rng = self.activity_rng(tour.booking_week, show_idx);
            let venue_name = self.synth_tour_venue_name(&tour.region_name, &mut rng);

            let reception = shows::compute_reception(
                &self.band,
                tour.condition_stress,
                tour.condition_health,
                tour.era_genre_modifier,
                tour.road_creativity,
                &mut rng,
            );
            let verdict = ShowVerdict::from_reception(reception);
            let attendance_factor = shows::reception_attendance_factor(reception);
            // Word of mouth: momentum carries from show to show, so the
            // same night's own reception affects only its own attendance
            // fill, not its own take. Money is centered on the per-show
            // share regardless of venue size (design §B).
            let money_multiplier = attendance_factor * tour.momentum;

            let fill_ratio = (tour.base_fill_ratio * money_multiplier).clamp(0.0, 1.0);
            let attendance = (tour.synth_capacity as f32 * fill_ratio).round() as u32;
            let take = (tour.per_show_share * money_multiplier).max(0.0).round() as u32;

            self.apply_show_verdict_rewards(verdict);
            // The road's own copy of creativity moves by the same rule, so
            // the reception rolls read what the pre-split loop's live read
            // gave them — the weekly tick's creativity drain lands on the
            // player, not on the band's nights (§F).
            tour.road_creativity = tour
                .road_creativity
                .saturating_add(show_creativity_gain(verdict))
                .min(constants::MAX_CREATIVITY);
            tour.momentum = shows::apply_momentum_delta(tour.momentum, verdict);

            week_gross = week_gross.saturating_add(take);

            tour.rows.push(ShowReport {
                week: self.week,
                venue_name,
                verdict: verdict.label().to_string(),
                reception,
                attendance,
                capacity: tour.synth_capacity,
                take,
            });
        }

        self.player.earn_money(week_gross);

        // Touring wears harder than a night at home (§A): the rig's own
        // wear table, now charged the week it is actually endured rather
        // than the whole run's worth at booking.
        let (rig_health_cost, rig_stress_cost) = tour.rig.wear_per_week();
        self.player.stress = (self.player.stress + rig_stress_cost).min(constants::MAX_STRESS);
        self.player.health = self.player.health.saturating_sub(rig_health_cost);

        tour.weeks_done += 1;
        if tour.weeks_done >= tour.weeks_total {
            self.settle_tour(tour, TourEnding::Completed);
            return;
        }

        self.log(format!(
            "🚌 {} of {}, week {} of {} — {} shows, ${} at the door.",
            tour.rig.label(),
            tour.region_name,
            tour.weeks_done,
            tour.weeks_total,
            last_show - first_show,
            week_gross
        ));
        self.ongoing_activity = Some(OngoingActivity::Tour(tour));
    }

    /// Close out a tour and clear it from the calendar (design §F). A
    /// completed run settles its fame and regional fame — the whole-run
    /// rewards `tour_pot` quoted; one walked out of keeps only what already
    /// resolved.
    pub(in crate::game) fn settle_tour(&mut self, tour: TourInProgress, ending: TourEnding) {
        self.ongoing_activity = None;

        let gross: u32 = tour.rows.iter().map(|row| row.take).sum();
        let shows_played = tour.rows.len();

        if ending == TourEnding::Abandoned {
            let report = TourReport::from_rows(tour.rows, 0);
            self.last_tour_report = Some(report);
            self.log(format!(
                "🛑 Walked off the {} of {} after {} of {} weeks — {} shows played and ${} taken; the rest of the run, and the ${} it cost to book, are gone.",
                tour.rig.label(),
                tour.region_name,
                tour.weeks_done,
                tour.weeks_total,
                shows_played,
                gross,
                tour.cost
            ));
            return;
        }

        // Cap the fame gain against the live ceiling and apply it before
        // building the report, so the Tour Report screen and the 🚌 line
        // both show the fame actually applied to the band — comeback
        // doubling and caps included, not the pre-cap request.
        let live_cap = self.live_fame_cap();
        let fame_gain = tour.fame_gain.min(live_cap.saturating_sub(self.band.fame));
        let fame_applied = self.band.gain_fame_capped(fame_gain, live_cap);
        let report = TourReport::from_rows(tour.rows, fame_applied);
        if report.went_very_well() {
            self.player.happiness = (self.player.happiness + TOUR_WENT_WELL_HAPPINESS_GAIN)
                .min(constants::MAX_HAPPINESS);
            self.player.creativity = (self.player.creativity + TOUR_WENT_WELL_CREATIVITY_GAIN)
                .min(constants::MAX_CREATIVITY);
        }

        // The settlement roll gets the step index one past the last night,
        // so it can never collide with a show's stream (`rng.rs`).
        let mut rng = self.activity_rng(tour.booking_week, tour.shows_total);
        let regional_fame_gain = tour.regional_fame_gain_base
            + rng.gen_range(0..=TOUR_REGIONAL_FAME_GAIN_RNG_SPREAD as u16);
        let new_regional_fame =
            (tour.regional_fame_current as u16 + regional_fame_gain).min(100) as u8;
        // The 100-cap can eat part of the rolled gain — report the delta
        // that actually landed, not the roll.
        let regional_fame_applied = new_regional_fame - tour.regional_fame_current;
        self.regional_fame
            .insert(tour.regional_fame_key.clone(), new_regional_fame);

        let avg_verdict = ShowVerdict::from_reception(report.avg_reception);
        self.log(format!(
            "🚌 {} tour of {} ({}), {} weeks: {} shows, avg reception {} ({}) — grossed ${} against ${} in costs, fame +{}, regional fame {}% (+{}). Press R for the tour report.",
            tour.rig.label(),
            tour.region_name,
            tour.country_key.replace("_", " "),
            tour.weeks_total,
            tour.shows_total,
            report.avg_reception,
            avg_verdict.label(),
            gross,
            tour.cost,
            fame_applied,
            new_regional_fame,
            regional_fame_applied
        ));
        if report.went_very_well() {
            self.log("🌟 The tour went very well — spirits (and inspiration) are high.");
        }

        self.last_tour_report = Some(report);
    }
}

/// How a tour left the calendar (design §F).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(in crate::game) enum TourEnding {
    /// Every booked week played — the run settles its fame and regional fame.
    Completed,
    /// Walked out of, or forced off the road by an emergency: the nights
    /// already played stand, the rest of the pot does not.
    Abandoned,
}

/// The creativity a night on stage feeds, by verdict — the same table
/// `apply_show_verdict_rewards` applies to the player, factored out so a
/// tour's own road copy can move in lockstep (§F).
fn show_creativity_gain(verdict: ShowVerdict) -> u8 {
    match verdict {
        ShowVerdict::Great => GREAT_SHOW_CREATIVITY_GAIN,
        ShowVerdict::Transcendent => TRANSCENDENT_SHOW_CREATIVITY_GAIN,
        ShowVerdict::Solid | ShowVerdict::Rough => 0,
    }
}
