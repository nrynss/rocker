//! Record-deal scouting, buzz, scene poaching of rejected offers, and (M9)
//! the term a deal carries at signing plus the recoupment-dependent
//! renewal window that opens before a healthy term's expiry (design §E-4).

use crate::data_loader::{GameDataFiles, RecordLabel, RecordLabelsData};
use crate::game::band::Band;
use crate::game::constants::{
    DEAL_EXTENSION_ADVANCE_FRACTION, DEAL_EXTENSION_ALBUMS, DEAL_EXTENSION_TERM_WEEKS,
    DEAL_NEW_CONTRACT_ROYALTY_BUMP_MAX, DEAL_NEW_CONTRACT_ROYALTY_BUMP_MIN,
    DEAL_RENEWAL_DECENT_SALES_MIN_COMMERCIAL_SUCCESS, DEAL_RENEWAL_DEEP_RED_UNRECOUPED,
    DEAL_RENEWAL_WEAK_SALES_MAX_COMMERCIAL_SUCCESS, DEAL_RENEWAL_WINDOW_WEEKS,
    DEAL_TERM_BOUTIQUE_WEEKS, DEAL_TERM_INDEPENDENT_WEEKS, DEAL_TERM_MAJOR_WEEKS,
};
use rand::Rng;
use serde::{Deserialize, Serialize};

use super::GameWorld;

// Deal-offer buzz: the scale the tier thresholds in record_labels.json
// measure (independent 10, boutique 20, major 30). Fame carries most of it;
// records prove you can deliver; a charting record adds short-lived heat.
pub(crate) const BUZZ_PER_SINGLE: u32 = 3;
pub(crate) const BUZZ_PER_ALBUM: u32 = 4;
pub(crate) const BUZZ_CATALOG_CAP: u32 = 10;
pub(crate) const BUZZ_CHART_BONUS: u32 = 2;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PotentialDealOffer {
    pub label_name: String,
    pub label_tier: String,
    pub advance: u32,
    pub royalty_rate: f32,
    pub albums_required: u8,
    pub original_label_data: RecordLabel,
    /// The week the label withdraws the offer if it's ignored. `None`
    /// means it never expires — deliberately, because offers saved before
    /// expiry existed deserialize to `None`, and a bare numeric default
    /// (0) would kill every live offer the moment an old save loaded.
    #[serde(default)]
    pub expires_week: Option<u32>,
    /// Contract length in weeks (design §E-4), generated here at offer
    /// time. `0` on a pre-M9 save's still-pending offer — accepting it
    /// then signs a legacy-policy deal (see `RecordDeal::term_weeks`).
    #[serde(default)]
    pub term_weeks: u16,
    /// Set only on a renewal-window EXTENSION offer (design §E-4): the old
    /// deal's `unrecouped` balance, carried into the new deal's ledger on
    /// top of this offer's own (small) advance instead of starting fresh.
    /// `0` for every ordinary signing and every NEW CONTRACT renewal.
    #[serde(default)]
    pub carry_forward_unrecouped: i32,
}

impl GameWorld {
    pub fn poach_rejected_deal(&mut self, label_name: &str, rng: &mut impl Rng) -> Option<String> {
        if !rng.gen_bool(0.6) {
            return None;
        }
        let idx = self
            .bands
            .iter()
            .enumerate()
            .filter(|(_, b)| b.label.is_none() && b.fame >= 20)
            .max_by_key(|(_, b)| b.fame)
            .map(|(i, _)| i)?;
        let band = &mut self.bands[idx];
        band.label = Some(label_name.to_string());
        band.momentum = (band.momentum + 1).min(3);
        Some(band.name.clone())
    }
    pub(super) fn band_buzz(&self, band: &Band) -> u8 {
        let fame_heat = u32::from(band.fame) * 3 / 10;
        let catalog_heat = (band.singles_released.len() as u32 * BUZZ_PER_SINGLE
            + band.albums_released.len() as u32 * BUZZ_PER_ALBUM)
            .min(BUZZ_CATALOG_CAP);
        // (M3 note for M9: this used to read the legacy flat `self.charts`
        // field; that field is now vestigial (design §C — regional Top
        // 100s), so the check moved to the regional boards via
        // `player_is_charting`.)
        let chart_heat = if self.player_is_charting() {
            BUZZ_CHART_BONUS
        } else {
            0
        };
        (fame_heat + catalog_heat + chart_heat).min(100) as u8
    }

