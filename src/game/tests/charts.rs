//! The format axis on the regional charts (v0.9 §E): a singles Top 100 and
//! an albums Top 100 per territory, ranked and decayed independently, a
//! Worldwide aggregate composed per format, and the save migration that
//! lands a pre-0.9 board on the albums side.

use rand::SeedableRng;
use rand::rngs::StdRng;

use crate::data_loader::GameDataFiles;
use crate::game::world::{CHART_DEPTH, ChartBoard, ChartRegion, GameWorld};

use super::*;

fn fresh_world(seed: u64) -> GameWorld {
    let data = GameDataFiles::load().expect("data files present");
    GameWorld::new(&data, &mut StdRng::seed_from_u64(seed))
}

/// The point of §E: the two boards never see each other. A run of monster
/// singles cannot push a single album off the albums chart, however hot the
/// singles are — before the split, the same scores would have evicted the
/// whole album board twice over.
#[test]
fn a_hot_single_never_evicts_an_album() {
    let mut world = fresh_world(1);

    // A full albums board of modest, entirely evictable records.
    for i in 0..CHART_DEPTH {
        world.submit_chart_entry(
            ChartRegion::Uk,
            ReleaseType::Album,
            format!("Album {i}"),
            "Some Act".into(),
            false,
            200 + i as u32,
        );
    }
    let albums_before: Vec<String> = world
        .chart_board(ChartRegion::Uk, ReleaseType::Album)
        .iter()
        .map(|e| e.title.clone())
        .collect();
    assert_eq!(albums_before.len(), CHART_DEPTH);

    // Half again as many singles, each an order of magnitude bigger.
    for i in 0..CHART_DEPTH + 50 {
        world.submit_chart_entry(
            ChartRegion::Uk,
            ReleaseType::Single,
            format!("Single {i}"),
            "Chart Monster".into(),
            true,
            50_000 + i as u32,
        );
    }

    let albums_after: Vec<String> = world
        .chart_board(ChartRegion::Uk, ReleaseType::Album)
        .iter()
        .map(|e| e.title.clone())
        .collect();
    assert_eq!(
        albums_before, albums_after,
        "the albums board is untouched by a singles landslide"
    );
    assert_eq!(
        world
            .chart_board(ChartRegion::Uk, ReleaseType::Single)
            .len(),
        CHART_DEPTH,
        "the singles board evicted its own, and only its own"
    );
}

/// A release's peak is its peak *on its own format's board* (§E) — the
/// real-world meaning of a chart peak. A single released into a market whose
/// album chart is stacked with monsters still enters its own chart at #1.
#[test]
fn peak_chart_position_is_tracked_per_format() {
    let mut game = test_game();
    game.initialize_player("Test", "The Tests", genre::MusicGenre::Rock);

    // A Local albums board nobody could crack...
    for i in 0..20 {
        game.world.submit_chart_entry(
            ChartRegion::Local,
            ReleaseType::Album,
            format!("Untouchable {i}"),
            "The Giants".into(),
            false,
            500_000,
        );
    }

    game.band.fame = 80;
    let mut single = test_release(1, ReleaseType::Single);
    single.name = "B-Side Wins Again".to_string();
    single.release_quality = 90;
    game.just_released_music.push(single);
    game.week = INITIAL_SALES_WINDOW_WEEKS;
    game.process_music_releases_and_marketing();

    let released = game
        .band
        .singles_released
        .iter()
        .find(|r| r.name == "B-Side Wins Again")
        .expect("the single is in the catalogue");
    assert_eq!(
        released.peak_chart_position,
        Some(1),
        "a single is judged against singles, not against a stacked album chart"
    );
    assert!(
        game.world
            .chart_board(ChartRegion::Local, ReleaseType::Single)
            .iter()
            .any(|e| e.is_player && e.title == "B-Side Wins Again"),
        "and it is on the singles board, not the albums one"
    );
    assert!(
        !game
            .world
            .chart_board(ChartRegion::Local, ReleaseType::Album)
            .iter()
            .any(|e| e.is_player),
        "a single never appears on an albums board"
    );
}

/// Worldwide is still the four territories summed and re-ranked — composed
/// once per format, so a band's single and album aggregate separately even
/// when they share a name.
#[test]
fn the_worldwide_aggregate_composes_per_format() {
    let mut world = fresh_world(2);

    // Same title on both formats (a title track cut as a single), charting
    // in two territories each with different scores.
    for (format, uk, europe) in [
        (ReleaseType::Single, 1000u32, 500u32),
        (ReleaseType::Album, 3000, 2000),
    ] {
        world.submit_chart_entry(
            ChartRegion::Uk,
            format,
            "Title Track".into(),
            "The Band".into(),
            true,
            uk,
        );
        world.submit_chart_entry(
            ChartRegion::Europe,
            format,
            "Title Track".into(),
            "The Band".into(),
            true,
            europe,
        );
    }
    // Local must still never leak into the aggregate, on either board.
    world.submit_chart_entry(
        ChartRegion::Local,
        ReleaseType::Single,
        "Title Track".into(),
        "The Band".into(),
        true,
        900_000,
    );

    let singles = world.worldwide_chart(ReleaseType::Single);
    let albums = world.worldwide_chart(ReleaseType::Album);

    // Entry-week ramp is 0.6, so the sums are 600+300 and 1800+1200.
    assert_eq!(
        singles
            .iter()
            .find(|e| e.title == "Title Track")
            .expect("the single aggregates")
            .score,
        600 + 300
    );
    assert_eq!(
        albums
            .iter()
            .find(|e| e.title == "Title Track")
            .expect("the album aggregates")
            .score,
        1800 + 1200
    );
}

