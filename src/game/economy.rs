//! The money pipeline: recording and pressing costs, sales scoring,
//! and the weekly release/catalog payout.

use crate::game::music::{DistributionChannel, Release, ReleaseType};

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

    /// How much of a release's potential audience the band can actually
    /// reach. A label brings its distribution network; an independent act is
    /// capped by its own fame, floored by whatever channel it bought for
    /// this release (design §E-3, M6: `max(channel floor, fame formula)`).
    /// `channel` is ignored once signed — a label deal's `market_reach`
    /// always wins.
    fn distribution_multiplier(&self, channel: Option<DistributionChannel>) -> f32 {
        Self::reach_for(
            self.band.fame,
            self.band.current_deal().map(|deal| deal.market_reach),
            channel,
        )
    }

    /// The pure math behind [`Game::distribution_multiplier`], taking
    /// already-captured state instead of `&self` (M6): the sales-tail loop
    /// in [`Game::process_music_releases_and_marketing`] holds a mutable
    /// borrow of `self.band` per release and cannot call back into `&self`
    /// methods, so it captures `fame`/`market_reach` once up front and calls
    /// this directly for each release's own `distribution_channel`.
    fn reach_for(fame: u8, market_reach: Option<u8>, channel: Option<DistributionChannel>) -> f32 {
        match market_reach {
            Some(reach) => 0.5 + f32::from(reach) / 100.0,
            None => {
                let indie_formula =
                    INDIE_REACH_FLOOR + (f32::from(fame) / 100.0) * (1.0 - INDIE_REACH_FLOOR);
                indie_formula.max(channel.unwrap_or_default().reach_floor())
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
        // M5 (§E-2): the label's outlay on this release joins the recoupment
        // ledger — the pressing run (the same run `plan_pressing` hands a
        // signed release) at $/copy, plus the promo push at $/point. This is
        // the accrual point for "at each release the label's outlay is added".
        let pressing_copies = self.label_pressing_size(deal);
        let outlay = (pressing_copies as f32 * LABEL_RECOUP_PRESSING_PER_COPY) as i32
            + i32::from(push) * LABEL_RECOUP_PROMO_PER_PUSH;
        if let Some(release) = self.just_released_music.last_mut() {
            release.marketing_level_achieved = push;
            let release_name = release.name.clone();
            self.log(format!(
                "📣 {} puts its promo machine behind '{}' (+{} buzz).",
                label_name, release_name, push
            ));
        }
        if let Some(deal) = self.band.record_deal.as_mut() {
            deal.unrecouped = deal.unrecouped.saturating_add(outlay);
        }
    }

    /// M5 (§E-2): route a signed act's royalty through the label's recoupment
    /// ledger. While `unrecouped > 0` the royalty pays that balance down first
    /// and only the remainder reaches the player; an unsigned act — or a
    /// cleared ledger — passes the full amount straight through. Returns the
    /// dollars that actually reach the player.
    fn apply_recoupment(&mut self, royalty: u32) -> u32 {
        let Some(deal) = self.band.record_deal.as_mut() else {
            return royalty;
        };
        if deal.unrecouped <= 0 {
            return royalty;
        }
        let applied = (royalty as i32).min(deal.unrecouped);
        deal.unrecouped -= applied;
        royalty - applied as u32
    }

    /// M5 (§E-1, label half): a signed act's release that sold out or crossed
    /// a certification level makes the label press a fresh run — the same size
    /// `plan_pressing` would hand it — restocking the catalog tail. The new
    /// pressing cost joins the recoupment ledger (§E-2). The indie,
    /// player-initiated re-press is M6's job, not this.
    fn label_auto_repress(&mut self, release: &mut Release, reason: &str) {
        let Some(deal) = self.band.current_deal() else {
            return;
        };
        let fresh_run = self.label_pressing_size(deal);
        if fresh_run == 0 {
            return;
        }
        let label_name = deal.label_name.clone();
        release.copies_pressed = release.copies_pressed.saturating_add(fresh_run);
        let outlay = (fresh_run as f32 * LABEL_RECOUP_PRESSING_PER_COPY) as i32;
        if let Some(deal) = self.band.record_deal.as_mut() {
            deal.unrecouped = deal.unrecouped.saturating_add(outlay);
        }
        let release_name = release.name.clone();
        self.log(format!(
            "🏭 {} presses a fresh run of '{}' ({}) — {} more copies in stores.",
            label_name, release_name, reason, fresh_run
        ));
    }

    /// M5 (§E-1, label half — tail path): [`Game::label_auto_repress`] for a
    /// release that already lives inside `self.band` (the catalog tail), so it
    /// is addressed by id and each step takes a fresh, non-overlapping borrow
    /// rather than a `&mut Release` that would conflict with `&mut self`.
    fn label_auto_repress_by_id(&mut self, release_id: u32, reason: &str) {
        let Some(deal) = self.band.current_deal() else {
            return;
        };
        let fresh_run = self.label_pressing_size(deal);
        let label_name = deal.label_name.clone();
        if fresh_run == 0 {
            return;
        }
        // Bump the release's pressing in place (albums first, then singles).
        let mut release_name = None;
        for list in [
            &mut self.band.albums_released,
            &mut self.band.singles_released,
        ] {
            if let Some(release) = list.iter_mut().find(|r| r.id == release_id) {
                release.copies_pressed = release.copies_pressed.saturating_add(fresh_run);
                release_name = Some(release.name.clone());
                break;
            }
        }
        let Some(release_name) = release_name else {
            return;
        };
        let outlay = (fresh_run as f32 * LABEL_RECOUP_PRESSING_PER_COPY) as i32;
        if let Some(deal) = self.band.record_deal.as_mut() {
            deal.unrecouped = deal.unrecouped.saturating_add(outlay);
        }
        self.log(format!(
            "🏭 {} presses a fresh run of '{}' ({}) — {} more copies in stores.",
            label_name, release_name, reason, fresh_run
        ));
    }

    /// Convert a sales score into copies moved and money in hand. Demand is
    /// score × reach; you can't sell copies that were never pressed.
    pub(super) fn calculate_release_outcome(
        &self,
        sales_score: u32,
        release: &Release,
    ) -> (u32, u32, bool) {
        let demand = (sales_score as f32
            * self.distribution_multiplier(release.distribution_channel)
            * UNITS_PER_SCORE_POINT) as u32;
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

    // ========================================================================
    // Certifications (design §D): records certify off cumulative copies_sold.
    // One-shot per level when crossing a new threshold. Awards fame, happiness,
    // and commercial_success reputation.
    // ========================================================================

    /// Compute the certification level (0 = none, 1 = silver, 2 = gold,
    /// 3 = platinum, 4+ = multi-platinum count) from copies_sold.
    pub(crate) fn compute_certification_level(copies_sold: u32) -> u8 {
        if copies_sold < CERT_SILVER_THRESHOLD {
            0
        } else if copies_sold < CERT_GOLD_THRESHOLD {
            1
        } else if copies_sold < CERT_PLATINUM_THRESHOLD {
            2
        } else {
            // Platinum or multi-platinum: 3 + (additional 400k tiers)
            let multiplatinum_count =
                (copies_sold - CERT_PLATINUM_THRESHOLD) / CERT_MULTIPLATINUM_STEP;
            3 + multiplatinum_count as u8
        }
    }

    /// Compute certification awards for a release that has crossed a threshold.
    /// Returns a tuple of (old_level, new_level) if a new level was crossed, or None.
    pub(crate) fn compute_certification_awards(release: &Release) -> Option<(u8, u8)> {
        let new_level = Self::compute_certification_level(release.copies_sold);
        if new_level > release.certified {
            Some((release.certified, new_level))
        } else {
            None
        }
    }

    /// Apply the certification awards to the game state for a given level transition.
    pub(crate) fn apply_certification_awards(
        &mut self,
        old_level: u8,
        new_level: u8,
        release_name: &str,
        copies_sold: u32,
    ) {
        // Award bumps for each level from the old to the new (shouldn't normally skip).
        // Index into the bumps arrays (silver=0, gold=1, platinum=2).
        for level in (old_level + 1)..=new_level.min(3) {
            let idx = (level - 1) as usize;
            let level_name = match level {
                1 => "SILVER",
                2 => "GOLD",
                3 => "PLATINUM",
                _ => unreachable!(),
            };

            // Log the achievement.
            self.log(format!(
                "🏆 '{}' is certified {} — {} copies sold.",
                release_name, level_name, copies_sold
            ));

            // Award bumps (capped fame, capped happiness [0-100], capped reputation).
            if idx < CERT_FAME_BUMP.len() {
                self.band.gain_fame(CERT_FAME_BUMP[idx]);
            }
            if idx < CERT_HAPPINESS_BUMP.len() {
                self.player.happiness = self
                    .player
                    .happiness
                    .saturating_add(CERT_HAPPINESS_BUMP[idx])
                    .min(100);
            }
            if idx < CERT_COMMERCIAL_SUCCESS_BUMP.len() {
                self.band.reputation.commercial_success = self
                    .band
                    .reputation
                    .commercial_success
                    .saturating_add(CERT_COMMERCIAL_SUCCESS_BUMP[idx])
                    .min(100);
            }
        }

        // If multi-platinum (level > 3), award platinum bumps for each
        // *newly crossed* tier. Count from max(old_level, 3), not from 3:
        // an entry already at ×2 (level 4) climbing to ×3 (level 5) crosses
        // one new tier, not two — otherwise every step past ×2 re-awards
        // tiers it already banked (and double-logs).
        if new_level > 3 {
            let multiplatinum_count = new_level - old_level.max(3);
            for _ in 0..multiplatinum_count {
                // Award platinum bumps again for each multi-platinum tier.
                self.log(format!(
                    "🏆 '{}' is certified PLATINUM — {} copies sold.",
                    release_name, copies_sold
                ));
                self.band.gain_fame(CERT_FAME_BUMP[2]);
                self.player.happiness = self
                    .player
                    .happiness
                    .saturating_add(CERT_HAPPINESS_BUMP[2])
                    .min(100);
                self.band.reputation.commercial_success = self
                    .band
                    .reputation
                    .commercial_success
                    .saturating_add(CERT_COMMERCIAL_SUCCESS_BUMP[2])
                    .min(100);
            }
        }
    }

    pub(super) fn process_music_releases_and_marketing(&mut self) {
        let current_week = self.week;

        // M5 (§E-2): snapshot the recoupment ledger so the weekly status line
        // at the end can tell "still in the red" from "cleared this week".
        let unrecouped_at_start = self
            .band
            .current_deal()
            .map(|deal| deal.unrecouped)
            .unwrap_or(0);

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
                // M5 (§E-2): royalties recoup the label's ledger before the
                // player is paid. `total_income_generated` stays the record's
                // gross earning; only what reaches the bank is netted here.
                let to_player = self.apply_recoupment(income);
                self.player.earn_money(to_player);

                // Check for certification milestones (§D).
                let mut certified_this_pass = false;
                if let Some((old_level, new_level)) = Self::compute_certification_awards(&release) {
                    let release_name = release.name.clone();
                    let copies_sold = release.copies_sold;
                    release.certified = new_level;
                    self.apply_certification_awards(
                        old_level,
                        new_level,
                        &release_name,
                        copies_sold,
                    );
                    certified_this_pass = true;
                }

                // M5 (§E-1, label half): a signed act's sold-out or freshly
                // certified release makes the label press a fresh run (its
                // cost joins the ledger, §E-2). Sold-out takes priority — a
                // sold-out record can't also have certified this same pass
                // without more stock, and a single fresh run answers both.
                if self.band.current_deal().is_some() {
                    if sold_out {
                        self.label_auto_repress(&mut release, "sold out");
                    } else if certified_this_pass {
                        self.label_auto_repress(&mut release, "certified");
                    }
                }

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
                    // M9 (design §E-4): free agency comes at the LATER of
                    // all albums delivered and the term served — an early
                    // finish keeps the band signed (and the recoupment
                    // ledger alive) until the clock runs out too.
                    match self.band.fulfill_album_obligation(current_week) {
                        band::DealCompletionOutcome::FreeAgent { label_name } => {
                            self.log(format!(
                                "🤝 That album completes your deal with {} — you're a free agent again!",
                                label_name
                            ));
                        }
                        band::DealCompletionOutcome::ObligationDelivered {
                            label_name,
                            term_end_week,
                        } => {
                            self.log(format!(
                                "🤝 Obligation delivered — under contract with {} until week {}.",
                                label_name, term_end_week
                            ));
                        }
                        band::DealCompletionOutcome::StillActive => {}
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
        // M6: reach depends on each release's own distribution channel, so
        // the scalar `distribution` this loop used to hoist is now computed
        // per release via `Self::reach_for` — same reason `market_reach` (not
        // a ready multiplier) is captured here rather than above.
        let market_reach = self.band.current_deal().map(|deal| deal.market_reach);
        let fame = self.band.fame as f32;
        // Gross catalog royalty this week, pooled across the whole back
        // catalog. It cannot be paid out inside the loop below (which holds a
        // mutable borrow of `self.band`), so M5's recoupment paydown and the
        // player payout both happen once, after the loop.
        let mut catalog_gross_this_week: u32 = 0;

        // Collect certifications to apply after the loop (to avoid borrow checker issues).
        let mut certifications_to_award: Vec<(String, u8, u8, u32)> = Vec::new();

        // M5 (§E-1, label half — tail path): a signed release runs its stock
        // down over months, and certification (§D) lands here on the tail, not
        // on the first run. Record the ids that need a fresh label run — stock
        // depleted, or a certification level crossed — and re-press them after
        // the loop (which holds a mutable borrow of `self.band`). Without this
        // a signed act caps out at one ~12k label run and can never certify.
        let mut tail_stock_capped: Vec<u32> = Vec::new();
        let mut tail_certified: Vec<u32> = Vec::new();

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
                        // copies that still exist in the pressing. Reach is
                        // this release's own channel (M6), not a global
                        // scalar — a Regional/National upgrade never
                        // retroactively boosts older stock's tail.
                        let distribution =
                            Self::reach_for(fame as u8, market_reach, release.distribution_channel);
                        let wanted = (ongoing_sales_score as f32
                            * distribution
                            * UNITS_PER_SCORE_POINT) as u32
                            / 5;
                        let mut units = wanted;
                        let mut stock_capped = false;
                        if release.copies_pressed > 0 {
                            let remaining =
                                release.copies_pressed.saturating_sub(release.copies_sold);
                            if wanted >= remaining {
                                // Demand met or outran the shelf: the record is
                                // selling out. It sells what's left this week.
                                units = remaining;
                                stock_capped = true;
                            }
                        }
                        // M5 (§E-1): a signed release whose tail demand is
                        // throttled by depleted stock needs a fresh label run.
                        // Indies never auto-repress — that's the player's M6
                        // RePress. Deferred to after the loop (borrow).
                        if royalty_rate.is_some() && stock_capped {
                            tail_stock_capped.push(release.id);
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
                        catalog_gross_this_week += ongoing_income;

                        // Check for certification milestones after each tail sale (§D).
                        // We collect these and apply them after the loop to avoid borrow checker issues.
                        if let Some((old_level, new_level)) =
                            Self::compute_certification_awards(release)
                        {
                            certifications_to_award.push((
                                release.name.clone(),
                                old_level,
                                new_level,
                                release.copies_sold,
                            ));
                            release.certified = new_level;
                            // M5 (§E-1): a signed release that certifies on the
                            // tail also triggers a fresh run (kept in stores).
                            if royalty_rate.is_some() {
                                tail_certified.push(release.id);
                            }
                        }
                    }
                }
            }
        }

        // Apply all collected certifications.
        for (release_name, old_level, new_level, copies_sold) in certifications_to_award {
            self.apply_certification_awards(old_level, new_level, &release_name, copies_sold);
        }

        // M5 (§E-1, label half — tail path): re-press the signed releases that
        // ran their stock down or certified this week, so a live hit stays in
        // stores and can keep selling toward the next certification. Dedup so a
        // release that did both gets one fresh run; certification carries the
        // more informative reason. Certifications are applied first (above), so
        // their fame bump sizes the fresh run — same ordering as the first-run
        // path.
        let mut planned_repress: std::collections::HashSet<u32> = std::collections::HashSet::new();
        for id in tail_certified {
            if planned_repress.insert(id) {
                self.label_auto_repress_by_id(id, "certified");
            }
        }
        for id in tail_stock_capped {
            if planned_repress.insert(id) {
                self.label_auto_repress_by_id(id, "sold out");
            }
        }

        // M5 (§E-2): the week's catalog royalty recoups the label's ledger
        // before it reaches the player. For an indie (or a cleared ledger) the
        // whole pool passes through, so the trickle line reads as it always
        // did; while in the red, $0 reaches the bank and the recoup line below
        // carries the news instead.
        let catalog_to_player = self.apply_recoupment(catalog_gross_this_week);
        if catalog_to_player > 0 {
            self.player.earn_money(catalog_to_player);
            self.log(format!(
                "💵 Catalog royalties trickle in: ${}.",
                catalog_to_player
            ));
        }

        // M5 (§E-2): one aggregated status line per week — never once per
        // release. While the label is still owed, report the balance; the week
        // it clears, say so once (the snapshot at the top of the pass tells a
        // still-red ledger from one that just went even).
        if let Some(deal) = self.band.current_deal() {
            if deal.unrecouped > 0 {
                self.log(format!(
                    "⚖️ Label recouping: ${} remaining.",
                    deal.unrecouped
                ));
            } else if unrecouped_at_start > 0 {
                let label_name = deal.label_name.clone();
                self.log(format!(
                    "✅ {} has recouped in full — the royalties are yours now.",
                    label_name
                ));
            }
        }
    }
}
