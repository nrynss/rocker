//! Player weekly actions (split by concern). Methods remain on `Game`.

use crate::game::music::{ActiveMarketingCampaign, DistributionChannel, MarketingCampaignType};
use rand::Rng;

use super::super::constants;
use super::super::*;

impl Game {
    /// The fee due at release under an indie distribution channel (design
    /// §E-3, M6) — `Ok(0)` while signed, since a label deal's `market_reach`
    /// ignores channels entirely. Validates the fame gate here too, so a
    /// stale channel choice (fame slipped, or a pre-M6 default) can never
    /// charge — or silently apply — a channel the band no longer qualifies
    /// for. `pub` (not `pub(in crate::game)`): the picker UI previews this
    /// fee before the player commits to recording.
    pub fn plan_distribution(&self, channel: DistributionChannel) -> Result<i32, String> {
        if self.band.current_deal().is_some() {
            return Ok(0);
        }
        if !channel.is_available(self.band.fame) {
            return Err(format!(
                "{} needs {} fame — you're not there yet.",
                channel.label(),
                channel.fame_gate()
            ));
        }
        Ok(channel.fee())
    }

    /// Releases eligible for an indie re-press right now (design §E-1):
    /// pressed to a finite run (`copies_pressed > 0` — pre-0.6 uncapped
    /// legacy stock never qualifies) and sold through
    /// `REPRESS_LOW_STOCK_SOLD_RATIO` of it. A signed act never sees this
    /// list — its label re-presses on its own initiative
    /// (`economy::label_auto_repress`, M5), not the player's.
    pub fn repressable_releases(&self) -> Vec<&music::Release> {
        if self.band.current_deal().is_some() {
            return Vec::new();
        }
        self.band
            .singles_released
            .iter()
            .chain(self.band.albums_released.iter())
            .filter(|release| {
                release.copies_pressed > 0
                    && release.copies_sold as f32
                        >= release.copies_pressed as f32 * constants::REPRESS_LOW_STOCK_SOLD_RATIO
            })
            .collect()
    }

    /// §E-1 (indie half), M6: re-press an already-released record that's
    /// sold out or running low, at a pressing tier the player chooses and
    /// pays for out of pocket. Instant — no week consumed (`turn.rs`).
    /// Signed acts don't get a choice here at all: the label restocks on its
    /// own (`economy::label_auto_repress`), so this stays player-only.
    pub(in crate::game) fn action_re_press(
        &mut self,
        release_id: u32,
        pressing: Option<usize>,
    ) -> Result<(), String> {
        if let Some(deal) = self.band.current_deal() {
            return Err(format!(
                "{} restocks the catalog automatically — re-pressing by hand isn't your call while you're signed.",
                deal.label_name
            ));
        }

        // Read-only lookup first: type and stock decide eligibility and the
        // pressing bill, both needed before the mutable borrow below (same
        // split M5's `label_auto_repress_by_id` uses for the same reason).
        let (release_type, copies_pressed, copies_sold) = self
            .band
            .singles_released
            .iter()
            .chain(self.band.albums_released.iter())
            .find(|release| release.id == release_id)
            .map(|release| {
                (
                    release.release_type,
                    release.copies_pressed,
                    release.copies_sold,
                )
            })
            .ok_or_else(|| format!("Release with ID {} not found.", release_id))?;

        if copies_pressed == 0 {
            return Err(
                "That release was never pressed to a finite run — there's nothing to top up."
                    .to_string(),
            );
        }
        let sold_ratio = copies_sold as f32 / copies_pressed as f32;
        if sold_ratio < constants::REPRESS_LOW_STOCK_SOLD_RATIO {
            return Err(format!(
                "Still {} copies on the shelves — not worth a fresh run yet.",
                copies_pressed.saturating_sub(copies_sold)
            ));
        }

        let (run, cost) = self.plan_pressing(&release_type, pressing)?;
        if !self.player.can_afford(cost) {
            return Err(format!(
                "A fresh pressing run costs ${} — you can't afford it.",
                cost
            ));
        }
        self.player.spend_money(cost);

        let release = self
            .band
            .singles_released
            .iter_mut()
            .chain(self.band.albums_released.iter_mut())
            .find(|release| release.id == release_id)
            .expect("existence just checked above");
        release.copies_pressed = release.copies_pressed.saturating_add(run);
        let release_name = release.name.clone();
        self.log(format!(
            "🏭 Re-pressed '{}' — {} more copies for ${}.",
            release_name, run, cost
        ));
        Ok(())
    }

