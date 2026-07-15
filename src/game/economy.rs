//! The money pipeline: recording and pressing costs, sales scoring,
//! and the weekly release/catalog payout.

use crate::game::music::{Release, ReleaseType};

use super::constants::{self, *};
use super::*;

impl Game {
    pub(super) fn calculate_release_sales_score(&self, release: &Release) -> u32 {
        let quality_score = release.release_quality as f32 * SALES_QUALITY_WEIGHT;
        let marketing_score = release.marketing_level_achieved as f32 * SALES_MARKETING_WEIGHT;
        let fame_score = self.band.fame as f32 * SALES_FAME_WEIGHT;

        let era_sales_modifier = self
            .timeline
            .get_current_era()
            .market_conditions
            .record_sales_growth
            / 100.0
            + 1.0;

        let genre_modifier = release
            .genre
            .as_ref()
            .and_then(|g| self.world.dynamic_genre_modifiers.get(g).copied())
            .unwrap_or(1.0);

        // The era's tastes: the same modifier scene-band releases live by.
        let era_genre_modifier = release
            .genre
            .as_ref()
            .map(|g| {
                self.data_files
                    .era_genre_modifier(self.timeline.get_current_year(), g.aliases())
            })
            .unwrap_or(1.0);

        let base_score = quality_score + marketing_score + fame_score;
        (base_score * era_sales_modifier * genre_modifier * era_genre_modifier).max(0.0) as u32
    }

    /// How much of a release's potential audience the band can actually reach.
    /// A label brings its distribution network; an independent act is capped
    /// by its own fame — a nobody pressing records sells them locally at best.
    fn distribution_multiplier(&self) -> f32 {
        match self.band.current_deal() {
            Some(deal) => 0.5 + f32::from(deal.market_reach) / 100.0,
            None => {
                INDIE_REACH_FLOOR + (f32::from(self.band.fame) / 100.0) * (1.0 - INDIE_REACH_FLOOR)
            }
        }
    }

    /// Studio cost of a release. Pressing is a separate bill.
    pub fn recording_cost(&self, release_type: &ReleaseType) -> i32 {
        let base = match release_type {
            ReleaseType::Single => constants::SINGLE_RECORDING_COST,
            ReleaseType::Album => constants::ALBUM_RECORDING_BASE_COST,
        };
        (base as f32 * self.timeline.get_recording_cost_modifier()) as i32
    }

    /// What a pressing run of `copies` costs to buy yourself.
    pub fn pressing_cost(&self, release_type: &ReleaseType, copies: u32) -> i32 {
        let (setup, per_copy) = match release_type {
            ReleaseType::Single => (PRESSING_SETUP_SINGLE, PRESSING_PER_COPY_SINGLE),
            ReleaseType::Album => (PRESSING_SETUP_ALBUM, PRESSING_PER_COPY_ALBUM),
        };
        ((setup + per_copy * copies as f32) * self.timeline.get_recording_cost_modifier()) as i32
    }

    /// How many copies the label presses: its network plus your name.
    fn label_pressing_size(&self, deal: &band::RecordDeal) -> u32 {
        u32::from(deal.market_reach) * LABEL_PRESSING_PER_REACH
            + u32::from(self.band.fame) * LABEL_PRESSING_PER_FAME
    }

    /// Who presses this release and what it costs the band: the label's
    /// network for free when signed, otherwise the chosen run out of pocket.
    pub(super) fn plan_pressing(
        &self,
        release_type: &ReleaseType,
        pressing: Option<usize>,
    ) -> Result<(u32, i32), String> {
        if let Some(deal) = self.band.current_deal() {
            return Ok((self.label_pressing_size(deal), 0));
        }
        let tier = pressing.unwrap_or(0);
        let (_, copies) = *PRESSING_TIERS
            .get(tier)
            .ok_or("Invalid pressing run selected.")?;
        Ok((copies, self.pressing_cost(release_type, copies)))
    }