    pub fn generate_deal_offers(
        &self,
        band: &Band,
        game_data: &GameDataFiles,
        rng: &mut impl Rng,
    ) -> Vec<PotentialDealOffer> {
        // M9 (design §E-4): a breach's cooldown blocks every new offer.
        // This is the single choke point `check_and_generate_deal_offers`
        // (turn.rs) calls through for the player, so gating here — rather
        // than at that call site — is enough.
        if band.deal_cooldown > 0 {
            return Vec::new();
        }
        let mut offers = Vec::new();
        let labels_data = game_data.get_record_labels_data();
        let buzz = self.band_buzz(band);

        let label_tiers = [
            (
                "Major",
                &labels_data.major_labels,
                &labels_data
                    .label_requirements
                    .major_label_interest_threshold,
            ),
            (
                "Independent",
                &labels_data.independent_labels,
                &labels_data
                    .label_requirements
                    .independent_label_interest_threshold,
            ),
            (
                "Boutique",
                &labels_data.boutique_labels,
                &labels_data
                    .label_requirements
                    .boutique_label_interest_threshold,
            ),
        ];

        for (tier_name, labels_in_tier, threshold) in &label_tiers {
            for label in *labels_in_tier {
                // The `singles` column reads as "records out, of any kind":
                // an album on the shelf opens doors at least as well as a
                // 45, so an act that went straight to albums isn't
                // invisible to every A&R desk in town.
                if band.fame >= threshold.fame
                    && band.albums_released.len() >= threshold.albums as usize
                    && band.total_releases() >= threshold.singles as usize
                    && buzz >= threshold.buzz
                {
                    // Check if already signed with this label
                    if let Some(current_deal) = band.current_deal()
                        && current_deal.label_name == label.name
                    {
                        continue; // Already signed with this label
                    }

                    // Random chance to make an offer
                    let offer_chance = match *tier_name {
                        "Major" => {
                            if band.fame > 70 {
                                0.30
                            } else if band.fame > 50 {
                                0.20
                            } else {
                                0.10
                            }
                        }
                        "Independent" => {
                            if band.fame > 40 {
                                0.40
                            } else if band.fame > 20 {
                                0.25
                            } else {
                                0.15
                            }
                        }
                        "Boutique" => {
                            if band.fame > 10 {
                                0.50
                            } else {
                                0.20
                            }
                        }
                        _ => 0.10,
                    };

                    if rng.gen_bool(offer_chance) {
                        let advance_percentage = match band.fame {
                            0..=20 => rng.gen_range(0.0..0.4), // Lower end for low fame
                            21..=50 => rng.gen_range(0.3..0.7),
                            51..=100 => rng.gen_range(0.6..1.0), // Higher end for high fame
                            _ => 0.5,
                        };
                        let advance_range_span = label.advance_range[1] - label.advance_range[0];
                        let calculated_advance = label.advance_range[0]
                            + (advance_range_span as f32 * advance_percentage) as u32;

                        let advance = calculated_advance
                            .clamp(label.advance_range[0], label.advance_range[1]);

                        let royalty_rate = label.royalty_rate as f32 / 100.0;

                        let albums_required = match *tier_name {
                            "Major" => rng.gen_range(2..=4),
                            "Independent" => rng.gen_range(1..=3),
                            "Boutique" => rng.gen_range(1..=2),
                            _ => 2,
                        };
                        let term_weeks = term_weeks_for_tier(tier_name, rng);

                        offers.push(PotentialDealOffer {
                            label_name: label.name.clone(),
                            label_tier: tier_name.to_string(),
                            advance,
                            royalty_rate,
                            albums_required,
                            original_label_data: label.clone(),
                            // The world has no clock; the game stamps the
                            // deadline when the offer lands on the table.
                            expires_week: None,
                            term_weeks,
                            carry_forward_unrecouped: 0,
                        });
                    }
                }
            }
        }
        offers
    }

