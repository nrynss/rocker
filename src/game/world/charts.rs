//! Regional Top 100 charts (design §C) with a format axis (v0.9 §E): four
//! stored sales territories plus the home-scene Local board, each keeping a
//! **singles** Top 100 and an **albums** Top 100 that rank and decay
//! independently, and a Worldwide aggregate derived per format. A single
//! competes with singles and an album with albums — which is what a chart
//! peak has always meant in the real world, and what makes a cheap 45 a way
//! to buy chart presence instead of a doomed run at the album board.
//!
//! The board lifecycle itself is unchanged from §C: records ramp in, climb,
//! peak, and slide, instead of vanishing the instant one hot week produces
//! eleven better scores.

use std::cmp::Ordering;
use std::fmt;

use serde::de::{self, Deserializer, Visitor};
use serde::{Deserialize, Serialize, Serializer};

use super::GameWorld;
use super::regions::ChartRegion;
use crate::game::music::ReleaseType;

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

/// The separator between the two halves of a board's save key. Chosen
/// because no `ChartRegion` or `ReleaseType` variant name contains it, so
/// splitting a key back apart is unambiguous.
const BOARD_KEY_SEP: char = '/';

/// One stored board: a territory × a release format (§E).
///
/// §E asks for `regional_charts` keyed by `(ChartRegion, ReleaseType)`. A
/// literal tuple key is not available to us — saves are JSON, JSON object
/// keys must be strings, and `serde_json` rejects a tuple key outright.
/// Nesting a second map under each region would serialize, but it changes
/// the *shape* of the `regional_charts` value, which means a 0.8-era save
/// stops deserializing altogether instead of migrating. Flattening the pair
/// into a single string key (`"Uk/Album"`) keeps the field a flat map of
/// boards and lets the key parser carry the migration itself: a pre-0.9
/// save's bare `"Uk"` reads back as that region's albums board (see
/// [`ChartBoard::from_save_key`]), so there is no window in which a loaded
/// save is half-migrated.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ChartBoard {
    pub region: ChartRegion,
    pub format: ReleaseType,
}

impl ChartBoard {
    pub const fn new(region: ChartRegion, format: ReleaseType) -> Self {
        Self { region, format }
    }

    /// This board's save key — `"<Region>/<Format>"`, both halves spelled
    /// exactly as their serde variant names, so a key written by hand into
    /// a save file and one written by `serialize` agree.
    fn save_key(&self) -> String {
        format!(
            "{}{}{}",
            region_key(self.region),
            BOARD_KEY_SEP,
            format_key(self.format)
        )
    }

    /// Parse a `regional_charts` key off a save.
    ///
    /// A `"<Region>/<Format>"` key is a 0.9+ board and reads back verbatim.
    /// A **bare** `"<Region>"` key is a pre-0.9 save, from before the format
    /// axis existed: those entries land on that region's *albums* board,
    /// because the pre-cycle economy was album-dominated (§E). That is the
    /// same "an old field seeds the board it always really meant" move
    /// [`GameWorld::migrate_legacy_charts`] makes for the pre-0.5 flat
    /// top-10, one layer lower down — and doing it in the key parser means
    /// it cannot be forgotten by a caller that loads a save without asking
    /// for a migration pass.
    fn from_save_key(key: &str) -> Option<Self> {
        match key.split_once(BOARD_KEY_SEP) {
            Some((region, format)) => Some(Self::new(parse_region(region)?, parse_format(format)?)),
            None => Some(Self::new(parse_region(key)?, ReleaseType::Album)),
        }
    }
}

/// Sort index for the format axis. `ReleaseType` lives in `music.rs` and is
/// deliberately not made `Ord` from here, so `ChartBoard`'s ordering spells
/// the format order out itself: singles before albums, region first, which
/// keeps the `BTreeMap` iteration order (decay, filler, the label ledger's
/// sweep) every bit as deterministic as it was when the key was a bare
/// region.
const fn format_index(format: ReleaseType) -> u8 {
    match format {
        ReleaseType::Single => 0,
        ReleaseType::Album => 1,
    }
}

const fn format_key(format: ReleaseType) -> &'static str {
    match format {
        ReleaseType::Single => "Single",
        ReleaseType::Album => "Album",
    }
}

