//! Player weekly actions (split by concern). Methods remain on `Game`.

use crate::game::music::{ActiveMarketingCampaign, MarketingCampaignType};
use rand::Rng;

use super::super::constants;
use super::super::*;

impl Game {
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
            // royalty dollar pays it back before the band sees a cent.
            unrecouped: advance as i32,
        };
        self.band.sign_deal(new_deal);
        self.player.earn_money(advance);
        self.pending_deal_offers.clear();
        self.log(format!(
            "✍️ Signed with {}! ${} advance in the bank — you owe them {} album{}.",
            label_name,
            advance,
            albums_required,
            if albums_required == 1 { "" } else { "s" }
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

    pub(in crate::game) fn action_accept_support_tour(
        &mut self,
        rng: &mut impl Rng,
    ) -> Result<(), String> {
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
        // A support run is touring: same weekly stress and wear as a headline tour.
        let weeks = offer.weeks as u8;
        self.player.stress = (self.player.stress
            + constants::TOUR_STRESS_COST_PER_WEEK.saturating_mul(weeks))
        .min(constants::MAX_STRESS);
        self.player.health = self
            .player
            .health
            .saturating_sub(constants::TOUR_HEALTH_COST_PER_WEEK.saturating_mul(weeks));
        self.band.gain_fame(offer.fame_gain);
        self.week += offer.weeks;
        self.log(format!(
            "🎟️ Opened for {} for {} weeks — ${} and a taste of the big stage (fame +{}).",
            offer.host_band, offer.weeks, offer.pay, offer.fame_gain
        ));

        if rng.gen_bool(0.25) {
            self.band.gain_fame(2);
            self.log("🔥 Their crowd adopted you — encores every night (+2 fame).");
        }
        Ok(())
    }

    pub(in crate::game) fn action_decline_support_tour(&mut self) -> Result<(), String> {
        let Some(offer) = self.pending_support_offer.take() else {
            return Err("Nobody has offered you a support slot.".to_string());
        };
        self.log(format!("🚫 Passed on {}'s support slot.", offer.host_band));
        Ok(())
    }
}