/// The 0.8-era save shape: `regional_charts` keyed by a bare region name.
/// Those boards were album-dominated, so they land on the albums side (§E)
/// — and every scrap of ramp/decay state (`base_score`, `weeks_on_chart`,
/// `peak_position`) survives the trip, because a record mid-run must keep
/// decaying from where it was rather than restarting its climb.
#[test]
fn a_0_8_era_save_migrates_its_boards_onto_the_albums_side() {
    let world = fresh_world(3);
    let mut value = serde_json::to_value(&world).expect("world serializes");
    value["regional_charts"] = serde_json::json!({
        "Local": [{
            "title": "Home Turf", "band_name": "The Locals", "is_player": true,
            "score": 640, "weeks_on_chart": 3, "base_score": 900, "peak_position": 2
        }],
        "Uk": [{
            "title": "Big Country", "band_name": "The Nationals", "is_player": false,
            "score": 480, "weeks_on_chart": 7, "base_score": 700, "peak_position": 11
        }]
    });

    let loaded: GameWorld = serde_json::from_value(value).expect("a 0.8 save must still load");

    let local = loaded.chart_board(ChartRegion::Local, ReleaseType::Album);
    assert_eq!(local.len(), 1, "the Local board migrated to Local albums");
    assert_eq!(local[0].title, "Home Turf");
    assert_eq!(local[0].base_score, 900, "base_score survives");
    assert_eq!(local[0].weeks_on_chart, 3, "weeks_on_chart survives");
    assert_eq!(local[0].peak_position, 2, "peak_position survives");
    assert_eq!(local[0].score, 640, "the effective score survives");

    let uk = loaded.chart_board(ChartRegion::Uk, ReleaseType::Album);
    assert_eq!(uk.len(), 1);
    assert_eq!(uk[0].base_score, 700);
    assert_eq!(uk[0].weeks_on_chart, 7);
    assert_eq!(uk[0].peak_position, 11);

    for region in [ChartRegion::Local, ChartRegion::Uk] {
        assert!(
            loaded.chart_board(region, ReleaseType::Single).is_empty(),
            "{region:?} has no singles board — a pre-0.9 save never had one"
        );
    }

    // The pre-0.5 migration must not fire on top of it: `regional_charts`
    // is populated, so the legacy top-10 seeder stays out of the way.
    let mut loaded = loaded;
    loaded.migrate_legacy_charts();
    assert_eq!(
        loaded
            .chart_board(ChartRegion::Local, ReleaseType::Album)
            .len(),
        1,
        "a 0.8 save's boards are not re-seeded by the pre-0.5 migration"
    );

    // And a 0.9 board round-trips through the new key without migrating.
    let json = serde_json::to_string(&loaded).expect("serializes");
    assert!(
        json.contains("Local/Album"),
        "boards write the composite key: {json:.400}"
    );
    let round_tripped: GameWorld = serde_json::from_str(&json).expect("round-trips");
    assert_eq!(
        round_tripped.regional_charts.keys().collect::<Vec<_>>(),
        loaded.regional_charts.keys().collect::<Vec<_>>()
    );
}

/// Both migrations, in the order a real load performs them. The pre-0.5
/// fixture carries only the flat `charts` top-10 and no `regional_charts`,
/// so nothing is keyed and the key-parser migration is a no-op; then
/// `load_game`'s legacy seeder runs and puts the top-10 on Local's **albums**
/// board — the same destination the 0.8 shape migrates to, which is what
/// keeps a save that has been through both from ending up with two
/// half-boards of the same records.
#[test]
fn the_pre_0_5_and_0_8_migrations_compose() {
    let path = concat!(env!("CARGO_MANIFEST_DIR"), "/tests/fixtures/pre-0.5.sav");
    let mut game = Game::load_game(path).expect("a v0.4.0 save must keep loading");

    assert!(
        game.world.charts.is_empty(),
        "the legacy flat top-10 is drained by the load-time migration"
    );

    // Idempotent: a second pass (a re-load of an already-migrated save)
    // changes nothing.
    game.world.migrate_legacy_charts();

    let keys: Vec<&ChartBoard> = game.world.regional_charts.keys().collect();
    assert_eq!(
        keys,
        vec![&ChartBoard::new(ChartRegion::Local, ReleaseType::Album)],
        "exactly one board is seeded, and it is Local albums"
    );
    assert_eq!(
        game.world
            .chart_board(ChartRegion::Local, ReleaseType::Album)
            .len(),
        10
    );
    // Every seeded entry got its `base_score` backfilled, so the ramp/decay
    // pass has something to work from on the very next week.
    assert!(
        game.world
            .chart_board(ChartRegion::Local, ReleaseType::Album)
            .iter()
            .all(|e| e.base_score > 0),
        "base_score is backfilled from the legacy score"
    );
}