fn parse_format(key: &str) -> Option<ReleaseType> {
    match key {
        "Single" => Some(ReleaseType::Single),
        "Album" => Some(ReleaseType::Album),
        _ => None,
    }
}

const fn region_key(region: ChartRegion) -> &'static str {
    match region {
        ChartRegion::Local => "Local",
        ChartRegion::Uk => "Uk",
        ChartRegion::Europe => "Europe",
        ChartRegion::America => "America",
        ChartRegion::Japan => "Japan",
        ChartRegion::Worldwide => "Worldwide",
    }
}

fn parse_region(key: &str) -> Option<ChartRegion> {
    match key {
        "Local" => Some(ChartRegion::Local),
        "Uk" => Some(ChartRegion::Uk),
        "Europe" => Some(ChartRegion::Europe),
        "America" => Some(ChartRegion::America),
        "Japan" => Some(ChartRegion::Japan),
        // Never written as a storage key (Worldwide is derived), but a save
        // that somehow carries one should load rather than fail.
        "Worldwide" => Some(ChartRegion::Worldwide),
        _ => None,
    }
}

/// The format axis's display name — the modal's toggle and the chart news
/// lines share it so the two can never drift apart.
pub fn format_board_label(format: ReleaseType) -> &'static str {
    match format {
        ReleaseType::Single => "Singles",
        ReleaseType::Album => "Albums",
    }
}

impl Ord for ChartBoard {
    fn cmp(&self, other: &Self) -> Ordering {
        self.region
            .cmp(&other.region)
            .then_with(|| format_index(self.format).cmp(&format_index(other.format)))
    }
}

impl PartialOrd for ChartBoard {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Serialize for ChartBoard {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(&self.save_key())
    }
}

impl<'de> Deserialize<'de> for ChartBoard {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        struct KeyVisitor;

        impl Visitor<'_> for KeyVisitor {
            type Value = ChartBoard;

            fn expecting(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                f.write_str("a chart board key like \"Uk/Album\" (or a pre-0.9 \"Uk\")")
            }

