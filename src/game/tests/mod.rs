//! Unit tests for the `game` module, split by concern.
//!
//! Relocated out of `mod.rs` in the v0.5.1 structure cycle (T1) so the module
//! surface isn't buried under ~750 lines of tests. The shared harness lives
//! here; each concern is a submodule that pulls it in via `use super::*`.
//! Test function names are unchanged from when they lived in `mod.rs`.

use crate::game::music::{Release, ReleaseType};
use crate::game::world::PotentialDealOffer;

use super::constants::{self, *};
use super::*;

mod deals;
mod determinism;
mod fame;
mod history;
mod incidents;
mod label_moves;
mod lifestyle;
mod releases;
mod save_compat;
mod shows;
mod smoke;
mod studio;
mod support;

fn test_game() -> Game {
    Game::new().expect("data files present")
}

fn test_release(id: u32, release_type: ReleaseType) -> Release {
    Release {
        id,
        name: format!("Test Release {id}"),
        release_type,
        release_quality: 50,
        week_released: 0,
        songs_involved_quality_avg: 50,
        active_marketing: Vec::new(),
        marketing_level_achieved: 0,
        initial_sales_score: 0,
        total_income_generated: 0,
        genre: None,
        copies_pressed: 0,
        copies_sold: 0,
        peak_chart_position: None,
        singles_cut: 0,
    }
}

/// The biggest venue whose door policy admits the band right now.
fn best_open_venue(game: &Game) -> usize {
    (0..game.world.venues.len())
        .filter(|&i| game.world.venues[i].prestige <= game.band.fame.saturating_add(20))
        .max_by_key(|&i| game.world.venues[i].capacity)
        .expect("at least one venue is always open")
}

fn test_deal(market_reach: u8, royalty_rate: f32) -> band::RecordDeal {
    band::RecordDeal {
        label_name: "Test Records".to_string(),
        label_tier: "Major".to_string(),
        advance: 0,
        royalty_rate,
        albums_required: 2,
        albums_delivered: 0,
        market_reach,
    }
}

/// A pending offer as `check_and_generate_deal_offers` would leave it.
fn test_deal_offer(game: &Game, expires_week: Option<u32>) -> PotentialDealOffer {
    let label = game.data_files.get_record_labels_data().independent_labels[0].clone();
    PotentialDealOffer {
        label_name: label.name.clone(),
        label_tier: "Independent".to_string(),
        advance: 1_000,
        royalty_rate: 0.12,
        albums_required: 1,
        original_label_data: label,
        expires_week,
    }
}
