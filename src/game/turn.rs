//! Turn processing: the weekly tick. Advances the calendar, fires
//! events and news, ages offers, tracks visibility decay, and checks
//! for the end of the road.

use rand::Rng;

use super::constants::{self, *};
use super::rng;
use super::*;

impl Game {
    /// Fame gravity (design §C). While the band stays in the public eye the
    /// idle clock is held at zero; once it has been quiet past its grace
    /// window, fame drifts down a ramp (−1, −2, −3, −4, then −5/week) toward a
    /// permanent floor earned at the band's peak. Any activity resets the
    /// streak — and thus the ramp — to zero.
    pub(super) fn update_public_visibility(&mut self, action: &GameAction, weeks_elapsed: u32) {
        // Old saves default `peak_fame` to 0; lift it to the fame reached so
        // far before any floor reads it, so a loaded career keeps its peak.
        self.band.peak_fame = self.band.effective_peak_fame();

        if self.is_publicly_active(action) {
            self.idle_streak = 0;
            self.decay_streak = 0;
            return;
        }

        let mut faded: u8 = 0;
        for _ in 0..weeks_elapsed {
            self.idle_streak += 1;
            let grace = Self::idle_grace_weeks(self.band.fame);
            if self.idle_streak <= grace {
                continue;
            }
            // The ramp is its own clock: it counts *decaying* weeks, not
            // weeks-past-grace, so falling into a shorter grace tier
            // mid-decline cannot skip the gentle −1..−4 onset (§C).
            self.decay_streak += 1;
            let floor = self.fame_floor();
            if self.band.fame <= floor {
                continue; // decay stops dead at the floor
            }
            let ramp = self.decay_streak.min(u32::from(FAME_RAMP_MAX_DECAY)) as u8;
            let after = self.band.fame.saturating_sub(ramp).max(floor);
            faded = faded.saturating_add(self.band.fame - after);
            self.band.fame = after;
        }
        if faded > 0 {
            self.log(format!(
                "🕰️ Out of the public eye — the buzz cools (fame -{}).",
                faded
            ));
        }
    }

    /// The three ways to stay "in the picture" (design §C): a public action or
    /// a release in its launch window; any player record currently charting;
    /// or — once established (fame ≥ 60) — a release inside the recent window.
    fn is_publicly_active(&self, action: &GameAction) -> bool {
        let public_action = matches!(
            action,
            GameAction::Gig(_) | GameAction::GoOnTour(..) | GameAction::AcceptSupportTour
        ) || !self.just_released_music.is_empty();
        public_action
            || self.world.charts.iter().any(|entry| entry.is_player)
            || (self.band.fame >= ESTABLISHMENT_MIN_FAME && self.has_recent_release())
    }

    /// Whether any album or single (including one still in its launch window)
    /// came out within the establishment window.
    fn has_recent_release(&self) -> bool {
        self.band
            .albums_released
            .iter()
            .chain(self.band.singles_released.iter())
            .chain(self.just_released_music.iter())
            .any(|release| {
                self.week.saturating_sub(release.week_released)
                    <= ESTABLISHMENT_RELEASE_WINDOW_WEEKS
            })
    }

    /// Consecutive quiet weeks forgiven before decay, keyed by current fame.
    fn idle_grace_weeks(fame: u8) -> u32 {
        FAME_GRACE_TIERS
            .iter()
            .find(|(upper_fame, _)| fame <= *upper_fame)
            .map(|(_, grace)| *grace)
            .unwrap_or(FAME_GRACE_TIERS[FAME_GRACE_TIERS.len() - 1].1)
    }

    /// The permanent floor fame can never decay below, earned at the band's
    /// peak — with the top floor gated on a catalog of hits.
    fn fame_floor(&self) -> u8 {
        let peak = self.band.effective_peak_fame();
        let base = FAME_FLOOR_TIERS
            .iter()
            .find(|(min_peak, _)| peak >= *min_peak)
            .map(|(_, floor)| *floor)
            .unwrap_or(0);
        if peak >= 95 && self.hit_count() >= FAME_FLOOR_HITS_THRESHOLD {
            base.max(FAME_FLOOR_LEGEND)
        } else {
            base
        }
    }

    /// Released albums and singles that charted at all — the band's "hits".
    fn hit_count(&self) -> usize {
        self.band
            .albums_released
            .iter()
            .chain(self.band.singles_released.iter())
            .filter(|release| release.peak_chart_position.is_some())
            .count()
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

        self.maybe_trigger_incident(rng)?;

        self.player.weekly_health_decay();

        // World stream: scene + historical event selection (see `rng`).
        let mut wk_rng = rng::world_rng_for_week(self.world_seed, self.week as u64);

        // Drawn in a fixed order — historical events, then the scene update —
        // so a seed's world evolves the same regardless of what the player did.
        // Player-facing consequences of a historical event roll on the action
        // stream instead (`rng` on the process_turn path).
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

        self.label_single_cut_check(rng);
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
                | GameAction::ChangeLifestyle(_) // M2: instant, no week consumed
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
            self.update_lifestyle(&action);
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

        // Rockstar milestone: one-time event when thresholds are reached
        if !self.rockstar_achieved
            && self.band.fame >= constants::ROCKSTAR_FAME_THRESHOLD
            && self.band.albums_released.len() >= constants::ROCKSTAR_ALBUM_THRESHOLD as usize
        {
            self.rockstar_achieved = true;
            self.log(format!(
                "🌟 You've made it — {} is a bona fide ROCKSTAR. The game doesn't end here: legends are made in the second act.",
                self.band.name
            ));
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
        } else if self.game_over {
            "You walked away from the rock life on your own terms.".to_string()
        } else {
            "Game continues...".to_string()
        }
    }
}
