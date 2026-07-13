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

#[derive(Debug, Serialize, Deserialize)]
pub struct MusicTimeline {
    pub eras: HashMap<u32, MusicEra>,
    pub current_year: u32,
    /// History only happens once: events that have already made the news.
    #[serde(default)]
    pub triggered_events: std::collections::HashSet<String>,
}

impl MusicTimeline {
    pub fn new(data_files: &GameDataFiles) -> Self {
        let mut timeline = Self {
            eras: HashMap::new(),
            current_year: crate::data::constants::STARTING_YEAR,
            triggered_events: std::collections::HashSet::new(),
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

    /// Fire one of the current era's historical events, at most once each.
    /// Returns None once the era's history has fully played out.
    pub fn take_historical_event(&mut self, rng: &mut impl rand::Rng) -> Option<String> {
        if rng.gen_range(0..10) != 0 {
            return None;
        }
        let fresh: Vec<String> = self
            .get_current_era()
            .major_events
            .iter()
            .filter(|event| !self.triggered_events.contains(*event))
            .cloned()
            .collect();
        if fresh.is_empty() {
            return None;
        }
        let event = fresh[rng.gen_range(0..fresh.len())].clone();
        self.triggered_events.insert(event.clone());
        Some(event)
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::data_loader::GameDataFiles;
    use rand::SeedableRng;
    use rand::rngs::StdRng;

    #[test]
    fn historical_events_fire_at_most_once() {
        let data = GameDataFiles::load().expect("data files present");
        let mut timeline = MusicTimeline::new(&data);
        let era_event_count = timeline.get_current_era().major_events.len();

        let mut rng = StdRng::seed_from_u64(1);
        let mut seen = Vec::new();
        for _ in 0..2000 {
            if let Some(event) = timeline.take_historical_event(&mut rng) {
                assert!(
                    !seen.contains(&event),
                    "event '{event}' fired twice — memory failed"
                );
                seen.push(event);
            }
        }

        assert!(!seen.is_empty(), "some events should fire over 2000 weeks");
        assert!(
            seen.len() <= era_event_count,
            "never more distinct events than the era defines ({} > {})",
            seen.len(),
            era_event_count
        );
        // Once exhausted, the era stops producing news.
        assert_eq!(seen.len(), timeline.triggered_events.len());
    }

    #[test]
    fn advancing_year_unlocks_new_events() {
        let data = GameDataFiles::load().expect("data files present");
        let mut timeline = MusicTimeline::new(&data);
        let mut rng = StdRng::seed_from_u64(2);

        // Drain 1970's events.
        for _ in 0..3000 {
            timeline.take_historical_event(&mut rng);
        }
        let after_1970 = timeline.triggered_events.len();

        // A later era brings its own, previously-unseen events.
        timeline.current_year = 1977;
        let mut fired_new = false;
        for _ in 0..3000 {
            if timeline.take_historical_event(&mut rng).is_some() {
                fired_new = true;
            }
        }
        assert!(fired_new, "a new era should surface fresh events");
        assert!(timeline.triggered_events.len() > after_1970);
    }
}
