//! Sales territories, the Local scene board, and the presence API that
//! gates who charts where (design §C).
//!
//! `ChartRegion` names every chart tab. `Local` is the home scene's board —
//! a UK **subset view**, not a territory: every scene band competes there
//! and its sales are already UK sales, so Local never feeds Worldwide.
//! `Uk` / `Europe` / `America` / `Japan` are the four stored, independently
//! decayed sales territories (`ChartRegion::TERRITORIES`). `Worldwide` is
//! never a storage key — it is derived on demand by summing the four
//! territories (see `GameWorld::worldwide_chart` in `charts.rs`) and exists
//! here only so the UI can cycle to it as a tab.
//!
//! This module owns the presence computation: which regions a *scene*
//! band's release reaches (`unsigned_spillover`, `signed_spread`) and the
//! territory filler that keeps four Top-100 boards fed without simulating
//! four scenes (`GameWorld::fill_territories`). M10 consumes
//! `ChartRegion`/`ChartRegion::TERRITORIES` for the player's own regional
//! presence and sales wiring in `economy.rs` — it does not edit this file.

use rand::Rng;
use serde::{Deserialize, Serialize};

use crate::data_loader::GameDataFiles;
use crate::game::genre::MusicGenre;
use crate::game::timeline::MusicTimeline;

use super::GameWorld;

/// A chart board / tab. `BTreeMap`-keyable (`Ord`) so `regional_charts`
/// iterates in a fixed, deterministic order regardless of insertion order.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum ChartRegion {
    Local,
    Uk,
    Europe,
    America,
    Japan,
    /// Derived aggregate — see the module docs. Never a `regional_charts`
    /// key; only ever used to select the Worldwide tab in the UI.
    Worldwide,
}

/// The territories an Independent-tier act can pick its "+1" from —
/// everywhere but the UK it already has.
const OTHER_TERRITORIES: [ChartRegion; 3] = [
    ChartRegion::Europe,
    ChartRegion::America,
    ChartRegion::Japan,
];

impl ChartRegion {
    /// The four sales territories that sum into Worldwide. Local is
    /// deliberately excluded — its sales already live inside Uk.
    pub const TERRITORIES: [ChartRegion; 4] = [
        ChartRegion::Uk,
        ChartRegion::Europe,
        ChartRegion::America,
        ChartRegion::Japan,
    ];

    /// Every chart tab, in the UI's `←/→` cycling order.
    pub const TAB_ORDER: [ChartRegion; 6] = [
        ChartRegion::Local,
        ChartRegion::Uk,
        ChartRegion::Europe,
        ChartRegion::America,
        ChartRegion::Japan,
        ChartRegion::Worldwide,
    ];

    pub fn label(&self) -> &'static str {
        match self {
            ChartRegion::Local => "Local",
            ChartRegion::Uk => "UK",
            ChartRegion::Europe => "Europe",
            ChartRegion::America => "America",
            ChartRegion::Japan => "Japan",
            ChartRegion::Worldwide => "Worldwide",
        }
    }

    /// The next tab, wrapping — `→` in the charts modal.
    pub fn next_tab(&self) -> ChartRegion {
        let idx = Self::TAB_ORDER.iter().position(|r| r == self).unwrap_or(0);
        Self::TAB_ORDER[(idx + 1) % Self::TAB_ORDER.len()]
    }

    /// The previous tab, wrapping — `←` in the charts modal.
    pub fn prev_tab(&self) -> ChartRegion {
        let idx = Self::TAB_ORDER.iter().position(|r| r == self).unwrap_or(0);
        let len = Self::TAB_ORDER.len();
        Self::TAB_ORDER[(idx + len - 1) % len]
    }
}

/// Regions an **unsigned** scene band's release reaches beyond Local:
/// nothing, until the act is famous enough nationally to spill over into
/// the UK sales territory (design §C — "unsigned acts chart Local, UK
/// spillover at fame ≥ 60").
pub fn unsigned_spillover(fame: u8) -> &'static [ChartRegion] {
    const SPILLOVER_FAME: u8 = 60;
    if fame >= SPILLOVER_FAME {
        &[ChartRegion::Uk]
    } else {
        &[]
    }
}

/// Regions a **signed** scene band's release reaches beyond Local, by
/// label tier (design §C): Boutique stays UK-only, Independent reaches UK
/// plus one further territory (picked from the band's own release roll,
/// so it stays reproducible for a given seed), Major blankets all four.
/// An unrecognised tier name is treated as Boutique — home turf only.
pub fn signed_spread(label_tier: &str, rng: &mut impl Rng) -> Vec<ChartRegion> {
    match label_tier {
        "Major" => ChartRegion::TERRITORIES.to_vec(),
        "Independent" => {
            let extra = OTHER_TERRITORIES[rng.gen_range(0..OTHER_TERRITORIES.len())];
            vec![ChartRegion::Uk, extra]
        }
        _ => vec![ChartRegion::Uk],
    }
}

