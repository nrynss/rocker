use serde::{Deserialize, Serialize};
use super::music::{Song, Release}; // Import new structs

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Band {
    pub name: String,
    pub fame: u8,  // 0-100
    pub skill: u8, // 0-100
    pub unreleased_songs: Vec<Song>,
    pub singles_released: Vec<Release>,
    pub albums_released: Vec<Release>,
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
    pub label_name: String,
    pub label_tier: String, // e.g., "Major", "Independent", "Boutique"
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
            unreleased_songs: Vec::new(),
            singles_released: Vec::new(),
            albums_released: Vec::new(),
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
        self.unreleased_songs.len() >= crate::data::constants::MIN_ALBUM_SONGS as usize // Assuming MIN_ALBUM_SONGS is available
    }

    pub fn can_record_single(&self) -> bool {
        self.unreleased_songs.len() >= 1
    }

    pub fn total_releases(&self) -> usize { // Changed to usize to match Vec::len()
        self.singles_released.len() + self.albums_released.len()
    }

    pub fn dominant_genres_match(&self, target_genres: &[&str]) -> bool {
        // This is a simplified check - in a full implementation,
        // the band would have a genre field
        // For now, we'll use a placeholder that returns true for any genre
        // TODO: Add actual genre tracking to Band struct
        target_genres.len() > 0 // Placeholder - always return true if genres provided
    }

    pub fn current_deal(&self) -> Option<&RecordDeal> {
        self.record_deal.as_ref()
    }

    pub fn sign_deal(&mut self, deal: RecordDeal) {
        self.record_deal = Some(deal);
    }

    pub fn fulfill_album_obligation(&mut self) -> bool {
        if let Some(deal) = &mut self.record_deal {
            deal.albums_delivered += 1;
            if deal.albums_delivered >= deal.albums_required {
                // Deal completed
                // For now, let's clear the deal. Another option could be to mark it as completed.
                self.record_deal = None; 
                return true; // Deal completed
            }
        }
        false // Deal not completed or no deal active
    }

    pub fn remaining_albums_for_deal(&self) -> u8 {
        if let Some(deal) = &self.record_deal {
            if deal.albums_delivered < deal.albums_required {
                return deal.albums_required - deal.albums_delivered;
            }
        }
        0 // No deal or deal completed
    }

    pub fn drop_deal(&mut self) {
        self.record_deal = None;
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
