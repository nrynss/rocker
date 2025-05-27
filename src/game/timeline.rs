use crate::data_loader::GameDataFiles;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MusicEra {
    pub year: u32,
    pub era_name: String,
    pub dominant_genres: Vec<String>,
    pub market_conditions: MarketConditions,
    pub major_events: Vec<String>,
    pub technology_changes: Vec<String>,
    pub industry_trends: IndustryTrends,
    pub recording_cost_modifier: f32,
    pub gig_pay_modifier: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MarketConditions {
    pub overall_demand: u8,
    pub saturation: u8,
    pub innovation_openness: u8,
    pub major_label_dominance: u8,
    pub touring_market: u8,
    pub record_sales_growth: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IndustryTrends {
    pub album_vs_singles: f32,
    pub studio_quality_importance: u8,
    pub image_importance: u8,
    pub media_influence: u8,
    pub fan_loyalty_factor: u8,
}

pub struct MusicTimeline {
    pub eras: HashMap<u32, MusicEra>,
    pub current_year: u32,
}

impl MusicTimeline {
    pub fn new(data_files: &GameDataFiles) -> Self {
        let mut timeline = Self {
            eras: HashMap::new(),
            current_year: 1970,
        };
        timeline.load_from_data_files(data_files);
        timeline
    }

    fn load_from_data_files(&mut self, data_files: &GameDataFiles) {
        let timeline_data = data_files.get_timeline_data();

        for (year_str, era_data) in &timeline_data.timeline {
            if let Ok(year) = year_str.parse::<u32>() {
                let music_era = MusicEra {
                    year,
                    era_name: era_data.era_name.clone(),
                    dominant_genres: era_data.dominant_genres.clone(),
                    market_conditions: MarketConditions {
                        overall_demand: era_data.market_conditions.overall_demand,
                        saturation: era_data.market_conditions.saturation,
                        innovation_openness: era_data.market_conditions.innovation_openness,
                        major_label_dominance: era_data.market_conditions.major_label_dominance,
                        touring_market: era_data.market_conditions.touring_market,
                        record_sales_growth: era_data.market_conditions.record_sales_growth,
                    },
                    major_events: era_data.major_events.clone(),
                    technology_changes: era_data.technology_changes.clone(),
                    industry_trends: IndustryTrends {
                        album_vs_singles: era_data.industry_trends.album_vs_singles,
                        studio_quality_importance: era_data
                            .industry_trends
                            .studio_quality_importance,
                        image_importance: era_data.industry_trends.image_importance,
                        media_influence: era_data.industry_trends.media_influence,
                        fan_loyalty_factor: era_data.industry_trends.fan_loyalty_factor,
                    },
                    recording_cost_modifier: era_data.recording_cost_modifier,
                    gig_pay_modifier: era_data.gig_pay_modifier,
                };

                self.eras.insert(year, music_era);
            }
        }
    }

    pub fn get_current_era(&self) -> &MusicEra {
        self.eras
            .get(&self.current_year)
            .or_else(|| {
                // Find the most recent era if exact year not found
                let mut closest_year = 1970;
                for &year in self.eras.keys() {
                    if year <= self.current_year && year > closest_year {
                        closest_year = year;
                    }
                }
                self.eras.get(&closest_year)
            })
            .expect("Timeline should always have at least the 1970 era")
    }

    pub fn advance_year(&mut self) {
        self.current_year += 1;
    }

    pub fn get_genre_popularity(&self, genre: &str) -> u8 {
        let era = self.get_current_era();
        if era
            .dominant_genres
            .iter()
            .any(|g| g.to_lowercase() == genre.to_lowercase())
        {
            85 + (rand::random::<u8>() % 15) // 85-100
        } else {
            20 + (rand::random::<u8>() % 60) // 20-80
        }
    }

    pub fn get_market_modifier(&self) -> f32 {
        let era = self.get_current_era();
        let base = era.market_conditions.overall_demand as f32 / 100.0;
        let growth = 1.0 + (era.market_conditions.record_sales_growth / 100.0);
        base * growth
    }

    pub fn get_recording_cost_modifier(&self) -> f32 {
        self.get_current_era().recording_cost_modifier
    }

    pub fn get_gig_pay_modifier(&self) -> f32 {
        self.get_current_era().gig_pay_modifier
    }

    pub fn get_image_importance(&self) -> u8 {
        self.get_current_era().industry_trends.image_importance
    }

    pub fn get_media_influence(&self) -> u8 {
        self.get_current_era().industry_trends.media_influence
    }

    pub fn is_album_era(&self) -> bool {
        self.get_current_era().industry_trends.album_vs_singles > 0.7
    }

    pub fn get_innovation_bonus(&self) -> u8 {
        self.get_current_era().market_conditions.innovation_openness
    }

    pub fn get_major_label_power(&self) -> u8 {
        self.get_current_era()
            .market_conditions
            .major_label_dominance
    }

    pub fn get_era_description(&self) -> String {
        let era = self.get_current_era();
        format!(
            "{} ({}): {}",
            self.current_year,
            era.era_name,
            era.major_events.join(", ")
        )
    }

    pub fn get_trending_genres(&self) -> Vec<String> {
        self.get_current_era().dominant_genres.clone()
    }

    pub fn should_trigger_historical_event(&self) -> Option<String> {
        let era = self.get_current_era();
        if rand::random::<f32>() < 0.1 {
            // 10% chance
            Some(era.major_events[rand::random::<usize>() % era.major_events.len()].clone())
        } else {
            None
        }
    }

    pub fn get_current_year(&self) -> u32 {
        self.current_year
    }

    pub fn get_studio_quality_importance(&self) -> u8 {
        self.get_current_era()
            .industry_trends
            .studio_quality_importance
    }

    pub fn get_fan_loyalty_factor(&self) -> u8 {
        self.get_current_era().industry_trends.fan_loyalty_factor
    }
}