/// Which label tier (if any) a scene band's label name belongs to, so its
/// releases can be spread by `signed_spread`. `None` for an unsigned band
/// or a label name absent from the data files (treated as unsigned).
pub fn label_tier_for<'a>(label_name: &str, data_files: &'a GameDataFiles) -> Option<&'a str> {
    let labels = data_files.get_record_labels_data();
    if labels.major_labels.iter().any(|l| l.name == label_name) {
        Some("Major")
    } else if labels
        .independent_labels
        .iter()
        .any(|l| l.name == label_name)
    {
        Some("Independent")
    } else if labels.boutique_labels.iter().any(|l| l.name == label_name) {
        Some("Boutique")
    } else {
        None
    }
}

/// How many ambient (chart-only, no band state) releases land in each
/// sales territory this week — four Top-100 boards can't be fed by one
/// city's scene (design §C).
const FILLER_MIN_PER_TERRITORY: u32 = 4;
const FILLER_MAX_PER_TERRITORY: u32 = 6;

impl GameWorld {
    /// Territory filler: name-generated foreign acts, chart-only, scored
    /// on the same scale as a scene release. Draws from the injected world
    /// RNG in the fixed `ChartRegion::TERRITORIES` order (never `HashMap`
    /// iteration) so a seed's worldgen stays reproducible.
    pub(super) fn fill_territories(
        &mut self,
        rng: &mut impl Rng,
        timeline: &MusicTimeline,
        data_files: &GameDataFiles,
    ) {
        let era_year = timeline.get_current_era().year;
        for region in ChartRegion::TERRITORIES {
            let count = rng.gen_range(FILLER_MIN_PER_TERRITORY..=FILLER_MAX_PER_TERRITORY);
            for _ in 0..count {
                let band_name = data_files.generate_band_name(rng);
                let title = data_files.generate_song_title(rng);
                let genre = MusicGenre::random(rng);
                let genre_mod = data_files.era_genre_modifier(era_year, genre.aliases());
                let fame_proxy = rng.gen_range(20..=90) as f32;
                let quality = rng.gen_range(25..=85) as f32;
                let score = ((fame_proxy * 1.2 + quality * 2.5)
                    * genre_mod
                    * rng.gen_range(0.9..1.3)) as u32;
                self.submit_chart_entry(region, title, band_name, false, score);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rand::SeedableRng;
    use rand::rngs::StdRng;

    #[test]
    fn unsigned_acts_only_spill_into_uk_once_famous() {
        assert!(unsigned_spillover(59).is_empty());
        assert_eq!(unsigned_spillover(60), &[ChartRegion::Uk]);
        assert_eq!(unsigned_spillover(100), &[ChartRegion::Uk]);
    }

    #[test]
    fn signed_spread_matches_label_tier() {
        let mut rng = StdRng::seed_from_u64(1);
        assert_eq!(signed_spread("Boutique", &mut rng), vec![ChartRegion::Uk]);
        assert_eq!(
            signed_spread("Unknown Tier", &mut rng),
            vec![ChartRegion::Uk]
        );

        let independent = signed_spread("Independent", &mut rng);
        assert_eq!(independent.len(), 2);
        assert_eq!(independent[0], ChartRegion::Uk);
        assert!(OTHER_TERRITORIES.contains(&independent[1]));

        let major = signed_spread("Major", &mut rng);
        assert_eq!(major.len(), 4);
        for territory in ChartRegion::TERRITORIES {
            assert!(major.contains(&territory));
        }
    }

    #[test]
    fn tab_order_cycles_and_wraps_both_ways() {
        assert_eq!(ChartRegion::Local.next_tab(), ChartRegion::Uk);
        assert_eq!(ChartRegion::Worldwide.next_tab(), ChartRegion::Local);
        assert_eq!(ChartRegion::Local.prev_tab(), ChartRegion::Worldwide);
        assert_eq!(ChartRegion::Uk.prev_tab(), ChartRegion::Local);
    }

    #[test]
    fn filler_is_seeded_and_deterministic() {
        let data = GameDataFiles::load().expect("data files present");
        let timeline = MusicTimeline::new(&data);
        let mut world_a = GameWorld::new(&data, &mut StdRng::seed_from_u64(50));
        let mut world_b = GameWorld::new(&data, &mut StdRng::seed_from_u64(50));
        let mut rng_a = StdRng::seed_from_u64(60);
        let mut rng_b = StdRng::seed_from_u64(60);

        world_a.fill_territories(&mut rng_a, &timeline, &data);
        world_b.fill_territories(&mut rng_b, &timeline, &data);

        for region in ChartRegion::TERRITORIES {
            let a = world_a
                .regional_charts
                .get(&region)
                .cloned()
                .unwrap_or_default();
            let b = world_b
                .regional_charts
                .get(&region)
                .cloned()
                .unwrap_or_default();
            assert!(!a.is_empty(), "{region:?} should have filler entries");
            let titles_a: Vec<_> = a.iter().map(|e| (e.title.clone(), e.score)).collect();
            let titles_b: Vec<_> = b.iter().map(|e| (e.title.clone(), e.score)).collect();
            assert_eq!(titles_a, titles_b, "same seed, same filler for {region:?}");
        }
    }
}
