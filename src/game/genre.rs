//! The single genre enum for the player band, scene acts, and releases.
//! Future Musician work (ability-derived proficiency) hangs off this type.

use crate::game::timeline::MusicTimeline;
use rand::Rng;
use serde::{Deserialize, Serialize};

// Ensure MusicGenre can be used as a HashMap key
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum MusicGenre {
    #[default]
    Rock,
    Pop,
    Metal,
    Punk,
    Alternative,
    Electronic,
    Folk,
    Jazz,
}

impl MusicGenre {
    pub const ALL: [MusicGenre; 8] = [
        MusicGenre::Rock,
        MusicGenre::Pop,
        MusicGenre::Metal,
        MusicGenre::Punk,
        MusicGenre::Alternative,
        MusicGenre::Electronic,
        MusicGenre::Folk,
        MusicGenre::Jazz,
    ];

    pub fn name(&self) -> &'static str {
        match self {
            MusicGenre::Rock => "Rock",
            MusicGenre::Pop => "Pop",
            MusicGenre::Metal => "Metal",
            MusicGenre::Punk => "Punk",
            MusicGenre::Alternative => "Alternative",
            MusicGenre::Electronic => "Electronic",
            MusicGenre::Folk => "Folk",
            MusicGenre::Jazz => "Jazz",
        }
    }

    /// Keys this genre answers to in markets.json's genre_era_modifiers.
    pub fn aliases(&self) -> &'static [&'static str] {
        match self {
            MusicGenre::Rock => &["rock", "arena_rock"],
            MusicGenre::Pop => &["pop", "synth_pop", "brit_pop", "disco"],
            MusicGenre::Metal => &["metal", "hair_metal", "pop_metal"],
            MusicGenre::Punk => &["punk", "post_punk"],
            MusicGenre::Alternative => &["alternative", "grunge", "new_wave"],
            MusicGenre::Electronic => &["synth_pop", "house", "new_wave", "disco"],
            MusicGenre::Folk => &["folk"],
            MusicGenre::Jazz => &["jazz", "blues"],
        }
    }

    pub(crate) fn random(rng: &mut impl Rng) -> Self {
        MusicGenre::ALL[rng.gen_range(0..MusicGenre::ALL.len())].clone()
    }

    /// A genre that fits the current trends, if any does.
    pub(crate) fn random_trending(timeline: &MusicTimeline, rng: &mut impl Rng) -> Self {
        let trending = timeline.get_trending_genres();
        let matching: Vec<MusicGenre> = MusicGenre::ALL
            .iter()
            .filter(|genre| trending.iter().any(|t| t.contains(genre.name())))
            .cloned()
            .collect();
        if matching.is_empty() {
            Self::random(rng)
        } else {
            matching[rng.gen_range(0..matching.len())].clone()
        }
    }
}

impl std::fmt::Display for MusicGenre {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.name())
    }
}
