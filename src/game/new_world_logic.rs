use crate::data_loader::{GameDataFiles, RecordLabel};
use crate::game::band::Band;
use crate::game::timeline::MusicTimeline;
use rand::{thread_rng, Rng};
use serde::{Deserialize, Serialize};
use std::collections::HashMap; // Ensure HashMap is imported

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PotentialDealOffer {
    pub label_name: String,
    pub label_tier: String,
    pub advance: u32,
    pub royalty_rate: f32,
    pub albums_required: u8,
    pub original_label_data: RecordLabel,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GameWorld {
    pub music_market: MusicMarket,
    pub competing_bands: Vec<CompetingBand>,
    pub venues: Vec<Venue>,
    pub current_trends: MusicTrend,
    pub dynamic_genre_modifiers: HashMap<MusicGenre, f32>, // Action 2.1: Ensure this line is present
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MusicMarket {
    pub demand: u8,
    pub saturation: u8,
    pub economic_state: EconomicState,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum EconomicState {
    Recession,
    Stagnant,
    Growing,
    Booming,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompetingBand {
    pub name: String,
    pub fame: u8,
    pub latest_release: String,
    pub genre: MusicGenre,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Venue {
    pub name: String,
    pub capacity: u32,
    pub prestige: u8,
    pub base_payment: u32,
    pub location: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum MusicTrend {
    Rock,
    Pop,
    Metal,
    Punk,
    Alternative,
    Electronic,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)] // Action 2.2: Ensure all derives are present
pub enum MusicGenre {
    Rock,
    Pop,
    Metal,
    Punk,
    Alternative,
    Electronic,
    Folk,
    Jazz,
}

impl GameWorld {
    pub fn new(data_files: &GameDataFiles) -> Self {
        Self {
            music_market: MusicMarket {
                demand: 50,
                saturation: 30,
                economic_state: EconomicState::Growing,
            },
            competing_bands: Self::generate_competing_bands(data_files),
            venues: Self::generate_venues(data_files),
            current_trends: MusicTrend::Rock,
            dynamic_genre_modifiers: HashMap::new(), // Action 2.1: Ensure initialized
        }
    }

    pub fn update_week(&mut self, timeline: &MusicTimeline) {
        let mut rng = thread_rng();
        self.update_market_with_timeline(&mut rng, timeline);
        self.update_competing_bands(&mut rng);
        self.update_trends_with_timeline(timeline);

        let mut new_modifiers = HashMap::new();
        for (genre, val) in self.dynamic_genre_modifiers.iter_mut() { // Should be iter() if just reading, or handle mutation carefully
            let decayed_val = (*val - 1.0) * 0.95 + 1.0;
            if (decayed_val - 1.0).abs() > 0.01 {
                 new_modifiers.insert(genre.clone(), decayed_val);
            }
        }
        self.dynamic_genre_modifiers = new_modifiers;
    }

    fn update_market_with_timeline(&mut self, rng: &mut impl Rng, timeline: &MusicTimeline) {
        let era = timeline.get_current_era();
        let target_demand = era.market_conditions.overall_demand;
        if self.music_market.demand < target_demand {
            self.music_market.demand = (self.music_market.demand + 2).min(target_demand);
        } else if self.music_market.demand > target_demand {
            self.music_market.demand = (self.music_market.demand.saturating_sub(2)).max(target_demand);
        }
        if rng.gen_range(0..10) == 0 {
            self.music_market.economic_state = match era.market_conditions.record_sales_growth {
                x if x > 20.0 => EconomicState::Booming,
                x if x > 10.0 => EconomicState::Growing,
                x if x > 0.0 => EconomicState::Stagnant,
                _ => EconomicState::Recession,
            };
        }
        if era.market_conditions.innovation_openness > 80 {
            self.music_market.saturation = self.music_market.saturation.saturating_sub(1);
        } else {
            self.music_market.saturation = (self.music_market.saturation + 1).min(95);
        }
    }

    fn update_trends_with_timeline(&mut self, timeline: &MusicTimeline) {
        let trending_genres = timeline.get_trending_genres();
        if !trending_genres.is_empty() {
            let mut rng = thread_rng();
            if rng.gen_range(0..10) == 0 {
                self.current_trends = match trending_genres[0].as_str() {
                    "Rock" => MusicTrend::Rock,
                    "Pop" => MusicTrend::Pop,
                    "Metal" | "Hard Rock" | "Hair Metal" => MusicTrend::Metal,
                    "Punk" | "Post-Punk" => MusicTrend::Punk,
                    "Alternative" | "Grunge" | "New Wave" => MusicTrend::Alternative,
                    _ => MusicTrend::Electronic,
                };
            }
        }
    }

    fn update_competing_bands(&mut self, rng: &mut impl Rng) {
        for band in &mut self.competing_bands {
            if rng.gen_range(0..8) == 0 {
                let fame_change = rng.gen_range(-3..=3);
                band.fame = (band.fame as i8 + fame_change).clamp(0, 100) as u8;
            }
            if rng.gen_range(0..12) == 0 {
                band.latest_release = Self::generate_song_title_simple(rng);
            }
        }
    }

    fn generate_competing_bands(data_files: &GameDataFiles) -> Vec<CompetingBand> {
        let mut rng = thread_rng();
        let mut bands = Vec::new();
        for _ in 0..8 {
            bands.push(CompetingBand {
                name: data_files.random_band_name(),
                fame: rng.gen_range(10..60),
                latest_release: data_files.random_song_title(),
                genre: match rng.gen_range(0..6) {
                    0 => MusicGenre::Rock, 1 => MusicGenre::Metal, 2 => MusicGenre::Punk,
                    3 => MusicGenre::Alternative, 4 => MusicGenre::Pop, _ => MusicGenre::Electronic,
                },
            });
        }
        bands
    }

    fn generate_venues(data_files: &GameDataFiles) -> Vec<Venue> {
        let mut venues = Vec::new();
        let cities = vec!["Downtown", "City Center", "Industrial District", "Uptown", "Sports Complex"];
        let capacities = [50, 200, 500, 2000, 20000];
        let prestiges = [10, 25, 40, 70, 95];
        let payments = [100, 300, 800, 3000, 15000];
        for i in 0..5 {
            venues.push(Venue {
                name: data_files.random_venue_name(),
                capacity: capacities[i], prestige: prestiges[i], base_payment: payments[i],
                location: format!("{}, {}", cities[i], data_files.random_city()),
            });
        }
        venues
    }

    fn generate_song_title_simple(rng: &mut impl Rng) -> String {
        let adjectives = ["Broken", "Electric", "Dark", "Wild", "Lost", "Burning"];
        let nouns = ["Dreams", "Hearts", "Nights", "Roads", "Wings", "Thunder"];
        format!("{} {}", adjectives[rng.gen_range(0..adjectives.len())], nouns[rng.gen_range(0..nouns.len())])
    }

    pub fn get_available_venues_for_fame(&self, fame: u8) -> Vec<&Venue> {
        self.venues.iter().filter(|venue| venue.prestige <= fame + 20).collect()
    }

    pub fn get_market_modifier(&self) -> f32 {
        let demand_mod = self.music_market.demand as f32 / 100.0;
        let saturation_penalty = 1.0 - (self.music_market.saturation as f32 / 200.0);
        let economic_mod = match self.music_market.economic_state {
            EconomicState::Recession => 0.7, EconomicState::Stagnant => 0.9,
            EconomicState::Growing => 1.1, EconomicState::Booming => 1.3,
        };
        demand_mod * saturation_penalty * economic_mod
    }

    pub fn generate_deal_offers(&self, band: &Band, game_data: &GameDataFiles, rng: &mut impl Rng) -> Vec<PotentialDealOffer> {
        let mut offers = Vec::new();
        let labels_data = game_data.get_record_labels_data();
        let label_tiers = [
            ("Major", &labels_data.major_labels, &labels_data.label_requirements.major_label_interest_threshold),
            ("Independent", &labels_data.independent_labels, &labels_data.label_requirements.independent_label_interest_threshold),
            ("Boutique", &labels_data.boutique_labels, &labels_data.label_requirements.boutique_label_interest_threshold),
        ];

        for (tier_name, labels_in_tier, threshold) in &label_tiers {
            for label in *labels_in_tier {
                let buzz_placeholder = band.fame / 5;
                // Action 2.3: Correct field access for band's releases
                if band.fame >= threshold.fame &&
                   band.albums_released.len() as u8 >= threshold.albums &&
                   band.singles_released.len() as u8 >= threshold.singles &&
                   buzz_placeholder >= threshold.buzz {
                    if let Some(current_deal) = band.current_deal() {
                        if current_deal.label_name == label.name { continue; }
                    }
                    let offer_chance = match *tier_name {
                        "Major" => if band.fame > 70 { 0.30 } else if band.fame > 50 { 0.20 } else { 0.10 },
                        "Independent" => if band.fame > 40 { 0.40 } else if band.fame > 20 { 0.25 } else { 0.15 },
                        "Boutique" => if band.fame > 10 { 0.50 } else { 0.20 },
                        _ => 0.10,
                    };
                    if rng.gen_bool(offer_chance) {
                        let advance_percentage = match band.fame {
                            0..=20 => rng.gen_range(0.0..0.4), 21..=50 => rng.gen_range(0.3..0.7),
                            51..=100 => rng.gen_range(0.6..1.0), _ => 0.5,
                        };
                        let advance_range_span = label.advance_range[1] - label.advance_range[0];
                        let calculated_advance = label.advance_range[0] + (advance_range_span as f32 * advance_percentage) as u32;
                        let advance = calculated_advance.clamp(label.advance_range[0], label.advance_range[1]);
                        let royalty_rate = label.royalty_rate as f32 / 100.0;
                        let albums_required = match *tier_name {
                            "Major" => rng.gen_range(2..=4), "Independent" => rng.gen_range(1..=3),
                            "Boutique" => rng.gen_range(1..=2), _ => 2,
                        };
                        offers.push(PotentialDealOffer {
                            label_name: label.name.clone(), label_tier: tier_name.to_string(),
                            advance, royalty_rate, albums_required,
                            original_label_data: label.clone(),
                        });
                    }
                }
            }
        }
        offers
    }
}

impl std::fmt::Display for MusicTrend {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            MusicTrend::Rock => write!(f, "Rock"), MusicTrend::Pop => write!(f, "Pop"),
            MusicTrend::Metal => write!(f, "Metal"), MusicTrend::Punk => write!(f, "Punk"),
            MusicTrend::Alternative => write!(f, "Alternative"), MusicTrend::Electronic => write!(f, "Electronic"),
        }
    }
}

impl std::fmt::Display for EconomicState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            EconomicState::Recession => write!(f, "Recession"), EconomicState::Stagnant => write!(f, "Stagnant"),
            EconomicState::Growing => write!(f, "Growing"), EconomicState::Booming => write!(f, "Booming"),
        }
    }
}
