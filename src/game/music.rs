use rand::{thread_rng, Rng};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Song {
    pub title: String,
    pub quality: u8, // 0-100
    pub style: MusicStyle,
    pub length: u16,           // in seconds
    pub lyrics_quality: u8,    // 0-100
    pub commercial_appeal: u8, // 0-100
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Album {
    pub title: String,
    pub songs: Vec<Song>,
    pub overall_quality: u8,
    pub production_quality: u8,
    pub artwork_quality: u8,
    pub release_date: u32, // week number
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Single {
    pub song: Song,
    pub b_side: Option<Song>,
    pub production_quality: u8,
    pub marketing_budget: u32,
    pub chart_position: Option<u8>, // 1-100, lower is better
    pub weeks_on_chart: u8,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum MusicStyle {
    Rock,
    HardRock,
    Metal,
    Punk,
    Alternative,
    Pop,
    Blues,
    Folk,
    Experimental,
}

pub struct MusicGenerator;

impl MusicGenerator {
    pub fn generate_song_title() -> String {
        let mut rng = thread_rng();

        let title_patterns = [
            Self::adjective_noun_pattern,
            Self::verb_noun_pattern,
            Self::place_pattern,
            Self::emotion_pattern,
            Self::abstract_pattern,
        ];

        let pattern = title_patterns[rng.gen_range(0..title_patterns.len())];
        pattern(&mut rng)
    }

    fn adjective_noun_pattern(rng: &mut impl Rng) -> String {
        let adjectives = [
            "Broken", "Electric", "Dark", "Wild", "Lost", "Burning", "Frozen", "Crimson", "Silver",
            "Golden", "Black", "White", "Neon", "Velvet",
        ];
        let nouns = [
            "Dreams",
            "Hearts",
            "Nights",
            "Roads",
            "Wings",
            "Thunder",
            "Lightning",
            "Fire",
            "Ice",
            "Storm",
            "Rain",
            "Sun",
            "Moon",
            "Stars",
            "Angels",
        ];

        format!(
            "{} {}",
            adjectives[rng.gen_range(0..adjectives.len())],
            nouns[rng.gen_range(0..nouns.len())]
        )
    }

    fn verb_noun_pattern(rng: &mut impl Rng) -> String {
        let verbs = [
            "Running",
            "Flying",
            "Falling",
            "Dancing",
            "Screaming",
            "Whisper",
            "Chasing",
            "Breaking",
            "Building",
            "Destroying",
            "Creating",
            "Loving",
        ];
        let objects = [
            "Free",
            "Wild",
            "High",
            "Low",
            "Fast",
            "Slow",
            "Hard",
            "Soft",
            "Through the Night",
            "in the Rain",
            "with Fire",
            "like Thunder",
        ];

        format!(
            "{} {}",
            verbs[rng.gen_range(0..verbs.len())],
            objects[rng.gen_range(0..objects.len())]
        )
    }

    fn place_pattern(rng: &mut impl Rng) -> String {
        let places = [
            "Highway 101",
            "Downtown Blues",
            "City Lights",
            "Country Road",
            "Midnight Train",
            "Ocean Drive",
            "Mountain High",
            "Valley Low",
            "Desert Wind",
            "Forest Deep",
            "River Wide",
            "Bridge to Nowhere",
        ];

        places[rng.gen_range(0..places.len())].to_string()
    }

    fn emotion_pattern(rng: &mut impl Rng) -> String {
        let emotions = [
            "Love Me Tonight",
            "Break My Heart",
            "Make Me Whole",
            "Tear Me Down",
            "Lift Me Up",
            "Let Me Go",
            "Hold Me Close",
            "Set Me Free",
            "Make Me Feel",
            "Help Me Heal",
            "Show Me Love",
            "Give Me Hope",
        ];

        emotions[rng.gen_range(0..emotions.len())].to_string()
    }

    fn abstract_pattern(rng: &mut impl Rng) -> String {
        let concepts = [
            "Time", "Space", "Reality", "Dreams", "Memory", "Hope", "Fear", "Truth", "Lies",
            "Power", "Freedom", "Destiny", "Karma", "Zen",
        ];
        let modifiers = [
            "Eternal",
            "Infinite",
            "Ultimate",
            "Perfect",
            "Broken",
            "Lost",
            "Found",
            "Hidden",
            "Secret",
            "Sacred",
            "Forbidden",
            "Ancient",
        ];

        if rng.gen_bool(0.5) {
            format!(
                "{} {}",
                modifiers[rng.gen_range(0..modifiers.len())],
                concepts[rng.gen_range(0..concepts.len())]
            )
        } else {
            concepts[rng.gen_range(0..concepts.len())].to_string()
        }
    }

    pub fn generate_album_title() -> String {
        let mut rng = thread_rng();

        let album_patterns = [
            "The Chronicles",
            "Volume One",
            "Greatest Hits",
            "Live at the Venue",
            "Unplugged",
            "The Collection",
            "Best Of",
            "Rarities",
        ];

        if rng.gen_bool(0.7) {
            // Use song title pattern for album
            Self::generate_song_title()
        } else {
            // Use album-specific pattern
            album_patterns[rng.gen_range(0..album_patterns.len())].to_string()
        }
    }

    pub fn calculate_song_quality(band_skill: u8, player_energy: u8, creativity_bonus: u8) -> u8 {
        let mut rng = thread_rng();

        let base_quality = band_skill as i16;
        let energy_modifier = if player_energy > 50 { 10i16 } else { 0i16 };
        let creativity_modifier = creativity_bonus as i16;
        let random_factor = rng.gen_range(-10..=10);

        let total = base_quality + energy_modifier + creativity_modifier + random_factor;
        total.clamp(1, 100) as u8
    }

    pub fn determine_commercial_appeal(
        quality: u8,
        current_trend: &crate::game::world::MusicTrend,
        style: &MusicStyle,
    ) -> u8 {
        let appeal = quality;

        // Adjust based on current trends
        let trend_bonus = match (current_trend, style) {
            (crate::game::world::MusicTrend::Rock, MusicStyle::Rock) => 10,
            (crate::game::world::MusicTrend::Rock, MusicStyle::HardRock) => 8,
            (crate::game::world::MusicTrend::Metal, MusicStyle::Metal) => 10,
            (crate::game::world::MusicTrend::Punk, MusicStyle::Punk) => 10,
            (crate::game::world::MusicTrend::Alternative, MusicStyle::Alternative) => 10,
            (crate::game::world::MusicTrend::Pop, MusicStyle::Pop) => 10,
            (crate::game::world::MusicTrend::Electronic, MusicStyle::Experimental) => 5,
            _ => 0,
        };

        (appeal + trend_bonus).min(100)
    }
}

impl Song {
    pub fn new(title: String, band_skill: u8, player_energy: u8) -> Self {
        let mut rng = thread_rng();

        let quality = MusicGenerator::calculate_song_quality(band_skill, player_energy, 0);
        let style = match rng.gen_range(0..9) {
            0 => MusicStyle::Rock,
            1 => MusicStyle::HardRock,
            2 => MusicStyle::Metal,
            3 => MusicStyle::Punk,
            4 => MusicStyle::Alternative,
            5 => MusicStyle::Pop,
            6 => MusicStyle::Blues,
            7 => MusicStyle::Folk,
            _ => MusicStyle::Experimental,
        };

        Self {
            title,
            quality,
            style,
            length: rng.gen_range(120..300), // 2-5 minutes
            lyrics_quality: quality + rng.gen_range(-10..=10).clamp(1, 100) as u8,
            commercial_appeal: quality + rng.gen_range(-15..=15).clamp(1, 100) as u8,
        }
    }
}

impl Single {
    pub fn new(song: Song, production_budget: u32) -> Self {
        let production_quality = ((production_budget / 10).min(100)) as u8;

        Self {
            song,
            b_side: None,
            production_quality,
            marketing_budget: 0,
            chart_position: None,
            weeks_on_chart: 0,
        }
    }

    pub fn calculate_chart_potential(&self, band_fame: u8, market_modifier: f32) -> u8 {
        let base_potential =
            (self.song.quality + self.song.commercial_appeal + self.production_quality) / 3;
        let fame_bonus = band_fame / 5;
        let marketing_bonus = (self.marketing_budget / 100).min(20) as u8;

        let total =
            ((base_potential + fame_bonus + marketing_bonus) as f32 * market_modifier) as u8;
        total.min(100)
    }
}

impl Album {
    pub fn new(title: String, songs: Vec<Song>, production_budget: u32, week: u32) -> Self {
        let overall_quality = if songs.is_empty() {
            0
        } else {
            songs.iter().map(|s| s.quality as u32).sum::<u32>() / songs.len() as u32
        } as u8;

        let production_quality = ((production_budget / 100).min(100)) as u8;
        let artwork_quality = rand::thread_rng().gen_range(20..80);

        Self {
            title,
            songs,
            overall_quality,
            production_quality,
            artwork_quality,
            release_date: week,
        }
    }

    pub fn calculate_sales_potential(&self, band_fame: u8, market_modifier: f32) -> u32 {
        let quality_factor =
            (self.overall_quality + self.production_quality + self.artwork_quality) as f32 / 3.0;
        let fame_factor = band_fame as f32;
        let base_sales = (quality_factor * fame_factor * 10.0) as u32;

        (base_sales as f32 * market_modifier) as u32
    }
}

impl std::fmt::Display for MusicStyle {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            MusicStyle::Rock => write!(f, "Rock"),
            MusicStyle::HardRock => write!(f, "Hard Rock"),
            MusicStyle::Metal => write!(f, "Metal"),
            MusicStyle::Punk => write!(f, "Punk"),
            MusicStyle::Alternative => write!(f, "Alternative"),
            MusicStyle::Pop => write!(f, "Pop"),
            MusicStyle::Blues => write!(f, "Blues"),
            MusicStyle::Folk => write!(f, "Folk"),
            MusicStyle::Experimental => write!(f, "Experimental"),
        }
    }
}
