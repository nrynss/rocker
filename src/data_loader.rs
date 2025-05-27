use rand::{thread_rng, Rng};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::Path;

#[derive(Debug, Deserialize, Serialize)]
pub struct TimelineData {
    pub timeline: HashMap<String, EraData>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct EraData {
    pub era_name: String,
    pub dominant_genres: Vec<String>,
    pub market_conditions: MarketConditionsData,
    pub major_events: Vec<String>,
    pub technology_changes: Vec<String>,
    pub industry_trends: IndustryTrendsData,
    pub recording_cost_modifier: f32,
    pub gig_pay_modifier: f32,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct MarketConditionsData {
    pub overall_demand: u8,
    pub saturation: u8,
    pub innovation_openness: u8,
    pub major_label_dominance: u8,
    pub touring_market: u8,
    pub record_sales_growth: f32,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct IndustryTrendsData {
    pub album_vs_singles: f32,
    pub studio_quality_importance: u8,
    pub image_importance: u8,
    pub media_influence: u8,
    pub fan_loyalty_factor: u8,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct RecordLabelsData {
    pub major_labels: Vec<RecordLabel>,
    pub independent_labels: Vec<RecordLabel>,
    pub boutique_labels: Vec<RecordLabel>,
    pub label_requirements: LabelRequirements,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct RecordLabel {
    pub name: String,
    pub grade: String,
    pub market_reach: u8,
    pub financial_power: u8,
    pub artist_development: u8,
    pub creative_freedom: u8,
    pub royalty_rate: u8,
    pub advance_range: [u32; 2],
    pub specialty_genres: Vec<String>,
    pub founded: u32,
    pub reputation: String,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct LabelRequirements {
    pub major_label_interest_threshold: InterestThreshold,
    pub independent_label_interest_threshold: InterestThreshold,
    pub boutique_label_interest_threshold: InterestThreshold,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct InterestThreshold {
    pub fame: u8,
    pub albums: u8,
    pub singles: u8,
    pub buzz: u8,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct MarketsData {
    pub markets: HashMap<String, CountryMarket>,
    pub market_modifiers: MarketModifiers,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct CountryMarket {
    pub regions: HashMap<String, RegionMarket>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct RegionMarket {
    pub name: String,
    pub major_cities: Vec<String>,
    pub population: u32,
    pub economic_strength: u8,
    pub music_acceptance: HashMap<String, u8>,
    pub venue_density: u8,
    pub media_influence: u8,
    pub record_sales_per_capita: f32,
    pub touring_infrastructure: u8,
    pub cultural_trends: Vec<String>,
    pub economic_factors: EconomicFactors,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct EconomicFactors {
    pub disposable_income: u8,
    pub unemployment_rate: f32,
    pub music_spending_ratio: f32,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct MarketModifiers {
    pub breakthrough_thresholds: HashMap<String, BreakthroughLevel>,
    pub genre_era_modifiers: HashMap<String, HashMap<String, f32>>,
    pub economic_cycle_effects: HashMap<String, EconomicCycleEffect>,
    pub cultural_resistance_factors: HashMap<String, HashMap<String, f32>>,
    pub touring_costs: HashMap<String, TouringCosts>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct BreakthroughLevel {
    pub fame_required: u8,
    pub fan_base: u32,
    pub revenue_multiplier: f32,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct EconomicCycleEffect {
    pub record_sales_modifier: f32,
    pub touring_revenue_modifier: f32,
    pub label_advance_modifier: f32,
    pub merchandise_modifier: f32,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct TouringCosts {
    pub base_cost_per_show: u32,
    pub travel_cost_modifier: f32,
    pub equipment_cost_modifier: f32,
}

pub struct GameDataFiles {
    pub song_adjectives: Vec<String>,
    pub song_nouns: Vec<String>,
    pub song_verbs: Vec<String>,
    pub song_emotions: Vec<String>,
    pub song_places: Vec<String>,
    pub album_titles: Vec<String>,
    pub band_names: Vec<String>,
    pub band_member_names: Vec<String>,
    pub venue_names: Vec<String>,
    pub city_names: Vec<String>,
    pub timeline_data: TimelineData,
    pub record_labels_data: RecordLabelsData,
    pub markets_data: MarketsData,
}

impl GameDataFiles {
    pub fn load() -> Result<Self, Box<dyn std::error::Error>> {
        // Check if data directory exists
        if !Path::new("data").exists() {
            return Err("Data directory not found! Please create the data/ directory with all required files.".into());
        }

        Ok(Self {
            song_adjectives: Self::load_text_file("data/song_adjectives.txt")?,
            song_nouns: Self::load_text_file("data/song_nouns.txt")?,
            song_verbs: Self::load_text_file("data/song_verbs.txt")?,
            song_emotions: Self::load_text_file("data/song_emotions.txt")?,
            song_places: Self::load_text_file("data/song_places.txt")?,
            album_titles: Self::load_text_file("data/album_titles.txt")?,
            band_names: Self::load_text_file("data/band_names.txt")?,
            band_member_names: Self::load_text_file("data/band_member_names.txt")?,
            venue_names: Self::load_text_file("data/venue_names.txt")?,
            city_names: Self::load_text_file("data/city_names.txt")?,
            timeline_data: Self::load_json_file("data/timeline.json")?,
            record_labels_data: Self::load_json_file("data/record_labels.json")?,
            markets_data: Self::load_json_file("data/markets.json")?,
        })
    }

    fn load_text_file(path: &str) -> Result<Vec<String>, Box<dyn std::error::Error>> {
        if !Path::new(path).exists() {
            return Err(format!("Required file not found: {}", path).into());
        }

        let content = fs::read_to_string(path)?;
        Ok(content
            .lines()
            .map(|line| line.trim().to_string())
            .filter(|line| !line.is_empty() && !line.starts_with('#'))
            .collect())
    }

    fn load_json_file<T: for<'de> Deserialize<'de>>(
        path: &str,
    ) -> Result<T, Box<dyn std::error::Error>> {
        if !Path::new(path).exists() {
            return Err(format!("Required file not found: {}", path).into());
        }

        let content = fs::read_to_string(path)?;
        Ok(serde_json::from_str(&content)?)
    }

    pub fn random_song_title(&self) -> String {
        let mut rng = thread_rng();

        match rng.gen_range(0..5) {
            0 => self.adjective_noun_pattern(&mut rng),
            1 => self.verb_pattern(&mut rng),
            2 => self.emotion_pattern(&mut rng),
            3 => self.place_pattern(&mut rng),
            _ => self.simple_pattern(&mut rng),
        }
    }

    fn adjective_noun_pattern(&self, rng: &mut impl Rng) -> String {
        let adj = &self.song_adjectives[rng.gen_range(0..self.song_adjectives.len())];
        let noun = &self.song_nouns[rng.gen_range(0..self.song_nouns.len())];
        format!("{} {}", adj, noun)
    }

    fn verb_pattern(&self, rng: &mut impl Rng) -> String {
        let verb = &self.song_verbs[rng.gen_range(0..self.song_verbs.len())];
        let noun = &self.song_nouns[rng.gen_range(0..self.song_nouns.len())];
        format!("{} {}", verb, noun)
    }

    fn emotion_pattern(&self, rng: &mut impl Rng) -> String {
        self.song_emotions[rng.gen_range(0..self.song_emotions.len())].clone()
    }

    fn place_pattern(&self, rng: &mut impl Rng) -> String {
        self.song_places[rng.gen_range(0..self.song_places.len())].clone()
    }

    fn simple_pattern(&self, rng: &mut impl Rng) -> String {
        self.song_nouns[rng.gen_range(0..self.song_nouns.len())].clone()
    }

    pub fn random_album_title(&self) -> String {
        let mut rng = thread_rng();

        if rng.gen_bool(0.3) {
            self.album_titles[rng.gen_range(0..self.album_titles.len())].clone()
        } else {
            self.random_song_title()
        }
    }

    pub fn random_band_name(&self) -> String {
        let mut rng = thread_rng();
        self.band_names[rng.gen_range(0..self.band_names.len())].clone()
    }

    pub fn random_band_member_name(&self) -> String {
        let mut rng = thread_rng();
        self.band_member_names[rng.gen_range(0..self.band_member_names.len())].clone()
    }

    pub fn random_venue_name(&self) -> String {
        let mut rng = thread_rng();
        self.venue_names[rng.gen_range(0..self.venue_names.len())].clone()
    }

    pub fn random_city(&self) -> String {
        let mut rng = thread_rng();
        self.city_names[rng.gen_range(0..self.city_names.len())].clone()
    }

    pub fn get_timeline_data(&self) -> &TimelineData {
        &self.timeline_data
    }

    pub fn get_record_labels_data(&self) -> &RecordLabelsData {
        &self.record_labels_data
    }

    pub fn get_markets_data(&self) -> &MarketsData {
        &self.markets_data
    }

    pub fn get_labels_for_tier(&self, tier: &str) -> &Vec<RecordLabel> {
        match tier {
            "major" => &self.record_labels_data.major_labels,
            "independent" => &self.record_labels_data.independent_labels,
            "boutique" => &self.record_labels_data.boutique_labels,
            _ => &self.record_labels_data.independent_labels,
        }
    }

    pub fn get_market_region(&self, country: &str, region: &str) -> Option<&RegionMarket> {
        self.markets_data
            .markets
            .get(country)
            .and_then(|country_data| country_data.regions.get(region))
    }

    pub fn get_all_regions(&self) -> Vec<&RegionMarket> {
        self.markets_data
            .markets
            .values()
            .flat_map(|country| country.regions.values())
            .collect()
    }

    pub fn get_genre_modifier(&self, year: u32, genre: &str) -> f32 {
        let year_str = year.to_string();
        self.markets_data
            .market_modifiers
            .genre_era_modifiers
            .get(&year_str)
            .and_then(|year_modifiers| year_modifiers.get(genre))
            .copied()
            .unwrap_or(1.0)
    }

    pub fn get_economic_cycle_effect(&self, cycle: &str) -> Option<&EconomicCycleEffect> {
        self.markets_data
            .market_modifiers
            .economic_cycle_effects
            .get(cycle)
    }

    pub fn validate_data_files() -> Result<(), Box<dyn std::error::Error>> {
        let required_files = [
            "data/song_adjectives.txt",
            "data/song_nouns.txt",
            "data/song_verbs.txt",
            "data/song_emotions.txt",
            "data/song_places.txt",
            "data/album_titles.txt",
            "data/band_names.txt",
            "data/band_member_names.txt",
            "data/venue_names.txt",
            "data/city_names.txt",
            "data/timeline.json",
            "data/record_labels.json",
            "data/markets.json",
        ];

        for file in &required_files {
            if !Path::new(file).exists() {
                return Err(format!("Required data file missing: {}", file).into());
            }
        }

        Ok(())
    }
}