            fn visit_str<E: de::Error>(self, value: &str) -> Result<Self::Value, E> {
                ChartBoard::from_save_key(value)
                    .ok_or_else(|| E::custom(format!("unknown chart board '{value}'")))
            }
        }

        deserializer.deserialize_str(KeyVisitor)
    }
}

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
    ///
    /// Which *format* of Local: albums, for the same reason a pre-0.9 save's
    /// regional entries migrate onto the albums boards (§E) — the pre-cycle
    /// economy was album-dominated, and an album board is the board those
    /// entries were competing on in all but name.
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
        self.regional_charts.insert(
            ChartBoard::new(ChartRegion::Local, ReleaseType::Album),
            seeded,
        );
    }

    /// One board's current standings, empty if nothing has ever charted
    /// there. The read half of the format axis — the UI, the panels, and
    /// the tests all go through here rather than indexing `regional_charts`
    /// with a hand-built key.
    pub fn chart_board(&self, region: ChartRegion, format: ReleaseType) -> &[ChartEntry] {
        self.regional_charts
            .get(&ChartBoard::new(region, format))
            .map(Vec::as_slice)
            .unwrap_or_default()
    }

    /// Whether the player currently has a charting record on any regional
    /// board (Local and both formats included) — the shared "am I in the
    /// charts right now" question a few other systems (fame decay pause,
    /// label buzz, the main-menu summary) key off.
    pub fn player_is_charting(&self) -> bool {
        self.regional_charts
            .values()
            .any(|entries| entries.iter().any(|e| e.is_player))
    }

    /// Advance every stored board (Local plus the four territories, each in
    /// both formats) by one week: ramp climbers, decay from week 2, re-rank,
    /// track peaks, evict below the floor or below rank 100. Pure score
    /// lifecycle — no special-cased eviction, no `is_player` favoritism in
    /// the mechanics (news is still player-only, since nobody wants
    /// scene-band spam). Boards never see each other: a hot single decays
    /// against other singles and can only ever evict a single (§E).
    pub(super) fn decay_charts(&mut self, news: &mut Vec<String>) {
        for (board, entries) in self.regional_charts.iter_mut() {
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
                news.push(slipped_off_line(entry, *board));
            }
            entries.retain(|e| e.score >= CHART_FLOOR_SCORE);
            // A player entry can also be pushed past rank 100 by a crowded
            // board while still above the floor — those are dropped by the
            // truncate below and would otherwise vanish silently. Report
            // them too. Floor-dropped entries were already removed by the
            // retain above, so there's no double-reporting.
            let depth_dropped: Vec<&ChartEntry> = entries
                .iter()
                .skip(CHART_DEPTH)
                .filter(|e| e.is_player)
                .collect();
            for entry in depth_dropped {
                news.push(slipped_off_line(entry, *board));
            }
            entries.truncate(CHART_DEPTH);
        }
    }

    /// Submit a release to one regional board, on its own format's half of
    /// that territory. Returns its 1-based position if it makes that board's
    /// Top 100. `region` must be a stored board — never
    /// `ChartRegion::Worldwide`, which is derived (see
    /// [`GameWorld::worldwide_chart`]) and is never a storage key.
    pub fn submit_chart_entry(
        &mut self,
        region: ChartRegion,
        format: ReleaseType,
        title: String,
        band_name: String,
        is_player: bool,
        score: u32,
    ) -> Option<usize> {
        let entries = self
            .regional_charts
            .entry(ChartBoard::new(region, format))
            .or_default();
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

    /// Worldwide: derived, never stored, and composed **per format** (§E) —
    /// the singles Worldwide is the four territories' singles boards summed,
    /// the albums Worldwide the four albums boards. The same release's
    /// effective scores summed across the four sales territories only (Local
    /// is a UK subset and never double-counts), re-ranked, top 100.
    /// Recompute fresh whenever asked — callers should call this after the
    /// weekly decay pass, same as any other read of the charts.
    pub fn worldwide_chart(&self, format: ReleaseType) -> Vec<ChartEntry> {
        use std::collections::HashMap;

        // (title, band_name) identifies "the same release" across boards.
        let mut totals: HashMap<(String, String), (u32, u32, u8, bool)> = HashMap::new();
        for region in ChartRegion::TERRITORIES {
            for entry in self.chart_board(region, format) {
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

/// The one "you've fallen off" news line, named by territory *and* format
/// now that a band can be sliding down the singles board while its album
/// holds (§E).
fn slipped_off_line(entry: &ChartEntry, board: ChartBoard) -> String {
    format!(
        "📉 '{}' slips off the {} {} chart after {} week{}.",
        entry.title,
        board.region.label(),
        format_board_label(board.format).to_ascii_lowercase(),
        entry.weeks_on_chart,
        if entry.weeks_on_chart == 1 { "" } else { "s" }
    )
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
        world.submit_chart_entry(
            ChartRegion::Uk,
            ReleaseType::Album,
            "Song".into(),
            "Band".into(),
            true,
            1000,
        );

        let board = |w: &GameWorld| w.chart_board(ChartRegion::Uk, ReleaseType::Album)[0].score;

        let entry_score = board(&world);
        assert_eq!(entry_score, 600, "entry week ramps to 0.6 of base");

        world.decay_charts(&mut news);
        let week1_score = board(&world);
        assert_eq!(week1_score, 850, "week 1 ramps to 0.85 of base");
        assert!(week1_score > entry_score, "still climbing");

        world.decay_charts(&mut news);
        let week2_score = board(&world);
        assert_eq!(
            week2_score,
            (1000.0f32 * CHART_DECAY) as u32,
            "week 2 ramps to peak and decay starts"
        );
        assert!(week2_score > week1_score, "peak week climbs past week 1");

        world.decay_charts(&mut news);
        let week3_score = board(&world);
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
                ReleaseType::Album,
                format!("Filler {i}"),
                "Someone".into(),
                false,
                1000 + i as u32,
            );
        }
        let depth = |w: &GameWorld| w.chart_board(ChartRegion::Europe, ReleaseType::Album).len();
        assert_eq!(depth(&world), CHART_DEPTH);

        // A tiny score can't crack a full, high-scoring board.
        let miss = world.submit_chart_entry(
            ChartRegion::Europe,
            ReleaseType::Album,
            "Nobody".into(),
            "Nobody".into(),
            false,
            1,
        );
        assert_eq!(miss, None);
        assert_eq!(depth(&world), CHART_DEPTH);

        // A huge score bumps the weakest entry off the bottom.
        let hit = world.submit_chart_entry(
            ChartRegion::Europe,
            ReleaseType::Album,
            "Smash".into(),
            "Star".into(),
            true,
            50_000,
        );
        assert_eq!(hit, Some(1));
        assert_eq!(depth(&world), CHART_DEPTH);
    }

    #[test]
    fn worldwide_sums_the_four_territories_and_excludes_local() {
        let mut world = fresh_world();
        world.submit_chart_entry(
            ChartRegion::Local,
            ReleaseType::Album,
            "Anthem".into(),
            "The Band".into(),
            true,
            900_000,
        );
        world.submit_chart_entry(
            ChartRegion::Uk,
            ReleaseType::Album,
            "Anthem".into(),
            "The Band".into(),
            true,
            1000,
        );
        world.submit_chart_entry(
            ChartRegion::Europe,
            ReleaseType::Album,
            "Anthem".into(),
            "The Band".into(),
            true,
            500,
        );
        world.submit_chart_entry(
            ChartRegion::Japan,
            ReleaseType::Album,
            "Solo Hit".into(),
            "Someone Else".into(),
            false,
            2000,
        );

        let worldwide = world.worldwide_chart(ReleaseType::Album);
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
        // §E: the pre-0.5 flat top-10 seeds Local's *albums* board.
        let local = game
            .world
            .chart_board(ChartRegion::Local, ReleaseType::Album);
        assert_eq!(local.len(), 10, "all ten legacy entries seed Local");
        assert!(
            game.world
                .chart_board(ChartRegion::Local, ReleaseType::Single)
                .is_empty(),
            "the singles board starts empty — a pre-0.9 save has no singles chart"
        );

        // Migrating again (e.g. a second load) must not double-seed.
        game.world.migrate_legacy_charts();
        assert_eq!(
            game.world
                .chart_board(ChartRegion::Local, ReleaseType::Album)
                .len(),
            10,
            "migration is idempotent"
        );
    }

    /// Decay re-ranks each board against itself (§E): a singles landslide
    /// leaves the albums board's ranks and peaks exactly where they were.
    #[test]
    fn decay_ranks_each_format_against_its_own_board() {
        let mut world = fresh_world();
        let mut news = Vec::new();
        for i in 0..5 {
            world.submit_chart_entry(
                ChartRegion::Uk,
                ReleaseType::Album,
                format!("Album {i}"),
                "Some Act".into(),
                false,
                1_000 + i as u32,
            );
            world.submit_chart_entry(
                ChartRegion::Uk,
                ReleaseType::Single,
                format!("Single {i}"),
                "Chart Monster".into(),
                false,
                500_000,
            );
        }

        world.decay_charts(&mut news);

        let albums = world.chart_board(ChartRegion::Uk, ReleaseType::Album);
        assert_eq!(albums.len(), 5, "no album was evicted by a single");
        assert_eq!(
            albums[0].title, "Album 4",
            "the biggest album tops the albums board"
        );
        assert_eq!(
            albums[0].peak_position, 1,
            "and its peak is #1 — the singles' scores never entered the ranking"
        );
    }

    #[test]
    fn board_keys_round_trip_and_sort_region_then_format() {
        for region in ChartRegion::TAB_ORDER {
            for format in [ReleaseType::Single, ReleaseType::Album] {
                let board = ChartBoard::new(region, format);
                let key = board.save_key();
                assert_eq!(
                    ChartBoard::from_save_key(&key),
                    Some(board),
                    "'{key}' should round-trip"
                );
            }
        }
        assert_eq!(ChartBoard::from_save_key("Uk/Nonsense"), None);
        assert_eq!(ChartBoard::from_save_key("Atlantis"), None);

        // Region first, singles before albums — the order `decay_charts` and
        // the label ledger's sweep walk the map in.
        assert!(
            ChartBoard::new(ChartRegion::Local, ReleaseType::Album)
                < ChartBoard::new(ChartRegion::Uk, ReleaseType::Single)
        );
        assert!(
            ChartBoard::new(ChartRegion::Uk, ReleaseType::Single)
                < ChartBoard::new(ChartRegion::Uk, ReleaseType::Album)
        );
    }
}