    pub(in crate::game) fn action_start_marketing_campaign(
        &mut self,
        release_id: u32,
        campaign_type: MarketingCampaignType,
    ) -> Result<(), String> {
        if let Some(deal) = self.band.current_deal() {
            return Err(format!(
                "Promotion is {}'s job — their people are already on it.",
                deal.label_name
            ));
        }
        let spec = campaign_type.spec();
        if !self.player.can_afford(spec.cost) {
            return Err(format!(
                "Not enough money for a {} campaign. Need ${}.",
                spec.name, spec.cost
            ));
        }

        let current_week = self.week;
        // Find in just_released_music first, then in already released music
        let release = self
            .just_released_music
            .iter_mut()
            .find(|r| r.id == release_id)
            .or_else(|| {
                self.band
                    .singles_released
                    .iter_mut()
                    .find(|r| r.id == release_id)
            })
            .or_else(|| {
                self.band
                    .albums_released
                    .iter_mut()
                    .find(|r| r.id == release_id)
            })
            .ok_or_else(|| {
                format!(
                    "Release with ID {} not found to start marketing campaign.",
                    release_id
                )
            })?;

        release.active_marketing.push(ActiveMarketingCampaign {
            campaign_type,
            start_week: current_week,
            end_week: current_week + spec.duration_weeks,
            effectiveness_bonus: spec.effectiveness_bonus,
        });

        release.marketing_level_achieved = release
            .active_marketing
            .iter()
            .map(|c| c.effectiveness_bonus as u32)
            .sum::<u32>()
            .min(100) as u8;
        let release_name = release.name.clone();

        self.player.spend_money(spec.cost);
        self.log(format!(
            "📣 {} campaign launched for '{}' — ${}, runs {} weeks, +{} buzz.",
            spec.name, release_name, spec.cost, spec.duration_weeks, spec.effectiveness_bonus
        ));
        Ok(())
    }
    pub(in crate::game) fn action_accept_deal(&mut self, offer_index: usize) -> Result<(), String> {
        if offer_index >= self.pending_deal_offers.len() {
            return Err("Invalid deal offer selected.".to_string());
        }
        let offer = self.pending_deal_offers.remove(offer_index);
        let label_name = offer.label_name.clone();
        let advance = offer.advance;
        let albums_required = offer.albums_required;
        let term_weeks = offer.term_weeks;
        let signed_week = self.week;
        // L-ENTITY (§A): link the deal to its `Label` world entity so the
        // weekly wholesale ledger books its margin to the right balance sheet.
        // Label names are unique across the seed data, so a name match is a
        // stable resolve; `None` only when labels aren't seeded (a pre-0.8
        // save), in which case the ledger simply books nothing.
        let label_id = self
            .world
            .labels
            .iter()
            .find(|(_, label)| label.name == label_name)
            .map(|(id, _)| *id);
        // M9: a renewal-window EXTENSION carries the old deal's unrecouped
        // balance forward. The offer snapshotted that balance when it was
        // generated, but the offer can sit pending for weeks while the catalog
        // tail keeps paying the ledger down — so re-read the *live* balance now
        // rather than carrying the stale (always higher) snapshot, which would
        // start the new deal deeper in the red than the band actually owes.
        // The old deal is still `current_deal()` here (before `sign_deal`
        // replaces it). `carry_forward_unrecouped == 0` marks an ordinary
        // signing or NEW CONTRACT renewal — those carry nothing.
        let carry_forward_unrecouped = if offer.carry_forward_unrecouped > 0 {
            self.band
                .current_deal()
                .map(|deal| deal.unrecouped.max(0))
                .unwrap_or(0)
        } else {
            0
        };
        let new_deal = band::RecordDeal {
            label_name: offer.label_name,
            label_tier: offer.label_tier,
            advance: offer.advance,
            royalty_rate: offer.royalty_rate,
            albums_required: offer.albums_required,
            albums_delivered: 0,
            market_reach: offer.original_label_data.market_reach,
            // M5 (§E-2): the advance is not a gift. The player banks it now
            // (below), but the same amount joins the recoupment ledger — every
            // royalty dollar pays it back before the band sees a cent, on top
            // of any renewal carry-forward computed above.
            unrecouped: advance as i32 + carry_forward_unrecouped,
            // M9 (design §E-4): the term's clock starts now. A pre-M9
            // pending offer deserializes with `term_weeks: 0` and signs a
            // legacy-policy deal (see `RecordDeal::term_weeks`).
            signed_week,
            term_weeks,
            // L-ENTITY (§A): resolved above; the ledger books to this id.
            label_id,
        };
        self.band.sign_deal(new_deal);
        self.player.earn_money(advance);
        // L-ENTITY (§A): the advance leaves the label's balance sheet — the
        // same event that seeds the deal's `unrecouped` ledger now also moves
        // the label's real money (design: "every dollar a label spends leaves
        // its capital"). The player's advance/recoupment is unchanged; this
        // only debits the label's side.
        if let Some(id) = label_id
            && let Some(label) = self.world.labels.get_mut(&id)
        {
            label.book(-(advance as i64));
        }
        self.pending_deal_offers.clear();
        self.log(format!(
            "✍️ Signed with {}! ${} advance in the bank — you owe them {} album{} over {} weeks.",
            label_name,
            advance,
            albums_required,
            if albums_required == 1 { "" } else { "s" },
            term_weeks
        ));
        Ok(())
    }

