//! Record-deal scouting, buzz, and scene poaching of rejected offers.

use crate::data_loader::{GameDataFiles, RecordLabel};
use crate::game::band::Band;
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
        let chart_heat = if self.charts.iter().any(|entry| entry.is_player) {
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
                        });
                    }
                }
            }
        }
        offers
    }
}
