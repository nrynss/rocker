//! Label initiatives: single-cuts and other label-driven actions
//! (design §C — Label single-cuts; §E-4/§E-5 — deal-clock breach, the
//! recoupment-dependent renewal window, memos, and recoup pressure).

use rand::Rng;

use super::constants::{
    DEAL_MEMO_CHANCE, DEAL_MEMO_DEADLINE_STRESS_PER_WEEK, DEAL_MEMO_DEADLINE_WINDOW_WEEKS,
    DEAL_MEMO_IDLE_WEEKS, DEAL_OFFER_LIFETIME_WEEKS, LABEL_CUT_CHANCE,
    LABEL_CUT_CHANCE_PRESSURE_MULTIPLIER, LABEL_CUT_IDLE_WEEKS, LABEL_CUT_IDLE_WEEKS_PRESSURED,
    LABEL_CUT_MAX_PER_ALBUM, LABEL_CUT_RELEASE_COOLDOWN_WEEKS, MAX_STRESS,
};
use super::music::ReleaseType;
use super::*;

impl Game {
    /// Check if the label should cut a single from an unreleased album.
    ///
    /// Fires only when ALL conditions hold (design §C):
    /// - signed to a label
    /// - some album has fewer than 2 singles already cut from it
    /// - the band has been quiet for ≥ 3 weeks (2 under recoup pressure, §E-5)
    /// - no release in the last 6 weeks
    /// - a 10% weekly roll succeeds (doubled under recoup pressure, §E-5)
    ///
    /// When it fires: picks the eligible album (most recent first), increments
    /// its cut counter, and creates a new SINGLE `Release` into
    /// `just_released_music` with label pressing and promo. The single enters
    /// the launch window like any release and counts toward activity rules.
    pub(super) fn label_single_cut_check(&mut self, rng: &mut impl Rng) {
        // Condition 1: must be signed.
        let Some(deal) = self.band.current_deal() else {
            return;
        };
        let label_name = deal.label_name.clone();
        let deal_market_reach = deal.market_reach;
        // M9 (design §E-5): a label in the red gets antsy about product —
        // the idle gate drops and the cut chance doubles while it's owed.
        let recoup_pressure = deal.unrecouped > 0;

        // Condition 2: find an eligible album (un-singled tracks, most recent first).
        // We need to check if there's an eligible album first.
        let has_eligible = self.label_has_cuttable_album();

        if !has_eligible {
            return;
        }

        // Condition 3: must be quiet (idle_streak ≥ 3, or ≥ 2 under pressure).
        let idle_gate = if recoup_pressure {
            LABEL_CUT_IDLE_WEEKS_PRESSURED
        } else {
            LABEL_CUT_IDLE_WEEKS
        };
        if self.idle_streak < idle_gate {
            return;
        }

        // Condition 4: no release in the last 6 weeks.
        if self.has_recent_release_for_cut() {
            return;
        }

        // Condition 5: roll on the rng (10% chance, doubled under recoup
        // pressure). This is the last check so weeks where earlier
        // conditions fail draw nothing from the stream.
        let cut_chance = if recoup_pressure {
            LABEL_CUT_CHANCE * LABEL_CUT_CHANCE_PRESSURE_MULTIPLIER
        } else {
            LABEL_CUT_CHANCE
        };
        if !rng.gen_bool(cut_chance) {
            return;
        }

        // All conditions passed — find and cut the single.
        // Now that we've passed all guards, we can mutate.
        let album = self
            .band
            .albums_released
            .iter_mut()
            .rev()
            .find(|album| album.singles_cut < LABEL_CUT_MAX_PER_ALBUM)
            .expect("has_eligible guarantees this");

        let album_name = album.name.clone();
        let album_quality = album.release_quality;
        let album_genre = album.genre.clone();

        // Increment the album's cut counter.
        album.singles_cut += 1;

        // Create a single cut from the album.
        let single_name = format!("{} (single)", album_name);

        // Label pressing size: the label's network plus your name (duplicated from economy.rs).
        let label_pressing = u32::from(deal_market_reach) * constants::LABEL_PRESSING_PER_REACH
            + u32::from(self.band.fame) * constants::LABEL_PRESSING_PER_FAME;

        let new_release = music::Release {
            id: self.next_release_id,
            name: single_name.clone(),
            release_type: ReleaseType::Single,
            release_quality: album_quality,
            week_released: self.week,
            songs_involved_quality_avg: album.songs_involved_quality_avg,
            active_marketing: Vec::new(),
            marketing_level_achieved: 0,
            initial_sales_score: 0,
            total_income_generated: 0,
            genre: album_genre,
            copies_pressed: label_pressing,
            copies_sold: 0,
            peak_chart_position: None,
            singles_cut: 0,
            certified: 0,
            // A label single-cut is always a signed act's release — channel
            // is meaningless (reach is `market_reach`, M6 §E-3). Freeze that
            // reach on the release so its tail survives the deal ending.
            distribution_channel: None,
            label_market_reach: Some(deal_market_reach),
        };

        self.just_released_music.push(new_release);
        self.next_release_id += 1;

        // Log the cut in the design's voice.
        self.log(format!(
            "📀 Without asking, {} pulls '{}' off the album as a single.",
            label_name, single_name
        ));

        // Apply the label's promo machine (same as any label release).
        self.apply_label_promo();
    }

    /// Check if any release (album, single, or launch-window) was released
    /// within the cooldown window. Used by label_single_cut_check.
    fn has_recent_release_for_cut(&self) -> bool {
        let cutoff_week = self.week.saturating_sub(LABEL_CUT_RELEASE_COOLDOWN_WEEKS);
        self.band
            .albums_released
            .iter()
            .chain(self.band.singles_released.iter())
            .chain(self.just_released_music.iter())
            .any(|release| release.week_released > cutoff_week)
    }

