use crate::data_loader::GameDataFiles;
use crate::game::timeline::MusicTimeline;
use rand::{thread_rng, Rng};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GameWorld {
    pub music_market: MusicMarket,
    pub competing_bands: Vec<CompetingBand>,
    pub venues: Vec<Venue>,
    pub current_trends: MusicTrend,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MusicMarket {
    pub demand: u8,     // 0-100, affects earnings
    pub saturation: u8, // 0-100, affects difficulty
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
    pub prestige: u8, // 0-100
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

#[derive(Debug, Clone, Serialize, Deserialize)]
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
        }
    }

    pub fn update_week(&mut self, timeline: &MusicTimeline) {
        let mut rng = thread_rng();

        // Update market conditions based on historical era
        self.update_market_with_timeline(&mut rng, timeline);

        // Update competing bands
        self.update_competing_bands(&mut rng);

        // Update trends based on timeline
        self.update_trends_with_timeline(timeline);
    }

    fn update_market_with_timeline(&mut self, rng: &mut impl Rng, timeline: &MusicTimeline) {
        let era = timeline.get_current_era();

        // Adjust market demand towards historical era demand
        let target_demand = era.market_conditions.overall_demand;
        if self.music_market.demand < target_demand {
            self.music_market.demand = (self.music_market.demand + 2).min(target_demand);
        } else if self.music_market.demand > target_demand {
            self.music_market.demand =
                (self.music_market.demand.saturating_sub(2)).max(target_demand);
        }

        // Update economic state based on era and some randomness
        if rng.gen_range(0..10) == 0 {
            self.music_market.economic_state = match era.market_conditions.record_sales_growth {
                x if x > 20.0 => EconomicState::Booming,
                x if x > 10.0 => EconomicState::Growing,
                x if x > 0.0 => EconomicState::Stagnant,
                _ => EconomicState::Recession,
            };
        }

        // Saturation grows over time but can decrease with innovation
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
                // 10% chance to align with historical trends
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
            // Bands occasionally gain or lose fame
            if rng.gen_range(0..8) == 0 {
                let fame_change = rng.gen_range(-3..=3);
                band.fame = (band.fame as i8 + fame_change).clamp(0, 100) as u8;
            }

            // Bands occasionally release new material
            if rng.gen_range(0..12) == 0 {
                // Use data files for new releases instead of hardcoded names
                // This would require access to data files, for now use a simple generator
                band.latest_release = Self::generate_song_title_simple(rng);
            }
        }
    }

    fn change_music_trend(&mut self, rng: &mut impl Rng) {
        self.current_trends = match rng.gen_range(0..6) {
            0 => MusicTrend::Rock,
            1 => MusicTrend::Pop,
            2 => MusicTrend::Metal,
            3 => MusicTrend::Punk,
            4 => MusicTrend::Alternative,
            _ => MusicTrend::Electronic,
        };
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
                    0 => MusicGenre::Rock,
                    1 => MusicGenre::Metal,
                    2 => MusicGenre::Punk,
                    3 => MusicGenre::Alternative,
                    4 => MusicGenre::Pop,
                    _ => MusicGenre::Electronic,
                },
            });
        }

        bands
    }

    fn generate_venues(data_files: &GameDataFiles) -> Vec<Venue> {
        let mut venues = Vec::new();
        let cities = vec![
            "Downtown",
            "City Center",
            "Industrial District",
            "Uptown",
            "Sports Complex",
        ];
        let capacities = [50, 200, 500, 2000, 20000];
        let prestiges = [10, 25, 40, 70, 95];
        let payments = [100, 300, 800, 3000, 15000];

        for i in 0..5 {
            venues.push(Venue {
                name: data_files.random_venue_name(),
                capacity: capacities[i],
                prestige: prestiges[i],
                base_payment: payments[i],
                location: format!("{}, {}", cities[i], data_files.random_city()),
            });
        }

        venues
    }

    fn generate_song_title_simple(rng: &mut impl Rng) -> String {
        let adjectives = ["Broken", "Electric", "Dark", "Wild", "Lost", "Burning"];
        let nouns = ["Dreams", "Hearts", "Nights", "Roads", "Wings", "Thunder"];

        format!(
            "{} {}",
            adjectives[rng.gen_range(0..adjectives.len())],
            nouns[rng.gen_range(0..nouns.len())]
        )
    }

    pub fn get_available_venues_for_fame(&self, fame: u8) -> Vec<&Venue> {
        self.venues
            .iter()
            .filter(|venue| venue.prestige <= fame + 20) // Allow some stretch
            .collect()
    }

    pub fn get_market_modifier(&self) -> f32 {
        let demand_mod = self.music_market.demand as f32 / 100.0;
        let saturation_penalty = 1.0 - (self.music_market.saturation as f32 / 200.0);
        let economic_mod = match self.music_market.economic_state {
            EconomicState::Recession => 0.7,
            EconomicState::Stagnant => 0.9,
            EconomicState::Growing => 1.1,
            EconomicState::Booming => 1.3,
        };

        demand_mod * saturation_penalty * economic_mod
    }
}

impl std::fmt::Display for MusicTrend {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            MusicTrend::Rock => write!(f, "Rock"),
            MusicTrend::Pop => write!(f, "Pop"),
            MusicTrend::Metal => write!(f, "Metal"),
            MusicTrend::Punk => write!(f, "Punk"),
            MusicTrend::Alternative => write!(f, "Alternative"),
            MusicTrend::Electronic => write!(f, "Electronic"),
        }
    }
}

impl std::fmt::Display for EconomicState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            EconomicState::Recession => write!(f, "Recession"),
            EconomicState::Stagnant => write!(f, "Stagnant"),
            EconomicState::Growing => write!(f, "Growing"),
            EconomicState::Booming => write!(f, "Booming"),
        }
    }
}
