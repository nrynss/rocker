//! The living world: market, scene, charts, venues, and deal scouting.

mod charts;
mod deals;
mod scene;
mod venues;

// Public API re-exports (scene size / chart width are part of the world surface;
// some are only read from tests today, but stay on the module root).
#[allow(unused_imports)]
pub use charts::{CHART_SIZE, ChartEntry};
pub use deals::PotentialDealOffer;
#[allow(unused_imports)]
pub use scene::{SCENE_MAX_BANDS, SCENE_MIN_BANDS, SCENE_START_BANDS, SceneBand};
pub use venues::Venue;

use crate::data_loader::GameDataFiles;
use crate::game::genre::MusicGenre;
use crate::game::timeline::MusicTimeline;
use rand::Rng;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GameWorld {
    pub music_market: MusicMarket,
    /// Every act on the scene besides the player's. Not "rivals" — just the
    /// hundreds of bands a real scene is made of.
    #[serde(alias = "competing_bands")]
    pub bands: Vec<SceneBand>,
    pub venues: Vec<Venue>,
    pub current_trends: MusicTrend,
    #[serde(default)]
    pub dynamic_genre_modifiers: std::collections::HashMap<MusicGenre, f32>,
    #[serde(default)]
    pub charts: Vec<ChartEntry>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MusicMarket {
    pub demand: u8,     // 0-100, affects earnings
    pub saturation: u8, // 0-100, affects difficulty
    pub economic_state: EconomicState,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum EconomicState {
    Recession,
    Stagnant,
    Growing,
    Booming,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum MusicTrend {
    Rock,
    Pop,
    Metal,
    Punk,
    Alternative,
    Electronic,
}

impl GameWorld {
    pub fn new(data_files: &GameDataFiles, rng: &mut impl Rng) -> Self {
        Self {
            music_market: MusicMarket {
                demand: 50,
                saturation: 55,
                economic_state: EconomicState::Growing,
            },
            bands: Self::generate_scene(data_files, rng),
            venues: Self::generate_venues(data_files, rng),
            current_trends: MusicTrend::Rock,
            dynamic_genre_modifiers: std::collections::HashMap::new(),
            charts: Vec::new(),
        }
    }

    /// Advance the world by one week. Returns newsworthy events.
    pub fn update_week(
        &mut self,
        timeline: &MusicTimeline,
        data_files: &GameDataFiles,
        rng: &mut impl Rng,
    ) -> Vec<String> {
        let mut news = Vec::new();

        self.update_market_with_timeline(rng, timeline, &mut news);
        self.decay_charts(&mut news);
        self.update_scene_bands(rng, timeline, data_files, &mut news);
        self.update_scene_population(rng, timeline, data_files, &mut news);
        self.update_trends_with_timeline(timeline, rng);

        // Decay dynamic genre modifiers
        let mut new_modifiers = std::collections::HashMap::new();
        for (genre, val) in self.dynamic_genre_modifiers.iter() {
            let decayed_val = (*val - 1.0) * 0.95 + 1.0;
            // Only keep it if it's significantly different from 1.0
            if (decayed_val - 1.0).abs() > 0.01 {
                new_modifiers.insert(genre.clone(), decayed_val);
            }
        }
        self.dynamic_genre_modifiers = new_modifiers;

        news
    }
    fn update_market_with_timeline(
        &mut self,
        rng: &mut impl Rng,
        timeline: &MusicTimeline,
        news: &mut Vec<String>,
    ) {
        let era = timeline.get_current_era();

        // Adjust market demand towards historical era demand
        let target_demand = era.market_conditions.overall_demand;
        if self.music_market.demand < target_demand {
            self.music_market.demand = (self.music_market.demand + 2).min(target_demand);
        } else if self.music_market.demand > target_demand {
            self.music_market.demand =
                (self.music_market.demand.saturating_sub(2)).max(target_demand);
        }

        // Update economic state based on era and some randomness
        if rng.gen_range(0..10) == 0 {
            let new_state = match era.market_conditions.record_sales_growth {
                x if x > 20.0 => EconomicState::Booming,
                x if x > 10.0 => EconomicState::Growing,
                x if x > 0.0 => EconomicState::Stagnant,
                _ => EconomicState::Recession,
            };
            if new_state != self.music_market.economic_state {
                news.push(match new_state {
                    EconomicState::Booming => {
                        "📈 Record sales are BOOMING — everyone's buying.".to_string()
                    }
                    EconomicState::Growing => {
                        "📈 The record industry is growing again.".to_string()
                    }
                    EconomicState::Stagnant => "📊 Industry sales have flattened out.".to_string(),
                    EconomicState::Recession => {
                        "📉 The industry has slid into a slump — money is tight.".to_string()
                    }
                });
                self.music_market.economic_state = new_state;
            }
        }

        // Saturation tracks how crowded the scene is; innovation opens space.
        let target_saturation = ((25 + self.bands.len() / 4) as u8).min(90);
        if self.music_market.saturation < target_saturation {
            self.music_market.saturation += 1;
        } else if self.music_market.saturation > target_saturation {
            self.music_market.saturation -= 1;
        }
        if era.market_conditions.innovation_openness > 80 {
            self.music_market.saturation = self.music_market.saturation.saturating_sub(1);
        }
    }

    fn update_trends_with_timeline(&mut self, timeline: &MusicTimeline, rng: &mut impl Rng) {
        let trending_genres = timeline.get_trending_genres();
        if !trending_genres.is_empty() && rng.gen_range(0..10) == 0 {
            // 10% chance to align with historical trends
            self.current_trends = match trending_genres[0].as_str() {
                "Rock" => MusicTrend::Rock,
                "Pop" => MusicTrend::Pop,
                "Metal" | "Hard Rock" | "Hair Metal" => MusicTrend::Metal,
                "Punk" | "Post-Punk" => MusicTrend::Punk,
                "Alternative" | "Grunge" | "New Wave" => MusicTrend::Alternative,
                _ => MusicTrend::Electronic,
            };
        }
    }
    pub fn get_market_modifier(&self) -> f32 {
        let demand_mod = self.music_market.demand as f32 / 100.0;
        let saturation_penalty = 1.0 - (self.music_market.saturation as f32 / 200.0);
        let economic_mod = match self.music_market.economic_state {
            EconomicState::Recession => 0.7,
            EconomicState::Stagnant => 0.9,
            EconomicState::Growing => 1.1,
            EconomicState::Booming => 1.3,
        };

        demand_mod * saturation_penalty * economic_mod
    }
}

impl std::fmt::Display for MusicTrend {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            MusicTrend::Rock => write!(f, "Rock"),
            MusicTrend::Pop => write!(f, "Pop"),
            MusicTrend::Metal => write!(f, "Metal"),
            MusicTrend::Punk => write!(f, "Punk"),
            MusicTrend::Alternative => write!(f, "Alternative"),
            MusicTrend::Electronic => write!(f, "Electronic"),
        }
    }
}

impl std::fmt::Display for EconomicState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            EconomicState::Recession => write!(f, "Recession"),
            EconomicState::Stagnant => write!(f, "Stagnant"),
            EconomicState::Growing => write!(f, "Growing"),
            EconomicState::Booming => write!(f, "Booming"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::game::band::Band;
    use crate::game::timeline::MusicTimeline;
    use rand::SeedableRng;
    use rand::rngs::StdRng;

    #[test]
    fn worldgen_is_reproducible_per_seed() {
        let data = GameDataFiles::load().expect("data files present");
        let world_a = GameWorld::new(&data, &mut StdRng::seed_from_u64(99));
        let world_b = GameWorld::new(&data, &mut StdRng::seed_from_u64(99));
        let world_c = GameWorld::new(&data, &mut StdRng::seed_from_u64(100));

        let names = |w: &GameWorld| w.bands.iter().map(|b| b.name.clone()).collect::<Vec<_>>();
        assert_eq!(names(&world_a), names(&world_b), "same seed, same scene");
        assert_ne!(
            names(&world_a),
            names(&world_c),
            "different seed, different scene"
        );
        assert_eq!(world_a.bands.len(), SCENE_START_BANDS);
    }

    #[test]
    fn scene_has_hundreds_of_mostly_distinct_bands() {
        let data = GameDataFiles::load().expect("data files present");
        let world = GameWorld::new(&data, &mut StdRng::seed_from_u64(7));

        assert!(
            world.bands.len() >= 150,
            "the scene should start in the hundreds"
        );
        let distinct: std::collections::HashSet<_> =
            world.bands.iter().map(|b| b.name.as_str()).collect();
        assert!(
            distinct.len() as f32 >= world.bands.len() as f32 * 0.95,
            "band names should be nearly all distinct: {} of {}",
            distinct.len(),
            world.bands.len()
        );
    }

    #[test]
    fn scene_population_stays_bounded_and_charts_fill() {
        let data = GameDataFiles::load().expect("data files present");
        let timeline = MusicTimeline::new(&data);
        let mut world = GameWorld::new(&data, &mut StdRng::seed_from_u64(11));
        let mut rng = StdRng::seed_from_u64(12);
        let mut news_seen = 0;

        for _ in 0..300 {
            news_seen += world.update_week(&timeline, &data, &mut rng).len();
            let n = world.bands.len();
            assert!(
                (SCENE_MIN_BANDS..=SCENE_MAX_BANDS).contains(&n),
                "scene size {n} out of bounds"
            );
            assert!(world.charts.len() <= CHART_SIZE);
        }

        assert!(
            !world.charts.is_empty(),
            "a living scene keeps the charts full"
        );
        assert!(news_seen > 0, "300 weeks should produce scene news");
    }

    #[test]
    fn chart_submission_ranks_and_evicts() {
        let data = GameDataFiles::load().expect("data files present");
        let mut world = GameWorld::new(&data, &mut StdRng::seed_from_u64(5));

        for i in 0..CHART_SIZE {
            world.submit_chart_entry(
                format!("Filler {i}"),
                "Someone".into(),
                false,
                100 + i as u32,
            );
        }
        let pos = world.submit_chart_entry("Big Hit".into(), "You".into(), true, 5000);
        assert_eq!(pos, Some(1), "a huge score should enter at #1");
        let flop = world.submit_chart_entry("Flop".into(), "You".into(), true, 1);
        assert_eq!(flop, None, "a tiny score should miss a full chart");
        assert_eq!(world.charts.len(), CHART_SIZE);
    }

    #[test]
    fn rejected_deals_get_poached_by_the_biggest_unsigned_act() {
        let data = GameDataFiles::load().expect("data files present");
        let mut world = GameWorld::new(&data, &mut StdRng::seed_from_u64(21));
        for band in &mut world.bands {
            band.label = None;
        }
        let biggest = world
            .bands
            .iter()
            .max_by_key(|b| b.fame)
            .map(|b| b.name.clone())
            .unwrap();

        let mut rng = StdRng::seed_from_u64(3);
        let mut poached = None;
        for _ in 0..20 {
            poached = world.poach_rejected_deal("Apex Records", &mut rng);
            if poached.is_some() {
                break;
            }
        }
        assert_eq!(poached.as_deref(), Some(biggest.as_str()));
        let signed = world.bands.iter().find(|b| b.name == biggest).unwrap();
        assert_eq!(signed.label.as_deref(), Some("Apex Records"));
    }

    // --- Deal scouting: label tiers unlock along a real career arc ---

    use crate::game::music::{Release, ReleaseType};

    fn record(release_type: ReleaseType) -> Release {
        Release {
            id: 0,
            name: "Test Record".to_string(),
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

    /// A player band with the given fame and catalog.
    fn act(fame: u8, singles: usize, albums: usize) -> Band {
        Band {
            fame,
            singles_released: (0..singles).map(|_| record(ReleaseType::Single)).collect(),
            albums_released: (0..albums).map(|_| record(ReleaseType::Album)).collect(),
            ..Band::default()
        }
    }

    /// Which tiers ever bite across plenty of offer rolls. The threshold
    /// gates are deterministic — only the per-label coin flip is random —
    /// so enough rolls make an unlocked tier impossible to miss, while a
    /// locked tier stays silent on every single roll.
    fn tiers_scouting(
        world: &GameWorld,
        data: &GameDataFiles,
        band: &Band,
    ) -> std::collections::HashSet<String> {
        let mut rng = StdRng::seed_from_u64(77);
        let mut tiers = std::collections::HashSet::new();
        for _ in 0..40 {
            for offer in world.generate_deal_offers(band, data, &mut rng) {
                tiers.insert(offer.label_tier);
            }
        }
        tiers
    }

    #[test]
    fn independent_labels_scout_a_working_act_early() {
        let data = GameDataFiles::load().expect("data files present");
        let world = GameWorld::new(&data, &mut StdRng::seed_from_u64(31));

        // Local fame and a first single out: indies bite, nobody bigger.
        let tiers = tiers_scouting(&world, &data, &act(25, 1, 0));
        assert!(
            tiers.contains("Independent"),
            "a fame-25 act with a single out should draw indie interest, got {tiers:?}"
        );
        assert!(
            !tiers.contains("Boutique"),
            "boutiques should wait for the mid-career"
        );
        assert!(
            !tiers.contains("Major"),
            "majors do not scout the small clubs"
        );

        // The same noise with nothing on the shelves draws nobody at all.
        let all_talk = tiers_scouting(&world, &data, &act(30, 0, 0));
        assert!(
            all_talk.is_empty(),
            "no catalog, no offers, got {all_talk:?}"
        );

        // An act that went straight to an album is a record out all the
        // same — not invisible for lacking a 45.
        let album_first = tiers_scouting(&world, &data, &act(25, 0, 1));
        assert!(
            album_first.contains("Independent"),
            "an album counts as a record out, got {album_first:?}"
        );
    }

    #[test]
    fn boutique_labels_court_the_mid_career_act() {
        let data = GameDataFiles::load().expect("data files present");
        let world = GameWorld::new(&data, &mut StdRng::seed_from_u64(31));

        let tiers = tiers_scouting(&world, &data, &act(50, 2, 1));
        assert!(
            tiers.contains("Boutique"),
            "a fame-50 act with a small catalog should draw boutique interest, got {tiers:?}"
        );
        assert!(
            !tiers.contains("Major"),
            "majors should still be out of reach at fame 50"
        );
    }

    #[test]
    fn major_labels_only_sign_genuinely_big_acts() {
        let data = GameDataFiles::load().expect("data files present");
        let world = GameWorld::new(&data, &mut StdRng::seed_from_u64(31));

        // An album and three singles out, but fame 60: the majors keep watching.
        let almost = tiers_scouting(&world, &data, &act(60, 3, 1));
        assert!(
            !almost.contains("Major"),
            "fame 60 should not yet be enough for the majors, got {almost:?}"
        );
        assert!(
            almost.contains("Boutique"),
            "the mid tiers should be all over them though"
        );

        // Ten more points of fame and the phone rings. (Under the old
        // placeholder — buzz = fame/5 against a threshold of 30 — no major
        // could call even at fame 100.)
        let star = tiers_scouting(&world, &data, &act(70, 3, 1));
        assert!(
            star.contains("Major"),
            "a fame-70 act with a real catalog should finally draw a major, got {star:?}"
        );

        // But no album, no major — singles alone don't prove you can
        // deliver the LPs a major contract is made of.
        let no_album = tiers_scouting(&world, &data, &act(70, 4, 0));
        assert!(
            !no_album.contains("Major"),
            "majors require an album in the catalog, got {no_album:?}"
        );
    }

    #[test]
    fn buzz_weighs_the_catalog_and_a_charting_record() {
        let data = GameDataFiles::load().expect("data files present");
        let mut world = GameWorld::new(&data, &mut StdRng::seed_from_u64(31));

        // Same fame, deeper catalog: more buzz...
        assert!(world.band_buzz(&act(50, 2, 1)) > world.band_buzz(&act(50, 0, 0)));
        // ...but a shelf of records alone never adds up to stardom.
        assert_eq!(
            world.band_buzz(&act(0, 10, 10)),
            deals::BUZZ_CATALOG_CAP as u8,
            "the catalog contribution is capped"
        );

        // A record on this week's chart adds heat while it lasts.
        let star = act(60, 3, 1);
        let cold = world.band_buzz(&star);
        world.submit_chart_entry("The Hit".into(), "The Test Pattern".into(), true, 5_000);
        assert_eq!(world.band_buzz(&star), cold + deals::BUZZ_CHART_BONUS as u8);
    }
}