    /// Whether some released album still has un-singled tracks (fewer than
    /// `LABEL_CUT_MAX_PER_ALBUM` cuts) — the single-cut mechanic's own
    /// eligibility check, shared with the "cut a single" memo (§E-5).
    fn label_has_cuttable_album(&self) -> bool {
        self.band
            .albums_released
            .iter()
            .any(|album| album.singles_cut < LABEL_CUT_MAX_PER_ALBUM)
    }

    /// The weekly deal-clock check (design §E-4/§E-5): breach, the
    /// cooldown, the renewal window, and label memos. Runs every week
    /// regardless of what the player did, alongside the single-cut check —
    /// the same "existing action stream" the label's other moves draw
    /// from. Wired from `turn.rs::advance_week_events`.
    pub(super) fn label_weekly_deal_check(&mut self, rng: &mut impl Rng) {
        // The cooldown from a past breach ticks down every week, deal or no
        // deal — it's what eventually lets new offers through again.
        self.band.tick_deal_cooldown();

        // Breach: the term's clock, independent of any release this week.
        if let Some(breach) = self.band.check_term_breach(self.week) {
            self.log(format!(
                "💔 {} drops you — the contract ran out with albums still owed.",
                breach.label_name
            ));
            if breach.written_off > 0 {
                self.log(format!(
                    "🗑️ {} writes off ${} still owed — they don't expect to see it now.",
                    breach.label_name, breach.written_off
                ));
            }
            // The deal just ended; nothing else this week is signed to.
            return;
        }

        // Free agency on the calendar alone (design §E-4): a deal that
        // delivered its albums early only has the term left to run out —
        // there's no reason to expect another release to trigger it.
        if let Some(label_name) = self.band.check_term_served_free_agency(self.week) {
            self.log(format!(
                "🤝 {}'s term is up — you're a free agent again!",
                label_name
            ));
            return;
        }

        let Some(deal) = self.band.current_deal() else {
            return;
        };
        let term_end_week = deal.term_end_week();

        // The renewal window (design §E-4): only while there's no offer
        // already on the table — the same guard the brand-new-signing
        // stream (`check_and_generate_deal_offers`) uses.
        if self.pending_deal_offers.is_empty()
            && let Some(mut offer) =
                self.world
                    .generate_renewal_offer(&self.band, &self.data_files, rng, self.week)
        {
            offer.expires_week = Some(self.week + DEAL_OFFER_LIFETIME_WEEKS);
            let label_name = offer.label_name.clone();
            let is_extension = offer.carry_forward_unrecouped > 0;
            self.pending_deal_offers = vec![offer];
            self.log(format!(
                "📬 {} puts {} on the table as your deal winds down — press V to review.",
                label_name,
                if is_extension {
                    "an extension"
                } else {
                    "a new contract"
                }
            ));
        }

        self.label_memo_check(rng, term_end_week);
    }

    /// Label memos (design §E-5): the label asks before it takes. Each
    /// condition is checked independently; at most one memo message logs
    /// per week (priority: deadline pressure, then the cut-single nudge,
    /// then the write-songs nudge). The deadline's stress bite applies
    /// whenever the condition holds, whether or not the memo message
    /// itself rolled — "the deadline is real pressure, not flavor".
    fn label_memo_check(&mut self, rng: &mut impl Rng, term_end_week: u32) {
        // Pull everything needed out of the deal up front — the borrow
        // can't survive into the `&mut self` calls (log, stress) below.
        let Some(deal) = self.band.current_deal() else {
            return;
        };
        let label_name = deal.label_name.clone();
        let albums_owed = deal.albums_owed();
        let albums_remaining = deal.albums_required.saturating_sub(deal.albums_delivered);
        // Only a *real* term has a deadline. A legacy deal (`term_weeks == 0`)
        // has `term_end_week == signed_week`, so `weeks_left` would saturate
        // to 0 and read as "past deadline" — phantom pressure on a deal that
        // can never breach. Gate on a real term (same guard the renewal
        // window uses via `renewal_window_open`).
        let has_real_term = deal.term_weeks > 0;
        let weeks_left = term_end_week.saturating_sub(self.week);
        let deadline_pressure =
            has_real_term && albums_owed && weeks_left <= DEAL_MEMO_DEADLINE_WINDOW_WEEKS;

        if deadline_pressure {
            self.player.stress = self
                .player
                .stress
                .saturating_add(DEAL_MEMO_DEADLINE_STRESS_PER_WEEK)
                .min(MAX_STRESS);
        }

        let no_progress =
            self.band.unreleased_songs.is_empty() && self.idle_streak >= DEAL_MEMO_IDLE_WEEKS;
        let material_idle =
            self.label_has_cuttable_album() && self.idle_streak >= DEAL_MEMO_IDLE_WEEKS;

        // One memo max per week (priority: deadline, then cut-single, then
        // write-songs); each condition still rolls independently.
        if deadline_pressure && rng.gen_bool(DEAL_MEMO_CHANCE) {
            self.log(format!(
                "📠 {}: 'The contract says {} more album{}. The clock says {} weeks.'",
                label_name,
                albums_remaining,
                if albums_remaining == 1 { "" } else { "s" },
                weeks_left
            ));
        } else if material_idle && rng.gen_bool(DEAL_MEMO_CHANCE) {
            self.log(format!(
                "📠 {}: 'Cut a single from that material — this week, ideally.'",
                label_name
            ));
        } else if no_progress && rng.gen_bool(DEAL_MEMO_CHANCE) {
            self.log(format!(
                "📠 {}: 'We need songs on tape. Write.'",
                label_name
            ));
        }
    }
}
