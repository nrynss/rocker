use super::genre::MusicGenre;
use super::music::{Release, Song}; // Import new structs
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Band {
    pub name: String,
    /// The sound the band plays. Saves from before genres existed load as Rock.
    #[serde(default)]
    pub genre: MusicGenre,
    pub fame: u8, // 0-100
    /// Highest fame the band has ever reached — the peak that its permanent
    /// floors are earned against (see fame gravity, design §C). Old saves
    /// default to 0; every read lifts it to current fame so a loaded career
    /// never forgets a peak it already stood on.
    #[serde(default)]
    pub peak_fame: u8,
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
    /// The label's distribution muscle (0-100), taken from the label data.
    #[serde(default = "default_market_reach")]
    pub market_reach: u8,
}

fn default_market_reach() -> u8 {
    50
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
            genre: MusicGenre::Rock,
            fame: 0,
            peak_fame: 0,
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
    /// The peak fame the band has stood on, robust against pre-0.6 saves that
    /// default `peak_fame` to 0: it can never read lower than current fame.
    pub fn effective_peak_fame(&self) -> u8 {
        self.peak_fame.max(self.fame)
    }

    /// Add fame the one true way. While the band is climbing back toward a
    /// peak it has already reached, the gain is doubled (the comeback rule,
    /// design §C); the result is clamped to `MAX_FAME` and the peak updated.
    /// Fame *losses* (idle decay, bad events) must not route through here.
    pub fn gain_fame(&mut self, amount: u8) {
        let peak = self.effective_peak_fame();
        let multiplier = if self.fame < peak {
            u16::from(crate::game::constants::FAME_COMEBACK_MULTIPLIER)
        } else {
            1
        };
        let gained = u16::from(amount) * multiplier;
        self.fame =
            (u16::from(self.fame) + gained).min(u16::from(crate::game::constants::MAX_FAME)) as u8;
        self.peak_fame = peak.max(self.fame);
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
        !self.unreleased_songs.is_empty()
    }

    pub fn total_releases(&self) -> usize {
        // Changed to usize to match Vec::len()
        self.singles_released.len() + self.albums_released.len()
    }

    /// Whether the band's genre answers to any of the given labels. Matching
    /// is tolerant of surface form: "Hair Metal" and "hair_metal" both match
    /// Metal (via [`MusicGenre::aliases`]), "Grunge" matches Alternative, and
    /// so on — so historical events can name sub-genres the coarse enum folds
    /// together. A label that maps to no genre (e.g. "Folk Rock") simply
    /// matches nothing.
    pub fn dominant_genres_match(&self, target_genres: &[&str]) -> bool {
        fn normalize(s: &str) -> String {
            s.chars()
                .filter(|c| c.is_ascii_alphanumeric())
                .map(|c| c.to_ascii_lowercase())
                .collect()
        }
        let keys: Vec<String> = std::iter::once(self.genre.name())
            .chain(self.genre.aliases().iter().copied())
            .map(normalize)
            .collect();
        target_genres
            .iter()
            .any(|target| keys.contains(&normalize(target)))
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
