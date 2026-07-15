//! Regional Top 100 charts (design §C): four stored, independently-decayed
//! sales territories plus the home-scene Local board, and a derived
//! Worldwide aggregate. Replaces the old single hard-`truncate`d top 10 —
//! records now ramp in, climb, peak, and slide, instead of vanishing the
//! instant one hot week produces eleven better scores.

use serde::{Deserialize, Serialize};

use super::GameWorld;
use super::regions::ChartRegion;

/// Board depth: eviction only below rank 100 (plus the score floor below).
pub const CHART_DEPTH: usize = 100;

/// Weekly compounding decay, active from week 2 onward.
const CHART_DECAY: f32 = 0.92;
/// Below this effective score an entry drops off its board, at any rank.
const CHART_FLOOR_SCORE: u32 = 25;

/// Ramp-in multipliers, keyed by `weeks_on_chart` (design §C): a record
/// debuts mid-chart, climbs for two weeks, then decay takes over.
const RAMP_ENTRY: f32 = 0.6; // weeks_on_chart == 0, at submission
const RAMP_WEEK1: f32 = 0.85; // weeks_on_chart == 1

/// One record on a regional Top 100.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChartEntry {
    pub title: String,
    pub band_name: String,
    pub is_player: bool,
    /// Current effective score: `base_score × ramp × decay`. Drives rank.
    pub score: u32,
    pub weeks_on_chart: u32,
    /// The release's underlying score, before ramp-in/decay. Old saves
    /// (pre-regional-charts) default to 0 and are backfilled at migration.
    #[serde(default)]
    pub base_score: u32,
    /// Best (lowest-numbered) rank this entry has ever held on its board.
    /// 0 means "never ranked yet" (fresh entry, or a pre-ramp-in save).
    #[serde(default)]
    pub peak_position: u8,
}

impl ChartEntry {
    fn note_position(&mut self, position: usize) {
        let position = position.min(u8::MAX as usize) as u8;
        if self.peak_position == 0 || position < self.peak_position {
            self.peak_position = position;
        }
    }
}

impl GameWorld {
    /// One-time migration for a save from before regional charts: the old
    /// flat top-10 was always the home scene's board, so it seeds Local
    /// verbatim (entries get treated as already fully ramped-in, since
    /// pre-migration saves have no ramp state to recover). The legacy
    /// `charts` field is drained and never written to again.
    pub fn migrate_legacy_charts(&mut self) {
        if !self.regional_charts.is_empty() || self.charts.is_empty() {
            return;
        }
        let seeded: Vec<ChartEntry> = std::mem::take(&mut self.charts)
            .into_iter()
            .map(|mut entry| {
                if entry.base_score == 0 {
                    entry.base_score = entry.score;
                }
                entry
            })
            .collect();
        self.regional_charts.insert(ChartRegion::Local, seeded);
    }

    /// Whether the player currently has a charting record on any regional
    /// board (Local included) — the shared "am I in the charts right now"
    /// question a few other systems (fame decay pause, label buzz, the
    /// main-menu summary) key off.
    pub fn player_is_charting(&self) -> bool {
        self.regional_charts
            .values()
            .any(|entries| entries.iter().any(|e| e.is_player))
    }

    /// Advance every stored board (Local plus the four territories) by one
    /// week: ramp climbers, decay from week 2, re-rank, track peaks, evict
    /// below the floor or below rank 100. Pure score lifecycle — no
    /// special-cased eviction, no `is_player` favoritism in the mechanics
    /// (news is still player-only, since nobody wants scene-band spam).
    pub(super) fn decay_charts(&mut self, news: &mut Vec<String>) {
        for (region, entries) in self.regional_charts.iter_mut() {
            for entry in entries.iter_mut() {
                entry.weeks_on_chart += 1;
                entry.score = match entry.weeks_on_chart {
                    1 => (entry.base_score as f32 * RAMP_WEEK1) as u32,
                    2 => (entry.base_score as f32 * CHART_DECAY) as u32,
                    _ => (entry.score as f32 * CHART_DECAY) as u32,
                };
            }
            entries.sort_by_key(|e| std::cmp::Reverse(e.score));
            for (idx, entry) in entries.iter_mut().enumerate() {
                entry.note_position(idx + 1);
            }

            let dropped: Vec<&ChartEntry> = entries
                .iter()
                .filter(|e| e.is_player && e.score < CHART_FLOOR_SCORE)
                .collect();
            for entry in dropped {
                news.push(format!(
                    "📉 '{}' slips off the {} chart after {} week{}.",
                    entry.title,
                    region.label(),
                    entry.weeks_on_chart,
                    if entry.weeks_on_chart == 1 { "" } else { "s" }
                ));
            }
            entries.retain(|e| e.score >= CHART_FLOOR_SCORE);
            entries.truncate(CHART_DEPTH);
        }
    }

