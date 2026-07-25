//! Turn processing: the weekly tick. Advances the calendar, fires
//! events and news, ages offers, tracks visibility decay, and checks
//! for the end of the road.

use rand::Rng;

use super::actions::TourEnding;
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
            || self.world.player_is_charting()
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
        // L-SCALE (v0.8 §B): support-slot pay ×3 to the era-real money scale —
        // the per-week base (was `50 + host_fame * 5`) triples to `150 +
        // host_fame * 15`, in lockstep with tour/gig income so a support run
        // stays a real cash path. `weeks` is a count, not a dollar — unchanged.
        let base_pay = weeks * (150 + u32::from(host_fame) * 15);
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
        // Sync the timeline with the current week. Since v0.9 §F the calendar
        // only ever moves one week at a time, but a loaded save can still
        // arrive several years past the timeline, so catch up year by year
        // rather than testing a single boundary.
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
        // M9 (design §E-4/§E-5): the deal's clock — cooldown decrement,
        // term-expiry breach, and the renewal window/memos — runs every
        // week alongside the single-cut check, on the same action stream.
        self.label_weekly_deal_check(rng);
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

    /// Resolve one more week of the commitment in progress (design §F).
    /// Each activity's own module owns its week: this is only the dispatch.
    /// Public within `game` so the tests can drive a whole tour with no
    /// weekly tick in between and prove the split changed nothing.
    pub(in crate::game) fn resolve_activity_week(&mut self) {
        // Taken out and handed to the resolver by value: a week's resolution
        // mutates the band, the player and the log, and the activity has to
        // be mutable throughout. Whatever comes back — a week further on, or
        // nothing at all because it settled — is what the resolver puts back.
        match self.ongoing_activity.take() {
            // §D: a session's weeks decide nothing — the record was settled at
            // booking — so this arm only counts them down and settles the
            // master at the end.
            Some(OngoingActivity::Recording(session)) => self.resolve_recording_week(session),
            Some(OngoingActivity::Tour(tour)) => self.resolve_tour_week(tour),
            Some(OngoingActivity::SupportTour(support)) => self.resolve_support_tour_week(support),
            Some(OngoingActivity::Break(rest)) => self.resolve_break_week(rest),
            None => {}
        }
    }

    /// §F — Abandoning: walk out of whatever is running. You keep what
    /// already resolved and forfeit the rest of the commitment, and the
    /// money still on the table with it.
    pub(in crate::game) fn action_abandon_activity(&mut self) -> Result<(), String> {
        if self.ongoing_activity.is_none() {
            return Err("You're not in the middle of anything to walk out of.".to_string());
        }
        self.abandon_ongoing_activity();
        Ok(())
    }

    /// Settle whatever is running as walked-out-of. Shared by the player's
    /// own `AbandonActivity` and by the emergency path in `check_game_over`,
    /// so an ending can never leave a commitment dangling for a save to
    /// resume into.
    fn abandon_ongoing_activity(&mut self) {
        match self.ongoing_activity.take() {
            Some(OngoingActivity::Recording(session)) => self.settle_recording(session, false),
            Some(OngoingActivity::Tour(tour)) => self.settle_tour(tour, TourEnding::Abandoned),
            Some(OngoingActivity::SupportTour(support)) => self.settle_support_tour(support, false),
            Some(OngoingActivity::Break(rest)) => self.settle_break(rest, false),
            None => {}
        }
    }

    /// The two actions a committed band may still take (design §F): walking
    /// out, and walking away from the game entirely. Everything else the
    /// player submits while an activity runs is ignored — the UI offers
    /// "continue", not a menu, and a stale keypress must not be able to
    /// start a second commitment on top of the first.
    fn overrides_ongoing_activity(action: &GameAction) -> bool {
        matches!(action, GameAction::AbandonActivity | GameAction::Quit)
    }

    pub fn process_turn(&mut self, action: GameAction) -> Result<bool, String> {
        if self.game_over {
            return Ok(false);
        }

        // One stream for the whole turn, keyed by the week the player acted
        // in. Since §F the calendar always moves exactly one week, so
        // consecutive turns always get consecutive streams.
        let mut rng = self.action_rng();

        // §F: a commitment in progress owns the turn. The submitted action is
        // ignored and one more week of the activity resolves instead — but
        // the *turn machinery* still needs to know what the band spent the
        // week doing, so the action that started the commitment stands in
        // for it (fame visibility §C and the lifestyle streaks both read it).
        let action =
            if self.ongoing_activity.is_some() && !Self::overrides_ongoing_activity(&action) {
                let carried = self
                    .ongoing_activity
                    .as_ref()
                    .expect("just checked")
                    .as_action();
                self.resolve_activity_week();
                carried
            } else {
                self.execute_action(action.clone(), &mut rng)?;
                action
            };

        let is_turn_consuming_action = !matches!(
            action,
            GameAction::AcceptDeal(_)
                | GameAction::RejectDeal(_)
                | GameAction::DeclineSupportTour
                | GameAction::StartMarketingCampaign(_, _)
                | GameAction::ChangeLifestyle(_) // M2: instant, no week consumed
                | GameAction::RePress { .. } // M6: instant re-press, no week consumed (§E-1)
                // §D: the weeks a record costs were the session's. Putting the
                // finished tapes on a truck is paperwork, and paperwork is
                // instant here — the same call `RePress` above already makes.
                | GameAction::ReleaseMaster { .. }
                | GameAction::AbandonActivity // §F: the week you walk out of is yours to spend
                | GameAction::Quit
        );

        if is_turn_consuming_action {
            // §F: the one place in the codebase that moves the calendar, and
            // it only ever moves it by one.
            self.week += 1;
            self.advance_week_events(&mut rng)?; // Process standard weekly events
            self.update_public_visibility(&action, 1);
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

        // §F — Abandoning: health and money emergencies force the walk-out
        // the player never got to choose. Without this the career ends with
        // a tour still on the books, and a save reloaded past the ending
        // would resume into a week of a run that nobody is on.
        if self.game_over && self.ongoing_activity.is_some() {
            self.abandon_ongoing_activity();
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
