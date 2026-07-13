//! Gig venues generated with the world.

use crate::data_loader::GameDataFiles;
use rand::Rng;
use serde::{Deserialize, Serialize};

use super::GameWorld;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Venue {
    pub name: String,
    pub capacity: u32,
    pub prestige: u8, // 0-100
    pub base_payment: u32,
    pub location: String,
}

impl GameWorld {
    pub(super) fn generate_venues(data_files: &GameDataFiles, rng: &mut impl Rng) -> Vec<Venue> {
        let districts = [
            "Downtown",
            "City Center",
            "Industrial District",
            "Uptown",
            "Sports Complex",
        ];
        let capacities = [50, 200, 500, 2000, 20000];
        let prestiges = [10, 25, 40, 70, 95];
        let payments = [100, 300, 800, 3000, 15000];

        let mut venues = Vec::new();
        for i in 0..5 {
            venues.push(Venue {
                name: data_files.venue_names[rng.gen_range(0..data_files.venue_names.len())]
                    .clone(),
                capacity: capacities[i],
                prestige: prestiges[i],
                base_payment: payments[i],
                location: format!(
                    "{}, {}",
                    districts[i],
                    data_files.city_names[rng.gen_range(0..data_files.city_names.len())].clone()
                ),
            });
        }

        venues
    }
}