    /// Submit a release to one regional board. Returns its 1-based
    /// position if it makes that board's Top 100. `region` must be a
    /// stored board — never `ChartRegion::Worldwide`, which is derived
    /// (see [`GameWorld::worldwide_chart`]) and is never a storage key.
    pub fn submit_chart_entry(
        &mut self,
        region: ChartRegion,
        title: String,
        band_name: String,
        is_player: bool,
        score: u32,
    ) -> Option<usize> {
        let entries = self.regional_charts.entry(region).or_default();
        entries.push(ChartEntry {
            title: title.clone(),
            band_name: band_name.clone(),
            is_player,
            score: (score as f32 * RAMP_ENTRY) as u32,
            weeks_on_chart: 0,
            base_score: score,
            peak_position: 0,
        });
        entries.sort_by_key(|e| std::cmp::Reverse(e.score));
        entries.retain(|e| e.score >= CHART_FLOOR_SCORE);
        entries.truncate(CHART_DEPTH);

        let position = entries
            .iter()
            .position(|e| e.weeks_on_chart == 0 && e.title == title && e.band_name == band_name)
            .map(|i| i + 1);
        if let Some(pos) = position {
            entries[pos - 1].note_position(pos);
        }
        position
    }

    /// Worldwide: derived, never stored. The same release's effective
    /// scores summed across the four sales territories only (Local is a
    /// UK subset and never double-counts), re-ranked, top 100. Recompute
    /// fresh whenever asked — callers should call this after the weekly
    /// decay pass, same as any other read of the charts.
    pub fn worldwide_chart(&self) -> Vec<ChartEntry> {
        use std::collections::HashMap;

        // (title, band_name) identifies "the same release" across boards.
        let mut totals: HashMap<(String, String), (u32, u32, u8, bool)> = HashMap::new();
        for region in ChartRegion::TERRITORIES {
            let Some(entries) = self.regional_charts.get(&region) else {
                continue;
            };
            for entry in entries {
                let key = (entry.title.clone(), entry.band_name.clone());
                let slot = totals.entry(key).or_insert((0, 0, 0, entry.is_player));
                slot.0 += entry.score;
                slot.1 = slot.1.max(entry.weeks_on_chart);
                slot.2 = slot.2.max(entry.peak_position);
            }
        }

        let mut list: Vec<ChartEntry> = totals
            .into_iter()
            .map(
                |((title, band_name), (score, weeks_on_chart, peak_position, is_player))| {
                    ChartEntry {
                        title,
                        band_name,
                        is_player,
                        score,
                        weeks_on_chart,
                        base_score: score,
                        peak_position,
                    }
                },
            )
            .collect();
        // Stable, fully deterministic order: score descending, then a
        // (title, band_name) tiebreak. The totals came from a `HashMap`,
        // whose iteration order is randomized per process, so sorting on
        // score alone would let tied entries — and thus which ones survive
        // the depth-100 truncation — vary between runs and even between
        // renders. The tiebreak pins the order regardless.
        list.sort_by(|a, b| {
            b.score
                .cmp(&a.score)
                .then_with(|| a.title.cmp(&b.title))
                .then_with(|| a.band_name.cmp(&b.band_name))
        });
        list.truncate(CHART_DEPTH);
        for (idx, entry) in list.iter_mut().enumerate() {
            entry.note_position(idx + 1);
        }
        list
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::data_loader::GameDataFiles;
    use rand::SeedableRng;
    use rand::rngs::StdRng;

    fn fresh_world() -> GameWorld {
        let data = GameDataFiles::load().expect("data files present");
        GameWorld::new(&data, &mut StdRng::seed_from_u64(1))
    }

    #[test]
    fn ramp_in_climbs_then_decay_takes_over() {
        let mut world = fresh_world();
        let mut news = Vec::new();
        world.submit_chart_entry(ChartRegion::Uk, "Song".into(), "Band".into(), true, 1000);

        let entry_score = world.regional_charts[&ChartRegion::Uk][0].score;
        assert_eq!(entry_score, 600, "entry week ramps to 0.6 of base");

        world.decay_charts(&mut news);
        let week1_score = world.regional_charts[&ChartRegion::Uk][0].score;
        assert_eq!(week1_score, 850, "week 1 ramps to 0.85 of base");
        assert!(week1_score > entry_score, "still climbing");

        world.decay_charts(&mut news);
        let week2_score = world.regional_charts[&ChartRegion::Uk][0].score;
        assert_eq!(
            week2_score,
            (1000.0f32 * CHART_DECAY) as u32,
            "week 2 ramps to peak and decay starts"
        );
        assert!(week2_score > week1_score, "peak week climbs past week 1");

        world.decay_charts(&mut news);
        let week3_score = world.regional_charts[&ChartRegion::Uk][0].score;
        assert!(
            week3_score < week2_score,
            "decay slides it back down after the peak"
        );
        assert_eq!(week3_score, (week2_score as f32 * CHART_DECAY) as u32);
    }

    #[test]
    fn depth_100_evicts_only_below_rank_100() {
        let mut world = fresh_world();
        for i in 0..CHART_DEPTH {
            world.submit_chart_entry(
                ChartRegion::Europe,
                format!("Filler {i}"),
                "Someone".into(),
                false,
                1000 + i as u32,
            );
        }
        assert_eq!(
            world.regional_charts[&ChartRegion::Europe].len(),
            CHART_DEPTH
        );

        // A tiny score can't crack a full, high-scoring board.
        let miss = world.submit_chart_entry(
            ChartRegion::Europe,
            "Nobody".into(),
            "Nobody".into(),
            false,
            1,
        );
        assert_eq!(miss, None);
        assert_eq!(
            world.regional_charts[&ChartRegion::Europe].len(),
            CHART_DEPTH
        );

        // A huge score bumps the weakest entry off the bottom.
        let hit = world.submit_chart_entry(
            ChartRegion::Europe,
            "Smash".into(),
            "Star".into(),
            true,
            50_000,
        );
        assert_eq!(hit, Some(1));
        assert_eq!(
            world.regional_charts[&ChartRegion::Europe].len(),
            CHART_DEPTH
        );
    }

    #[test]
    fn worldwide_sums_the_four_territories_and_excludes_local() {
        let mut world = fresh_world();
        world.submit_chart_entry(
            ChartRegion::Local,
            "Anthem".into(),
            "The Band".into(),
            true,
            900_000,
        );
        world.submit_chart_entry(
            ChartRegion::Uk,
            "Anthem".into(),
            "The Band".into(),
            true,
            1000,
        );
        world.submit_chart_entry(
            ChartRegion::Europe,
            "Anthem".into(),
            "The Band".into(),
            true,
            500,
        );
        world.submit_chart_entry(
            ChartRegion::Japan,
            "Solo Hit".into(),
            "Someone Else".into(),
            false,
            2000,
        );

        let worldwide = world.worldwide_chart();
        let anthem = worldwide
            .iter()
            .find(|e| e.title == "Anthem")
            .expect("anthem should be on the derived board");
        // Uk (600 after entry ramp) + Europe (300 after entry ramp) — Local's
        // 900,000 must never leak in.
        assert_eq!(anthem.score, 600 + 300);

        let solo = worldwide.iter().find(|e| e.title == "Solo Hit").unwrap();
        assert_eq!(solo.score, 1200);
    }

    #[test]
    fn old_save_charts_seed_local_then_stay_empty() {
        let path = concat!(env!("CARGO_MANIFEST_DIR"), "/tests/fixtures/pre-0.5.sav");
        let mut game = crate::game::Game::load_game(path).expect("v0.4.0 save loads");
        game.world.migrate_legacy_charts();

        assert!(game.world.charts.is_empty(), "legacy field is drained");
        let local = game
            .world
            .regional_charts
            .get(&ChartRegion::Local)
            .expect("Local should be seeded from the legacy chart");
        assert_eq!(local.len(), 10, "all ten legacy entries seed Local");

        // Migrating again (e.g. a second load) must not double-seed.
        game.world.migrate_legacy_charts();
        assert_eq!(
            game.world.regional_charts[&ChartRegion::Local].len(),
            10,
            "migration is idempotent"
        );
    }
}
