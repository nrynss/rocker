use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Band {
    pub name: String,
    pub fame: u8,  // 0-100
    pub skill: u8, // 0-100
    pub unreleased_songs: u8,
    pub singles: u8,
    pub albums: u8,
    pub members: Vec<BandMember>,
    pub record_deal: Option<RecordDeal>,
    pub reputation: BandReputation,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BandMember {
    pub name: String,
    pub instrument: Instrument,
    pub skill: u8,
    pub loyalty: u8, // 0-100, affects chance of leaving
    pub drug_problem: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Instrument {
    Guitar,
    Bass,
    Drums,
    Keyboard,
    Vocals,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RecordDeal {
    pub label: String,
    pub advance: u32,
    pub royalty_rate: f32, // Percentage
    pub albums_required: u8,
    pub albums_delivered: u8,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BandReputation {
    pub critical_acclaim: u8,   // 0-100
    pub commercial_success: u8, // 0-100
    pub live_performance: u8,   // 0-100
    pub media_presence: u8,     // 0-100
}

impl Default for Band {
    fn default() -> Self {
        Self {
            name: String::new(),
            fame: 0,
            skill: 20,
            unreleased_songs: 0,
            singles: 0,
            albums: 0,
            members: vec![
                BandMember {
                    name: "Dave".to_string(),
                    instrument: Instrument::Guitar,
                    skill: 25,
                    loyalty: 75,
                    drug_problem: false,
                },
                BandMember {
                    name: "Sarah".to_string(),
                    instrument: Instrument::Bass,
                    skill: 20,
                    loyalty: 80,
                    drug_problem: false,
                },
                BandMember {
                    name: "Mike".to_string(),
                    instrument: Instrument::Drums,
                    skill: 30,
                    loyalty: 70,
                    drug_problem: false,
                },
            ],
            record_deal: None,
            reputation: BandReputation::default(),
        }
    }
}

impl Default for BandReputation {
    fn default() -> Self {
        Self {
            critical_acclaim: 10,
            commercial_success: 5,
            live_performance: 15,
            media_presence: 0,
        }
    }
}

impl Band {
    pub fn new(name: String) -> Self {
        let mut band = Self::default();
        band.name = name;
        band
    }

    pub fn get_fame_level(&self) -> &str {
        match self.fame {
            0..=10 => "Unknown",
            11..=25 => "Local scene",
            26..=40 => "Regional",
            41..=60 => "National",
            61..=80 => "International",
            81..=95 => "Superstar",
            _ => "Legend",
        }
    }

    pub fn get_skill_level(&self) -> &str {
        match self.skill {
            0..=20 => "Amateur",
            21..=40 => "Competent",
            41..=60 => "Good",
            61..=80 => "Professional",
            81..=95 => "Expert",
            _ => "Virtuoso",
        }
    }

    pub fn average_member_skill(&self) -> u8 {
        if self.members.is_empty() {
            return 0;
        }
        let total: u32 = self.members.iter().map(|m| m.skill as u32).sum();
        (total / self.members.len() as u32) as u8
    }

    pub fn band_morale(&self) -> u8 {
        if self.members.is_empty() {
            return 0;
        }
        let total: u32 = self.members.iter().map(|m| m.loyalty as u32).sum();
        (total / self.members.len() as u32) as u8
    }

    pub fn has_record_deal(&self) -> bool {
        self.record_deal.is_some()
    }

    pub fn can_record_album(&self) -> bool {
        self.unreleased_songs >= 8
    }

    pub fn can_record_single(&self) -> bool {
        self.unreleased_songs >= 1
    }

    pub fn total_releases(&self) -> u8 {
        self.singles + self.albums
    }

    pub fn dominant_genres_match(&self, target_genres: &[&str]) -> bool {
        // This is a simplified check - in a full implementation,
        // the band would have a genre field
        // For now, we'll use a placeholder that returns true for any genre
        // TODO: Add actual genre tracking to Band struct
        target_genres.len() > 0 // Placeholder - always return true if genres provided
    }
}

impl std::fmt::Display for Instrument {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Instrument::Guitar => write!(f, "Guitar"),
            Instrument::Bass => write!(f, "Bass"),
            Instrument::Drums => write!(f, "Drums"),
            Instrument::Keyboard => write!(f, "Keyboard"),
            Instrument::Vocals => write!(f, "Vocals"),
        }
    }
}