    /// A label puts its promo machine behind every release it ships.
    pub(super) fn apply_label_promo(&mut self) {
        let Some(deal) = self.band.current_deal() else {
            return;
        };
        let push = (deal.market_reach / 2).clamp(10, 45);
        let label_name = deal.label_name.clone();
        if let Some(release) = self.just_released_music.last_mut() {
            release.marketing_level_achieved = push;
            let release_name = release.name.clone();
            self.log(format!(
                "📣 {} puts its promo machine behind '{}' (+{} buzz).",
                label_name, release_name, push
            ));
        }
    }

    /// Convert a sales score into copies moved and money in hand. Demand is
    /// score × reach; you can't sell copies that were never pressed.
    pub(super) fn calculate_release_outcome(
        &self,
        sales_score: u32,
        release: &Release,
    ) -> (u32, u32, bool) {
        let demand =
            (sales_score as f32 * self.distribution_multiplier() * UNITS_PER_SCORE_POINT) as u32;
        let sold_out = release.copies_pressed > 0 && demand > release.copies_pressed;
        let units_sold = if sold_out {
            release.copies_pressed
        } else {
            demand
        };
        let income = if let Some(deal) = self.band.current_deal() {
            ((units_sold * LABEL_INCOME_PER_COPY) as f32 * deal.royalty_rate) as u32
        } else {
            units_sold * INDIE_INCOME_PER_COPY
        };
        (income, units_sold, sold_out)
    }

