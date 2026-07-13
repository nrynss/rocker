//! Turn processing: the weekly tick. Advances the calendar, fires
//! events and news, ages offers, tracks visibility decay, and checks
//! for the end of the road.

use super::*;

impl Game {
    /// Track whether the band was in the public eye this turn. Fame starts
    /// to fade after IDLE_GRACE_WEEKS consecutive quiet weeks.
    pub(super) fn update_public_visibility(&mut self, action: &GameAction, weeks_elapsed: u32) {
        let publicly_active = matches!(
            action,
            GameAction::Gig(_) | GameAction::GoOnTour(_) | GameAction::AcceptSupportTour
        ) || !self.just_released_music.is_empty();
        if publicly_active {
            self.idle_streak = 0;
            return;
        }
        let mut faded: u8 = 0;
        for _ in 0..weeks_elapsed {
            self.idle_streak += 1;
            if self.idle_streak > IDLE_GRACE_WEEKS {
                faded = faded.saturating_add(IDLE_FAME_DECAY_PER_WEEK);
            }
        }
        let faded = faded.min(self.band.fame);
        if faded > 0 {
            self.band.fame -= faded;
            self.log(format!(
                "🕰️ Out of the public eye — the buzz cools (fame -{}).",
                faded
            ));
        }
    }

    /// Expire a stale support offer, or roll for a new one from a bigger
    /// scene act famous enough to headline over you.
    pub(super) fn update_support_tour_offer(&mut self, rng: &mut impl Rng) {
        if let Some(offer) = &self.pending_support_offer {
            if self.week >= offer.expires_week {
                let host = offer.host_band.clone();
                self.pending_support_offer = None;
                self.log(format!(
                    "🎟️ {}'s support slot went to another band — you sat on it too long.",
                    host
                ));
            }
            return;
        }

        if self.band.fame < SUPPORT_OFFER_MIN_FAME {
            return;
        }
        if !rng.gen_bool(SUPPORT_OFFER_CHANCE) {
            return;
        }

        let candidates: Vec<(String, u8)> = self
            .world
            .bands
            .iter()
            .filter(|b| b.fame >= self.band.fame.saturating_add(SUPPORT_OFFER_FAME_GAP))
            .map(|b| (b.name.clone(), b.fame))
            .collect();
        if candidates.is_empty() {
            return;
        }
        let (host_band, host_fame) = candidates[rng.gen_range(0..candidates.len())].clone();

        let weeks = rng.gen_range(2..=4u32);
        let base_pay = weeks * (50 + u32::from(host_fame) * 5);
        let pay = (base_pay as f32 * self.timeline.get_gig_pay_modifier()) as u32;
        let gap = host_fame.saturating_sub(self.band.fame);
        let fame_gain = (2 + gap / 8).clamp(3, 12);

        self.pending_support_offer = Some(SupportTourOffer {
            host_band: host_band.clone(),
            host_fame,
            weeks,
            pay,
            fame_gain,
            expires_week: self.week + SUPPORT_OFFER_LIFETIME_WEEKS,
        });
        self.log(format!(
            "🎟️ {} want '{}' opening their {}-week tour — ${} and real exposure. Press T to respond.",
            host_band, self.band.name, weeks, pay
        ));
    }

    /// When the era clearly loves — or has clearly abandoned — the band's
    /// sound, the press notices. Said once per swing, not every week.
    fn update_genre_trend_news(&mut self) {
        let era_fit = self
            .data_files
            .era_genre_modifier(self.timeline.get_current_year(), self.band.genre.aliases());
        let verdict: i8 = if era_fit >= GENRE_TREND_HOT {
            1
        } else if era_fit <= GENRE_TREND_COLD {
            -1
        } else {
            0
        };
        if verdict == self.genre_trend_reported {
            return;
        }
        self.genre_trend_reported = verdict;
        let genre = self.band.genre.name();
        match verdict {
            1 => self.log(format!(
                "🎸 {} is exploding — you're in the right scene at the right time.",
                genre
            )),
            -1 => self.log(format!(
                "🥶 {} is out of step with the times — the crowds are chasing a different sound.",
                genre
            )),
            _ => {}
        }
    }

    fn advance_week_events(&mut self, rng: &mut impl Rng) -> Result<(), String> {
        // Sync the timeline with the current week. Tours can jump several weeks
        // at once, so catch up year by year instead of testing a single boundary.
        let expected_year =
            constants::STARTING_YEAR + (self.week.saturating_sub(1)) / constants::WEEKS_PER_YEAR;
        while self.timeline.get_current_year() < expected_year {
            self.timeline.advance_year();
            let year = self.timeline.get_current_year();
            let era_name = self.timeline.get_current_era().era_name.clone();
            self.log(format!("🗓️ It's now {} — the era of {}.", year, era_name));
        }
        self.update_genre_trend_news();

        if let Some(event) = self.events.try_trigger_event(self.week, rng) {
            self.apply_random_event(event, rng)?;
        }

        self.player.weekly_health_decay();

        // Derive a weekly StdRng using splitmix64 key derivation from world_seed + week
        let mut key = self
            .world_seed
            .wrapping_add(self.week as u64)
            .wrapping_mul(0x9E3779B97F4A7C15);
        key = (key ^ (key >> 30)).wrapping_mul(0xBF58476D1CE4E5B8);
        key = (key ^ (key >> 27)).wrapping_mul(0x94D049BB133111EB);
        key ^= key >> 31;
        let mut wk_rng = StdRng::seed_from_u64(key);

        // The world stream (wk_rng) is drawn in a fixed order — historical
        // events, then the scene update — so a seed's world evolves the same
        // regardless of what the player did. The player-facing consequences
        // of a historical event roll on the action stream instead.
        if let Some(historical_event) = self.timeline.take_historical_event(&mut wk_rng) {
            self.apply_historical_event(&historical_event, rng)?;
            self.log(format!("📰 MUSIC NEWS: {}", historical_event));
        }

        let scene_news = self
            .world
            .update_week(&self.timeline, &self.data_files, &mut wk_rng);
        for item in scene_news {
            self.log(item);
        }

        self.update_support_tour_offer(rng);
        Ok(())
    }