    pub(in crate::game) fn action_reject_deal(
        &mut self,
        offer_index: usize,
        rng: &mut impl Rng,
    ) -> Result<(), String> {
        if offer_index >= self.pending_deal_offers.len() {
            return Err("Invalid deal offer selected.".to_string());
        }
        let offer = self.pending_deal_offers.remove(offer_index);
        self.log(format!("🚫 Turned down {}'s offer.", offer.label_name));

        if let Some(poaching_band) = self.world.poach_rejected_deal(&offer.label_name, rng) {
            self.log(format!(
                "📰 NEWS: {} signed with {} after you turned them down!",
                poaching_band, offer.label_name
            ));
        }
        Ok(())
    }

    /// Take the support slot (design §F). The guarantee is banked on the
    /// spot — that's what a guarantee is — and the run itself becomes an
    /// `OngoingActivity` whose weeks the calendar actually plays. This
    /// method never touches `Game::week`.
    pub(in crate::game) fn action_accept_support_tour(&mut self) -> Result<(), String> {
        if let Some(activity) = &self.ongoing_activity {
            return Err(format!(
                "You're already committed — {} ({}).",
                activity.headline(),
                activity.progress()
            ));
        }
        let Some(offer) = self.pending_support_offer.clone() else {
            return Err("Nobody has offered you a support slot.".to_string());
        };
        if self.player.stress >= TOUR_STRESS_GUARD {
            return Err("You're too stressed to head out on the road!".to_string());
        }
        if self.player.health < TOUR_HEALTH_GUARD {
            return Err("You're too unwell to head out on the road!".to_string());
        }
        self.pending_support_offer = None;

        self.player.earn_money(offer.pay);

        let support = SupportTourInProgress {
            host_band: offer.host_band.clone(),
            weeks_total: offer.weeks as u8,
            weeks_done: 0,
            start_week: self.week,
            pay: offer.pay,
            fame_gain: offer.fame_gain,
        };
        self.log(format!(
            "🎟️ Took {}'s support slot — {} weeks opening, ${} guaranteed.",
            offer.host_band, offer.weeks, offer.pay
        ));

        // The first week plays now; `turn.rs` drives the rest.
        self.resolve_support_tour_week(support);
        Ok(())
    }

    /// One week on someone else's stage (design §F): a support run is
    /// touring, so it charges the same weekly stress and wear a headline
    /// tour does — now the week it is actually endured.
    pub(in crate::game) fn resolve_support_tour_week(
        &mut self,
        mut support: SupportTourInProgress,
    ) {
        self.player.stress =
            (self.player.stress + constants::TOUR_STRESS_COST_PER_WEEK).min(constants::MAX_STRESS);
        self.player.health = self
            .player
            .health
            .saturating_sub(constants::TOUR_HEALTH_COST_PER_WEEK);

        support.weeks_done += 1;
        if support.weeks_done >= support.weeks_total {
            self.settle_support_tour(support, true);
            return;
        }

        self.log(format!(
            "🎟️ Opening for {}, week {} of {}.",
            support.host_band, support.weeks_done, support.weeks_total
        ));
        self.ongoing_activity = Some(OngoingActivity::SupportTour(support));
    }

    /// Close out a support run (design §F). Finishing it is what buys the
    /// exposure: walking out mid-run keeps the guarantee already banked but
    /// none of the fame the slot was worth.
    pub(in crate::game) fn settle_support_tour(
        &mut self,
        support: SupportTourInProgress,
        completed: bool,
    ) {
        self.ongoing_activity = None;

        if !completed {
            self.log(format!(
                "🛑 Left {}'s tour after {} of {} weeks — the exposure went with it.",
                support.host_band, support.weeks_done, support.weeks_total
            ));
            return;
        }

        // Report the fame actually applied (comeback doubling, caps), not
        // the offer's raw number.
        let fame_applied = self.band.gain_fame(support.fame_gain);
        self.log(format!(
            "🎟️ Opened for {} for {} weeks — ${} and a taste of the big stage (fame +{}).",
            support.host_band, support.weeks_total, support.pay, fame_applied
        ));

        // The encore roll settles on the activity stream, keyed off the week
        // the run started, so it can't be shifted by what the world drew
        // while the band was away (`rng.rs`).
        let mut rng = self.activity_rng(support.start_week, 0);
        if rng.gen_bool(0.25) {
            let encore_fame = self.band.gain_fame(2);
            self.log(format!(
                "🔥 Their crowd adopted you — encores every night (+{} fame).",
                encore_fame
            ));
        }
    }

    pub(in crate::game) fn action_decline_support_tour(&mut self) -> Result<(), String> {
        let Some(offer) = self.pending_support_offer.take() else {
            return Err("Nobody has offered you a support slot.".to_string());
        };
        self.log(format!("🚫 Passed on {}'s support slot.", offer.host_band));
        Ok(())
    }
}