    pub(super) fn process_music_releases_and_marketing(&mut self) {
        let current_week = self.week;

        let mut still_pending_release = Vec::new();
        for mut release in std::mem::take(&mut self.just_released_music) {
            if current_week >= release.week_released + INITIAL_SALES_WINDOW_WEEKS {
                let sales_score = self.calculate_release_sales_score(&release);
                release.initial_sales_score = sales_score;

                // The charts are a shared scoreboard: your record competes
                // against the scene's releases on the same sales scale.
                // TODO(M10): this is a minimal shim to keep the build green
                // after M3's regional-charts rework. It submits to Local
                // (always home turf) and UK (the design's home-territory
                // floor) at full score; it does not yet implement the
                // presence-scaled submission to Europe/America/Japan via
                // distribution channel / label market_reach, nor the
                // sum-over-territories demand model — both design §C, M10.
                let chart_position = self.world.submit_chart_entry(
                    world::ChartRegion::Local,
                    release.name.clone(),
                    self.band.name.clone(),
                    true,
                    sales_score,
                );
                self.world.submit_chart_entry(
                    world::ChartRegion::Uk,
                    release.name.clone(),
                    self.band.name.clone(),
                    true,
                    sales_score,
                );

                let (income, units_sold, sold_out) =
                    self.calculate_release_outcome(sales_score, &release);
                release.total_income_generated += income;
                release.copies_sold = units_sold;
                self.player.earn_money(income);

                let verdict = match sales_score {
                    0..=99 => "flopped",
                    100..=299 => "sold modestly",
                    300..=599 => "sold well",
                    _ => "is a SMASH HIT",
                };
                // A label's distribution spreads your name further than a
                // self-pressed run ever could.
                let fame_gain = if self.band.current_deal().is_some() {
                    (sales_score / 150).min(8) as u8
                } else {
                    (sales_score / 300).min(4) as u8
                };
                self.band.gain_fame(fame_gain);
                if fame_gain > 0 {
                    self.log(format!(
                        "💿 '{}' {} — moved {} copies, first-run earnings: ${}, fame +{}.",
                        release.name, verdict, units_sold, income, fame_gain
                    ));
                } else {
                    self.log(format!(
                        "💿 '{}' {} — moved {} copies, first-run earnings: ${}.",
                        release.name, verdict, units_sold, income
                    ));
                }
                if sold_out {
                    self.log(format!(
                        "📦 '{}' sold out — all {} copies gone; demand was there for more.",
                        release.name, release.copies_pressed
                    ));
                }
                if let Some(position) = chart_position {
                    release.peak_chart_position = Some(position as u8);
                    self.log(format!(
                        "📈 '{}' enters the charts at #{}.",
                        release.name, position
                    ));
                }

                let release_genre = release.genre.clone();
                if release.release_type == music::ReleaseType::Album {
                    if self.band.current_deal().is_some() && self.band.fulfill_album_obligation() {
                        self.log(
                            "🤝 That album completes your record deal — you're a free agent again!",
                        );
                    }
                    self.band.albums_released.push(release);
                } else {
                    self.band.singles_released.push(release);
                }

                if sales_score > PLAYER_MARKET_IMPACT_THRESHOLD_SALES_SCORE {
                    if let Some(genre_to_boost) = release_genre {
                        *self
                            .world
                            .dynamic_genre_modifiers
                            .entry(genre_to_boost)
                            .or_insert(1.0) += PLAYER_MARKET_IMPACT_GENRE_MOD_BONUS;
                    }
                    self.world.music_market.demand = (self.world.music_market.demand
                        + PLAYER_MARKET_IMPACT_DEMAND_BONUS)
                        .min(100);
                }
            } else {
                still_pending_release.push(release);
            }
        }
        self.just_released_music = still_pending_release;

        // Deal terms are captured up front: the catalogue loop below holds
        // mutable borrows into self.band, so it cannot call &self methods.
        let income_per_copy = if self.band.current_deal().is_some() {
            LABEL_INCOME_PER_COPY
        } else {
            INDIE_INCOME_PER_COPY
        };
        let royalty_rate = self.band.current_deal().map(|deal| deal.royalty_rate);
        let distribution = self.distribution_multiplier();
        let fame = self.band.fame as f32;
        let mut catalog_income_this_week: u32 = 0;

        for release_list in [
            &mut self.band.albums_released,
            &mut self.band.singles_released,
        ] {
            for release in release_list.iter_mut() {
                release
                    .active_marketing
                    .retain(|campaign| current_week < campaign.end_week);
                release.marketing_level_achieved = release
                    .active_marketing
                    .iter()
                    .map(|c| c.effectiveness_bonus as u32)
                    .sum::<u32>()
                    .min(100) as u8;

                if release.initial_sales_score > 0
                    && current_week > release.week_released + INITIAL_SALES_WINDOW_WEEKS
                {
                    let weeks_since_initial_window_end =
                        current_week - (release.week_released + INITIAL_SALES_WINDOW_WEEKS - 1);
                    let ongoing_sales_score_divisor =
                        1 + weeks_since_initial_window_end / TAIL_DIVISOR_WEEKS_PER_STEP;
                    let ongoing_sales_score = ((release.initial_sales_score as f32
                        + release.marketing_level_achieved as f32 * TAIL_MARKETING_WEIGHT
                        + fame * TAIL_FAME_WEIGHT)
                        / ongoing_sales_score_divisor as f32)
                        as u32;

                    if ongoing_sales_score > 10 {
                        // The long tail moves a trickle of copies — and only
                        // copies that still exist in the pressing.
                        let mut units = (ongoing_sales_score as f32
                            * distribution
                            * UNITS_PER_SCORE_POINT) as u32
                            / 5;
                        if release.copies_pressed > 0 {
                            units = units
                                .min(release.copies_pressed.saturating_sub(release.copies_sold));
                        }
                        if units == 0 {
                            continue;
                        }
                        release.copies_sold += units;
                        let gross = units * income_per_copy;
                        let ongoing_income = match royalty_rate {
                            Some(rate) => (gross as f32 * rate) as u32,
                            None => gross,
                        };
                        release.total_income_generated += ongoing_income;
                        self.player.earn_money(ongoing_income);
                        catalog_income_this_week += ongoing_income;
                    }
                }
            }
        }

        if catalog_income_this_week > 0 {
            self.log(format!(
                "💵 Catalog royalties trickle in: ${}.",
                catalog_income_this_week
            ));
        }
    }
}