    /// Quietly withdraw deal offers the player sat past their deadline.
    /// Losing interest is not a rejection: nobody poaches the vacated deal
    /// (that stays a consequence of `action_reject_deal` alone), and the
    /// cleared slate lets labels come knocking again. Offers from saves
    /// that predate expiry (`expires_week: None`) stay on the table
    /// forever, exactly as they always did.
    fn expire_stale_deal_offers(&mut self) {
        let week = self.week;
        let offers = std::mem::take(&mut self.pending_deal_offers);
        let (expired, live): (Vec<_>, Vec<_>) = offers
            .into_iter()
            .partition(|offer| offer.expires_week.is_some_and(|deadline| week >= deadline));
        self.pending_deal_offers = live;
        for offer in expired {
            self.log(format!(
                "📪 {}'s interest has cooled — their offer is off the table.",
                offer.label_name
            ));
        }
    }

    pub(super) fn check_and_generate_deal_offers(&mut self, rng: &mut impl Rng) {
        self.expire_stale_deal_offers();
        if self.pending_deal_offers.is_empty()
            && self.week.is_multiple_of(4)
            && self.band.record_deal.is_none()
        {
            let mut new_offers = self
                .world
                .generate_deal_offers(&self.band, &self.data_files, rng);
            for offer in &mut new_offers {
                offer.expires_week = Some(self.week + DEAL_OFFER_LIFETIME_WEEKS);
            }
            if !new_offers.is_empty() {
                let n = new_offers.len();
                self.pending_deal_offers = new_offers;
                self.log(format!(
                    "📬 {} record label{} sent you an offer — press V to review.",
                    n,
                    if n == 1 { "" } else { "s" }
                ));
            }
        }
    }

    pub fn process_turn(&mut self, action: GameAction) -> Result<bool, String> {
        if self.game_over {
            return Ok(false);
        }

        let is_turn_consuming_action = !matches!(
            action,
            GameAction::AcceptDeal(_)
                | GameAction::RejectDeal(_)
                | GameAction::DeclineSupportTour
                | GameAction::StartMarketingCampaign(_, _)
                | GameAction::Quit
        );

        // One stream for the whole turn, keyed by the week the player acted
        // in. Multi-week actions (tours, breaks) re-key next turn anyway.
        let mut rng = self.action_rng();

        let week_before = self.week;
        self.execute_action(action.clone(), &mut rng)?; // Execute action first

        if is_turn_consuming_action {
            self.week += 1; // Advance week only for turn-consuming actions
            self.advance_week_events(&mut rng)?; // Process standard weekly events
            self.update_public_visibility(&action, self.week - week_before);
        }

        // These happen after every action resolution, regardless of turn consumption
        self.process_music_releases_and_marketing();
        self.check_and_generate_deal_offers(&mut rng);
        self.check_game_over();

        Ok(!self.game_over)
    }

    fn check_game_over(&mut self) {
        if self.player.health == 0 {
            self.game_over = true;
        }
        if self.player.money < 0 && self.band.fame < 10 {
            self.game_over = true;
        }
        if self.band.fame >= constants::ROCKSTAR_FAME_THRESHOLD
            && self.band.albums_released.len() >= constants::ROCKSTAR_ALBUM_THRESHOLD as usize
        // Updated to check Vec length
        {
            self.game_over = true;
        }
    }

    pub fn is_game_over(&self) -> bool {
        self.game_over
    }

    pub fn get_status_message(&self) -> String {
        if self.player.health == 0 {
            "You died from poor health!".to_string()
        } else if self.player.money < 0 && self.band.fame < 10 {
            "You went broke and nobody knows who you are!".to_string()
        } else if self.band.fame >= constants::ROCKSTAR_FAME_THRESHOLD
            && self.band.albums_released.len() >= constants::ROCKSTAR_ALBUM_THRESHOLD as usize
        {
            "Congratulations! You're now a ROCKSTAR!".to_string()
        } else if self.game_over {
            "You walked away from the rock life on your own terms.".to_string()
        } else {
            "Game continues...".to_string()
        }
    }

    fn apply_random_event(
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

    fn apply_historical_event(&mut self, event: &str, rng: &mut impl Rng) -> Result<(), String> {
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