    /// The renewal window (design §E-4): 26 weeks [tune] before a healthy
    /// term's expiry, with all albums delivered, the label looks at its
    /// ledger and makes a move through the same offer stream a fresh
    /// signing uses. `None` when the window isn't open, or when the ledger
    /// says "deep in the red with weak sales" (the label just lets the
    /// clock run out).
    pub fn generate_renewal_offer(
        &self,
        band: &Band,
        game_data: &GameDataFiles,
        rng: &mut impl Rng,
        current_week: u32,
    ) -> Option<PotentialDealOffer> {
        let deal = band.current_deal()?;
        if !deal.renewal_window_open(current_week, DEAL_RENEWAL_WINDOW_WEEKS) {
            return None;
        }
        let labels_data = game_data.get_record_labels_data();
        let label_data = find_label_by_name(labels_data, &deal.label_tier, &deal.label_name)?;

        match renewal_decision(deal.unrecouped, band.reputation.commercial_success) {
            RenewalDecision::Silence => None,
            RenewalDecision::Extension => {
                let advance = (deal.advance as f32 * DEAL_EXTENSION_ADVANCE_FRACTION) as u32;
                Some(PotentialDealOffer {
                    label_name: deal.label_name.clone(),
                    label_tier: deal.label_tier.clone(),
                    advance,
                    royalty_rate: deal.royalty_rate,
                    albums_required: DEAL_EXTENSION_ALBUMS,
                    original_label_data: label_data.clone(),
                    expires_week: None,
                    term_weeks: DEAL_EXTENSION_TERM_WEEKS,
                    // The label protects its investment: the balance it's
                    // still owed carries into the new deal's ledger.
                    carry_forward_unrecouped: deal.unrecouped,
                })
            }
            RenewalDecision::NewContract => {
                let bump = rng.gen_range(
                    DEAL_NEW_CONTRACT_ROYALTY_BUMP_MIN..=DEAL_NEW_CONTRACT_ROYALTY_BUMP_MAX,
                );
                let royalty_rate = (deal.royalty_rate + bump).min(1.0);
                let advance_percentage = match band.fame {
                    0..=20 => rng.gen_range(0.0..0.4),
                    21..=50 => rng.gen_range(0.3..0.7),
                    51..=100 => rng.gen_range(0.6..1.0),
                    _ => 0.5,
                };
                let advance_range_span = label_data.advance_range[1] - label_data.advance_range[0];
                let advance = (label_data.advance_range[0]
                    + (advance_range_span as f32 * advance_percentage) as u32)
                    .clamp(label_data.advance_range[0], label_data.advance_range[1]);
                let term_weeks = term_weeks_for_tier(&deal.label_tier, rng);
                let albums_required = match deal.label_tier.as_str() {
                    "Major" => rng.gen_range(2..=4),
                    "Independent" => rng.gen_range(1..=3),
                    "Boutique" => rng.gen_range(1..=2),
                    _ => 2,
                };
                Some(PotentialDealOffer {
                    label_name: deal.label_name.clone(),
                    label_tier: deal.label_tier.clone(),
                    advance,
                    royalty_rate,
                    albums_required,
                    original_label_data: label_data.clone(),
                    expires_week: None,
                    term_weeks,
                    // Recouped — a fresh ledger starts clean off the new
                    // advance alone.
                    carry_forward_unrecouped: 0,
                })
            }
        }
    }
}

/// Contract term at signing, by tier (design §E-4 table) — shared between a
/// fresh signing and a renewal-window NEW CONTRACT.
fn term_weeks_for_tier(tier_name: &str, rng: &mut impl Rng) -> u16 {
    let (min, max) = match tier_name {
        "Major" => DEAL_TERM_MAJOR_WEEKS,
        "Independent" => DEAL_TERM_INDEPENDENT_WEEKS,
        "Boutique" => DEAL_TERM_BOUTIQUE_WEEKS,
        _ => DEAL_TERM_INDEPENDENT_WEEKS,
    };
    rng.gen_range(min..=max)
}

/// Look up a label's live data by name within its own tier — the renewal
/// window comes from the same label the band is already signed to.
fn find_label_by_name<'a>(
    labels_data: &'a RecordLabelsData,
    label_tier: &str,
    label_name: &str,
) -> Option<&'a RecordLabel> {
    let list = match label_tier {
        "Major" => &labels_data.major_labels,
        "Independent" => &labels_data.independent_labels,
        "Boutique" => &labels_data.boutique_labels,
        _ => return None,
    };
    list.iter().find(|l| l.name == label_name)
}

/// The three-way renewal-window ledger read (design §E-4). Recouped with
/// decent sales earns a new contract; deep in the red with weak sales gets
/// silence; everything else — chiefly "not yet recouped" — gets an
/// extension, the label protecting its investment rather than rewarding
/// the band.
enum RenewalDecision {
    NewContract,
    Extension,
    Silence,
}

fn renewal_decision(unrecouped: i32, commercial_success: u8) -> RenewalDecision {
    if unrecouped <= 0 && commercial_success >= DEAL_RENEWAL_DECENT_SALES_MIN_COMMERCIAL_SUCCESS {
        RenewalDecision::NewContract
    } else if unrecouped >= DEAL_RENEWAL_DEEP_RED_UNRECOUPED
        && commercial_success <= DEAL_RENEWAL_WEAK_SALES_MAX_COMMERCIAL_SUCCESS
    {
        RenewalDecision::Silence
    } else {
        RenewalDecision::Extension
    }
}
