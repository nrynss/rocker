//! Writing and recording: songwriting quality (creativity-driven, happiness
//! multiplier), recording quality, stress economy, streak tracking, and — from
//! S-RECORD (v0.9 §A/§C) — song-count bounds, best-N song selection, per-song
//! studio pricing, the studio ladder's gates and bonuses, and the booking quote.

use super::*;
use crate::game::music::DistributionChannel;
use crate::game::studios::{self, RecordingStudio};
use rand::SeedableRng;
use rand::rngs::StdRng;

/// Bank `count` songs of the given quality, ids continuing from whatever is
/// already in the notebook.
fn bank_songs(game: &mut Game, count: u32, quality: u8) {
    for _ in 0..count {
        let id = game.next_song_id;
        game.band.unreleased_songs.push(music::Song {
            id,
            name: format!("Song {id}"),
            songwriting_quality: quality,
        });
        game.next_song_id += 1;
    }
}

/// A song of a named quality, so a test can build a notebook whose ordering
/// and quality disagree.
fn bank_song(game: &mut Game, name: &str, quality: u8) {
    let id = game.next_song_id;
    game.band.unreleased_songs.push(music::Song {
        id,
        name: name.to_string(),
        songwriting_quality: quality,
    });
    game.next_song_id += 1;
}

/// A game that can afford anything and is famous enough for every room.
fn well_funded_game() -> Game {
    let mut game = test_game();
    game.initialize_player("Test", "Band", genre::MusicGenre::Rock);
    game.player.money = 10_000_000;
    game.player.stress = 0;
    game.band.fame = 100;
    game
}

#[test]
fn high_creativity_produces_better_songs_than_low_creativity() {
    // Two games, same seed and circumstances, varying only in creativity.
    let mut game_creative = test_game();
    game_creative.initialize_player("Test", "Band", genre::MusicGenre::Rock);
    game_creative.player.creativity = 100;
    game_creative.player.happiness = 50;
    game_creative.player.stress = 0;

    let mut game_dull = test_game();
    game_dull.initialize_player("Test", "Band", genre::MusicGenre::Rock);
    game_dull.player.creativity = 0;
    game_dull.player.happiness = 50;
    game_dull.player.stress = 0;

    // Both write with the same RNG seed.
    let seed = 42u64;
    let mut rng_creative = StdRng::seed_from_u64(seed);
    let mut rng_dull = StdRng::seed_from_u64(seed);

    // Manually call the quality calculation to test the formula (rather than
    // full write actions, which involve RNG for song count and titles).
    let quality_creative = game_creative.calculate_songwriting_quality(&mut rng_creative);
    let quality_dull = game_dull.calculate_songwriting_quality(&mut rng_dull);

    assert!(
        quality_creative > quality_dull,
        "high creativity (100) should yield better songs than zero: {} vs {}",
        quality_creative,
        quality_dull
    );
}

#[test]
fn happiness_multiplier_shifts_quality_by_the_0_8_factor() {
    let mut game_sad = test_game();
    game_sad.initialize_player("Test", "Band", genre::MusicGenre::Rock);
    game_sad.player.creativity = 50;
    game_sad.player.stress = 0;
    game_sad.player.happiness = 0; // min multiplier: 0.8

    let mut game_happy = test_game();
    game_happy.initialize_player("Test", "Band", genre::MusicGenre::Rock);
    game_happy.player.creativity = 50;
    game_happy.player.stress = 0;
    game_happy.player.happiness = 100; // max multiplier: 1.0

    let seed = 42u64;
    let mut rng_sad = StdRng::seed_from_u64(seed);
    let mut rng_happy = StdRng::seed_from_u64(seed);

    let quality_sad = game_sad.calculate_songwriting_quality(&mut rng_sad);
    let quality_happy = game_happy.calculate_songwriting_quality(&mut rng_happy);

    // The happiness multiplier ranges from 0.8 to 1.0 — a 20% spread.
    // At sad (0.8x) vs happy (1.0x), the happy version should be about 1.25x better.
    let ratio = quality_happy as f32 / quality_sad as f32;
    assert!(
        ratio > 1.15 && ratio < 1.3,
        "happiness multiplier (1.0 / 0.8 = 1.25) should shift quality: ratio {}",
        ratio
    );
}

#[test]
fn third_consecutive_write_week_costs_creativity_but_first_two_dont() {
    let mut game = test_game();
    game.initialize_player("Test", "Band", genre::MusicGenre::Rock);
    game.player.stress = 0; // No stress-driven creativity drain to confound the test.
    game.player.creativity = 100; // Plenty of creativity to consume.
    game.player.money = 10_000; // Enough to sustain operations.
    game.band.fame = 10;

    // Week 1: write songs (no creativity drain expected on streak < 3).
    let mut rng = StdRng::seed_from_u64(100);
    game.action_write_songs(&mut rng)
        .expect("write should work");
    let creativity_after_week1 = game.player.creativity;
    assert_eq!(
        creativity_after_week1, 100,
        "week 1 of writing should not drain creativity (no stress, streak 1 < threshold 3)"
    );

    // Week 2: write again (still no drain).
    game.update_lifestyle(&GameAction::WriteSongs); // Reset does nothing for writing
    game.action_write_songs(&mut rng)
        .expect("write should work");
    let creativity_after_week2 = game.player.creativity;
    assert_eq!(
        creativity_after_week2, 100,
        "week 2 of writing should not drain creativity (streak 2 < threshold 3)"
    );

    // Week 3: write yet again (NOW fatigue kicks in).
    game.update_lifestyle(&GameAction::WriteSongs); // Reset does nothing for writing
    game.action_write_songs(&mut rng)
        .expect("write should work");
    let creativity_after_week3 = game.player.creativity;
    assert!(
        creativity_after_week3 < creativity_after_week2,
        "week 3+ of writing should drain creativity: {} vs {}",
        creativity_after_week3,
        creativity_after_week2
    );
    assert_eq!(
        creativity_after_week3,
        creativity_after_week2.saturating_sub(constants::WRITING_FATIGUE_CREATIVITY_COST),
        "week 3 should cost exactly WRITING_FATIGUE_CREATIVITY_COST"
    );
}

#[test]
fn writing_under_stress_costs_creativity() {
    let mut game = test_game();
    game.initialize_player("Test", "Band", genre::MusicGenre::Rock);
    game.player.creativity = 100;
    game.player.stress = 60; // Above the WRITING_STRESS_CREATIVITY_THRESHOLD (50).
    game.player.money = 10_000;
    game.band.fame = 10;
    game.writing_streak = 0; // No fatigue drain (streak < 3).

    let mut rng = StdRng::seed_from_u64(200);
    game.action_write_songs(&mut rng)
        .expect("write should work");

    // Stress drain: (60 - 50) / 5 = 2 creativity points.
    let expected_drain = (game.player.stress - constants::WRITING_STRESS_CREATIVITY_THRESHOLD)
        / constants::WRITING_STRESS_CREATIVITY_DIVISOR;
    assert_eq!(
        game.player.creativity,
        100 - expected_drain,
        "writing at stress 60 should drain (60 - 50) / 5 = {} creativity",
        expected_drain
    );
}

#[test]
fn writing_streak_resets_after_non_writing_action() {
    let mut game = test_game();
    game.initialize_player("Test", "Band", genre::MusicGenre::Rock);
    game.player.stress = 0;
    game.player.creativity = 100;
    game.player.money = 10_000;
    game.band.fame = 10;

    // Build up a writing streak.
    let mut rng = StdRng::seed_from_u64(300);
    game.action_write_songs(&mut rng).expect("week 1 write");
    assert_eq!(game.writing_streak, 1);
    game.update_lifestyle(&GameAction::WriteSongs);

    game.action_write_songs(&mut rng).expect("week 2 write");
    assert_eq!(game.writing_streak, 2);
    game.update_lifestyle(&GameAction::WriteSongs);

    // Now do a non-writing action.
    game.update_lifestyle(&GameAction::LazeAround);
    assert_eq!(
        game.writing_streak, 0,
        "non-writing action should reset writing_streak"
    );
}

#[test]
fn writing_is_blocked_at_stress_90_or_above() {
    let mut game = test_game();
    game.initialize_player("Test", "Band", genre::MusicGenre::Rock);

    // At stress 89 (below block threshold), writing should work.
    game.player.stress = 89;
    game.player.money = 10_000;
    let mut rng = StdRng::seed_from_u64(400);
    let result = game.action_write_songs(&mut rng);
    assert!(result.is_ok(), "writing at stress 89 should succeed");

    // At stress 90 (at or above block threshold), writing should fail.
    game.player.stress = 90;
    let mut rng = StdRng::seed_from_u64(401);
    let result = game.action_write_songs(&mut rng);
    assert!(
        result.is_err(),
        "writing at stress 90 should be blocked: {:?}",
        result
    );

    // At stress 100 (way above), also blocked.
    game.player.stress = 100;
    let mut rng = StdRng::seed_from_u64(402);
    let result = game.action_write_songs(&mut rng);
    assert!(result.is_err(), "writing at stress 100 should be blocked");
}

#[test]
fn recording_is_blocked_at_stress_90_or_above() {
    let mut game = test_game();
    game.initialize_player("Test", "Band", genre::MusicGenre::Rock);
    game.player.money = 10_000;
    game.band.fame = 10;

    // S-RECORD (§A): a single is two songs, so bank a single's worth.
    bank_songs(&mut game, 2, 50);

    // At stress 89, recording should work.
    game.player.stress = 89;
    let result = game.action_record_single(MIN_SINGLE_SONGS, TEST_STUDIO);
    assert!(
        result.is_ok(),
        "recording at stress 89 should succeed: {:?}",
        result
    );
    // §D: the session owns the calendar until it wraps — play it out so the
    // band is free to try again below.
    finish_activity_in_one_sitting(&mut game);

    // Re-bank the songs since we recorded them.
    bank_songs(&mut game, 2, 50);

    // At stress 90, recording should fail.
    game.player.stress = 90;
    let result = game.action_record_single(MIN_SINGLE_SONGS, TEST_STUDIO);
    assert!(
        result.is_err(),
        "recording at stress 90 should be blocked: {:?}",
        result
    );
}

#[test]
fn recording_quality_penalty_applies_when_stress_exceeds_threshold() {
    let mut game = test_game();
    game.initialize_player("Test", "Band", genre::MusicGenre::Rock);
    game.player.stress = 60; // Below RECORDING_STRESS_PENALTY_THRESHOLD (70).
    game.player.happiness = 50;

    let song_quality = 50u8;
    let seed = 600u64;
    let mut rng_low_stress = StdRng::seed_from_u64(seed);
    let quality_low_stress =
        game.calculate_release_quality(song_quality, TEST_STUDIO, &mut rng_low_stress);

    game.player.stress = 80; // Above threshold, should incur penalty.
    let mut rng_high_stress = StdRng::seed_from_u64(seed);
    let quality_high_stress =
        game.calculate_release_quality(song_quality, TEST_STUDIO, &mut rng_high_stress);

    assert!(
        quality_high_stress < quality_low_stress,
        "stress > 70 should degrade recording quality: {} vs {}",
        quality_high_stress,
        quality_low_stress
    );
    // The penalty is −10, but there's randomness; we can't assert the exact difference.
    // Just check that high stress degrades it by a noticeable amount.
    assert!(
        quality_low_stress - quality_high_stress >= 5,
        "stress penalty should reduce quality by at least 5 points"
    );
}

// ---------------------------------------------------------------------------
// S-RECORD (v0.9 §A) — formats have shapes
// ---------------------------------------------------------------------------

/// A format's song count is a bounded choice, and the bounds are enforced at
/// the action, not merely offered by a picker (design §A).
#[test]
fn song_counts_outside_a_formats_range_are_refused() {
    let mut game = well_funded_game();
    bank_songs(&mut game, 20, 60);

    for songs in [MIN_SINGLE_SONGS, MAX_SINGLE_SONGS] {
        assert!(
            game.quote_recording(ReleaseType::Single, songs, TEST_STUDIO)
                .is_ok(),
            "{songs} songs is a legal single"
        );
    }
    for songs in [0, 1, MAX_SINGLE_SONGS + 1, MIN_ALBUM_SONGS] {
        assert!(
            game.quote_recording(ReleaseType::Single, songs, TEST_STUDIO)
                .is_err(),
            "{songs} songs is not a single"
        );
    }

    for songs in [MIN_ALBUM_SONGS, 10, MAX_ALBUM_SONGS] {
        assert!(
            game.quote_recording(ReleaseType::Album, songs, TEST_STUDIO)
                .is_ok(),
            "{songs} songs is a legal album"
        );
    }
    for songs in [MIN_ALBUM_SONGS - 1, MAX_ALBUM_SONGS + 1] {
        assert!(
            game.quote_recording(ReleaseType::Album, songs, TEST_STUDIO)
                .is_err(),
            "{songs} songs is not an album"
        );
    }

    // The action enforces the same bounds the quote does.
    assert!(
        game.action_record_single(1, TEST_STUDIO).is_err(),
        "a one-song single is not a single, however much money is on the table"
    );
}

/// A booking the notebook can't fill is refused with the shortfall named, and
/// costs the band nothing (design §A).
#[test]
fn a_short_notebook_is_refused_with_the_shortfall_named() {
    let mut game = well_funded_game();
    bank_songs(&mut game, 6, 60);
    let money_before = game.player.money;

    let err = game
        .quote_recording(ReleaseType::Album, 10, TEST_STUDIO)
        .expect_err("six banked songs cannot fill a ten-track album");
    assert!(
        err.contains('4') && err.contains("album"),
        "the error should name the four-song shortfall: {err}"
    );

    assert!(game.action_record_album(10, TEST_STUDIO).is_err());
    assert_eq!(
        game.band.unreleased_songs.len(),
        6,
        "a refused booking spends no songs"
    );
    assert_eq!(
        game.player.money, money_before,
        "a refused booking spends no money"
    );
}

/// The heart of §A: recording takes the best-quality N, not the last N
/// written. The old `get_selected_songs_for_release` drained the tail, which
/// silently spent whatever was newest.
#[test]
fn recording_selects_the_best_songs_not_the_newest() {
    let mut game = well_funded_game();
    // Deliberately worst-last, so "the tail" and "the best" disagree totally.
    bank_song(&mut game, "Masterpiece", 90);
    bank_song(&mut game, "Very Good", 80);
    bank_song(&mut game, "Filler", 20);
    bank_song(&mut game, "Junk", 10);

    let quote = game
        .quote_recording(ReleaseType::Single, 2, TEST_STUDIO)
        .expect("two of four songs");
    assert_eq!(
        quote.avg_song_quality, 85,
        "the quote averages the best two (90, 80), not the last two (20, 10)"
    );

    record_and_release(&mut game, ReleaseType::Single, 2, TEST_STUDIO, Some(0))
        .expect("affordable");

    let release = game.just_released_music.last().expect("recorded");
    assert_eq!(
        release.songs_involved_quality_avg, 85,
        "the record is made of the best two songs"
    );
    assert_eq!(
        release.name, "Single: Masterpiece",
        "the A-side is the strongest song on the record"
    );

    // Exactly the chosen songs left the notebook — the weak ones survive.
    let left: Vec<&str> = game
        .band
        .unreleased_songs
        .iter()
        .map(|song| song.name.as_str())
        .collect();
    assert_eq!(
        left,
        vec!["Filler", "Junk"],
        "exactly the two best songs were spent, and the rest kept their order"
    );
}

/// Cost is per song now, so what you are making enters the price (design §A).
#[test]
fn recording_cost_scales_with_song_count() {
    let game = well_funded_game();
    let two = game.recording_cost(TEST_STUDIO, 2);
    let four = game.recording_cost(TEST_STUDIO, 4);
    let fourteen = game.recording_cost(TEST_STUDIO, 14);

    assert!(two > 0, "studio time is never free");
    assert_eq!(four, two * 2, "twice the songs, twice the studio time");
    assert_eq!(fourteen, two * 7);

    // The era modifier still multiplies the whole bill, as it always did.
    let era = game.timeline.get_recording_cost_modifier();
    assert_eq!(
        two,
        (TEST_STUDIO.rate_per_song() as f32 * 2.0 * era) as i32,
        "cost is rate/song × songs × era modifier"
    );
}

/// §A: a single is the cheap, fast, low-ceiling release. Same quality, same
/// fame, same week — it simply earns less than an album does.
#[test]
fn a_single_earns_less_than_an_album_of_the_same_quality() {
    let game = well_funded_game();
    let mut single = test_release(1, ReleaseType::Single);
    let mut album = test_release(2, ReleaseType::Album);
    for release in [&mut single, &mut album] {
        release.release_quality = 70;
        release.marketing_level_achieved = 20;
    }

    let single_score = game.calculate_release_sales_score(&single);
    let album_score = game.calculate_release_sales_score(&album);
    assert!(
        single_score < album_score,
        "a single must not sell like an album: {single_score} vs {album_score}"
    );
    let ratio = single_score as f32 / album_score as f32;
    assert!(
        (ratio - SINGLE_SALES_FACTOR).abs() < 0.02,
        "the single factor should be ~{SINGLE_SALES_FACTOR}, measured {ratio}"
    );
}

// ---------------------------------------------------------------------------
// S-RECORD (v0.9 §C) — the five studios
// ---------------------------------------------------------------------------

/// Fame gates which rooms are selectable, and nothing else (design §C).
#[test]
fn fame_gates_which_studios_are_selectable() {
    let mut game = well_funded_game();
    bank_songs(&mut game, 20, 60);

    game.band.fame = 0;
    let open: Vec<RecordingStudio> = RecordingStudio::ALL
        .iter()
        .copied()
        .filter(|studio| game.studio_is_available(*studio))
        .collect();
    assert_eq!(
        open,
        vec![RecordingStudio::Home, RecordingStudio::DemoRoom],
        "an unknown band can still make a record, but only in the cheap rooms"
    );

    // The gates are monotone: fame only ever opens rooms, never closes them.
    let mut previously_open = 0;
    for fame in 0..=100u8 {
        game.band.fame = fame;
        let count = RecordingStudio::ALL
            .iter()
            .filter(|studio| game.studio_is_available(**studio))
            .count();
        assert!(count >= previously_open, "fame {fame} closed a room");
        previously_open = count;
    }
    assert_eq!(previously_open, 5, "fame 100 opens every room");

    // A gated room is refused by name, at the quote and at the action.
    game.band.fame = 0;
    let err = game
        .quote_recording(ReleaseType::Album, 10, RecordingStudio::WorldClass)
        .expect_err("fame 0 cannot book the world-class room");
    assert!(
        err.contains(RecordingStudio::WorldClass.label()),
        "the refusal names the room: {err}"
    );
    assert!(
        game.action_record_album(10, RecordingStudio::WorldClass)
            .is_err()
    );
}

/// The v0.7 §A lesson, applied to studios: a gate must never become a price.
/// Same studio + same songs = the same quote at any fame (design §C).
#[test]
fn a_studios_price_is_the_same_at_every_fame() {
    let mut game = well_funded_game();
    bank_songs(&mut game, 20, 60);

    for studio in RecordingStudio::ALL {
        let mut quotes = Vec::new();
        for fame in [studio.fame_gate(), 80, 90, 100] {
            game.band.fame = fame;
            let quote = game
                .quote_recording(ReleaseType::Album, 10, studio)
                .expect("fame is at or above the gate");
            quotes.push((quote.cost, quote.weeks));
        }
        assert!(
            quotes.windows(2).all(|pair| pair[0] == pair[1]),
            "{} re-priced itself with fame: {:?}",
            studio.label(),
            quotes
        );
    }
}

/// The studio ladder is a real ladder: a better room is worth more quality and
/// costs more money and more time (design §C).
#[test]
fn the_studio_ladder_orders_quality_cost_and_time() {
    let mut game = well_funded_game();
    game.player.happiness = 50;
    bank_songs(&mut game, 20, 60);

    let mut previous: Option<(u8, i32, u8)> = None;
    for studio in RecordingStudio::ALL {
        let quote = game
            .quote_recording(ReleaseType::Album, 10, studio)
            .expect("fame 100 opens every room");
        // The projected band is computed from the same formula the recording
        // runs, so comparing its floor compares the rooms themselves.
        let point = (quote.quality_low, quote.cost, quote.weeks);
        if let Some((prev_quality, prev_cost, prev_weeks)) = previous {
            assert!(
                point.0 > prev_quality,
                "{} should out-record the room below it: {} vs {}",
                studio.label(),
                point.0,
                prev_quality
            );
            assert!(
                point.1 > prev_cost,
                "{} should cost more than the room below it",
                studio.label()
            );
            assert!(
                point.2 >= prev_weeks,
                "{} should take at least as long as the room below it",
                studio.label()
            );
        }
        previous = Some(point);
    }

    // And the bonus is a shift, not a multiplier: the gap between two rooms is
    // the gap between their bonuses, whatever the songs are worth.
    let seed = 77u64;
    for avg in [20u8, 60, 90] {
        let demo = game.calculate_release_quality(
            avg,
            RecordingStudio::DemoRoom,
            &mut StdRng::seed_from_u64(seed),
        );
        let world = game.calculate_release_quality(
            avg,
            RecordingStudio::WorldClass,
            &mut StdRng::seed_from_u64(seed),
        );
        assert_eq!(
            i16::from(world) - i16::from(demo),
            i16::from(RecordingStudio::WorldClass.quality_bonus()),
            "the room's contribution should be the same fixed shift at avg {avg}"
        );
    }
}

/// The demo room is the neutral baseline the rest of the economy is measured
/// against (design §C): bonus 0, so it changes nothing about a recording.
#[test]
fn the_demo_room_is_the_neutral_baseline() {
    assert_eq!(RecordingStudio::default(), RecordingStudio::DemoRoom);
    assert_eq!(RecordingStudio::DemoRoom.quality_bonus(), 0);
    assert_eq!(
        RecordingStudio::DemoRoom.fame_gate(),
        0,
        "the baseline room must never be out of reach"
    );
}

/// `weeks = ceil(songs × weeks_per_song)`, clamped to `[1, RECORDING_MAX_WEEKS]`
/// (design §B). Quoted here; S-TIME is what spends them.
#[test]
fn the_session_length_is_songs_times_the_rooms_rate_rounded_up() {
    let mut game = well_funded_game();
    bank_songs(&mut game, 20, 60);

    let cases = [
        (ReleaseType::Single, 2u8, RecordingStudio::DemoRoom),
        (ReleaseType::Single, 3, RecordingStudio::Home),
        (ReleaseType::Album, 8, RecordingStudio::Regional),
        (ReleaseType::Album, 10, RecordingStudio::MajorCity),
        (ReleaseType::Album, 14, RecordingStudio::WorldClass),
    ];
    for (release_type, songs, studio) in cases {
        let quote = game
            .quote_recording(release_type, songs, studio)
            .expect("a legal booking");
        let expected = (f32::from(songs) * studio.weeks_per_song()).ceil() as u8;
        assert_eq!(
            quote.weeks,
            expected.clamp(1, RECORDING_MAX_WEEKS),
            "{songs} songs at the {} should be {expected} weeks",
            studio.label()
        );
        assert!(
            (1..=RECORDING_MAX_WEEKS).contains(&quote.weeks),
            "a session is always at least a week and never past the ceiling"
        );
    }

    // The cheapest possible booking still occupies a whole week.
    let shortest = game
        .quote_recording(ReleaseType::Single, 2, RecordingStudio::Home)
        .expect("two songs at home");
    assert_eq!(
        shortest.weeks, 1,
        "0.4 of a week still means the band is in the studio that week"
    );
}

/// The quote is a promise: what it says the studio time costs is what the
/// booking charges, and the record lands inside the band it projected
/// (design §C — "the quote comes first").
#[test]
fn the_quote_matches_what_booking_actually_charges() {
    let mut game = well_funded_game();
    game.player.happiness = 50;
    game.band.record_deal = None;
    bank_songs(&mut game, 12, 65);

    let studio = RecordingStudio::Regional;
    let quote = game
        .quote_recording(ReleaseType::Album, 10, studio)
        .expect("a legal booking");
    let (_, pressing_cost) = game
        .plan_pressing(&ReleaseType::Album, Some(0))
        .expect("tier 0 exists");
    let fee = game
        .plan_distribution(game.current_distribution_channel)
        .expect("channel available");
    let money_before = game.player.money;

    record_and_release(&mut game, ReleaseType::Album, 10, studio, Some(0)).expect("affordable");

    assert_eq!(
        game.player.money,
        money_before - (quote.cost + pressing_cost + fee),
        "the quoted studio time is exactly what was charged for it — plus the          pressing and distribution that §D moved to release day"
    );
    let release = game.just_released_music.last().expect("recorded");
    assert!(
        (quote.quality_low..=quote.quality_high).contains(&release.release_quality),
        "the record ({}) must land inside the projected band {}-{}",
        release.release_quality,
        quote.quality_low,
        quote.quality_high
    );
    assert_eq!(
        game.band.unreleased_songs.len(),
        2,
        "a ten-track album spends ten of the twelve banked songs"
    );
}

/// The bounds and defaults a picker reads (design §A table).
#[test]
fn a_bands_booking_defaults_follow_what_is_in_the_notebook() {
    let mut game = well_funded_game();

    assert_eq!(game.band.default_booking_songs(&ReleaseType::Single), None);
    assert_eq!(game.band.max_recordable_songs(&ReleaseType::Album), None);

    bank_songs(&mut game, 2, 50);
    assert_eq!(
        game.band.default_booking_songs(&ReleaseType::Single),
        Some(MIN_SINGLE_SONGS)
    );
    assert!(game.band.can_record_single());
    assert!(!game.band.can_record_album());

    // Eight songs is an album, but only an eight-track one — the default of
    // ten has to give way to the pile rather than refuse the record.
    bank_songs(&mut game, 6, 50);
    assert_eq!(
        game.band.default_booking_songs(&ReleaseType::Album),
        Some(MIN_ALBUM_SONGS)
    );

    bank_songs(&mut game, 20, 50);
    assert_eq!(
        game.band.default_booking_songs(&ReleaseType::Album),
        Some(studios::default_song_count(&ReleaseType::Album))
    );
    assert_eq!(
        game.band.max_recordable_songs(&ReleaseType::Album),
        Some(MAX_ALBUM_SONGS),
        "a deep notebook is capped by the format, not by the notebook"
    );
    assert_eq!(
        game.band.max_recordable_songs(&ReleaseType::Single),
        Some(MAX_SINGLE_SONGS)
    );
}

// ---------------------------------------------------------------------------
// S-MASTER (v0.9 §D) — recording is not pressing
// ---------------------------------------------------------------------------
//
// The pipeline under test is `book a session -> a Master in the vault -> a
// Release`. Two things carry this section. **The session is real time**: a
// booking charges the studio bill up front and then occupies the calendar one
// week at a time, exactly as a tour does. And **the master is finished the
// moment it is booked**: what comes out of the room is settled at the gate and
// frozen, so neither the weeks that pass nor a save taken in the middle of
// them can move it — which is what makes leaving a record in the vault
// indefinitely a strategy rather than a gamble.

/// A band with songs banked, money to burn, and both story-event taps
/// silenced, for the tests that drive the calendar through `process_turn`.
/// (Historical events and random incidents both hand out money and fame; a
/// money-exact comparison can't survive either. Same idiom as `time.rs`.)
fn session_band(seed: u64) -> Game {
    let mut game = crate::game::sim::seeded_game(seed);
    suppress_story_events(&mut game);
    game.player.money = 1_000_000;
    game.player.stress = 0;
    game.band.fame = 100; // every room open, every channel available
    bank_songs(&mut game, 20, 60);
    game
}

fn suppress_story_events(game: &mut Game) {
    let every_event: Vec<String> = game
        .timeline
        .eras
        .values()
        .flat_map(|era| era.major_events.iter().cloned())
        .collect();
    game.timeline.triggered_events.extend(every_event);
    park_events(game);
}

/// Re-park the incident clock after every turn — `try_trigger_event` writes
/// to it.
fn park_events(game: &mut Game) {
    game.events.last_event_week = u32::MAX;
}

/// A ten-track album at the major-city studio is six weeks (§C's table), which
/// is the shape every "a session is real time" test below wants.
const LONG_SESSION_STUDIO: RecordingStudio = RecordingStudio::MajorCity;

/// §B/§F: a booked session occupies exactly the weeks it quoted, one per turn,
/// and the studio bill is due in full at the gate — nothing further is charged
/// while the band is in the room.
#[test]
fn a_booked_session_takes_exactly_the_weeks_it_quoted() {
    let mut game = session_band(101);
    let quote = game
        .quote_recording(ReleaseType::Album, 10, LONG_SESSION_STUDIO)
        .expect("a legal booking");
    assert!(
        quote.weeks > 1,
        "the case is only interesting if the session spans weeks"
    );
    let booking_week = game.week;
    let money_before = game.player.money;

    game.process_turn(GameAction::RecordAlbum {
        songs: 10,
        studio: LONG_SESSION_STUDIO,
    })
    .expect("the fixture band can afford this session");
    assert_eq!(
        game.player.money,
        money_before - quote.cost,
        "the whole studio bill is due at booking (§B) — and, since §D, nothing else is"
    );

    for week in 1..quote.weeks {
        assert_eq!(
            game.week,
            booking_week + u32::from(week),
            "each turn of a session is exactly one week"
        );
        let activity = game.ongoing_activity.as_ref().expect("still in the room");
        assert!(matches!(activity, OngoingActivity::Recording(_)));
        assert_eq!(activity.weeks_done(), week);
        assert_eq!(
            activity.progress(),
            format!("week {} of {}", week + 1, quote.weeks)
        );
        assert!(
            game.vault().is_empty(),
            "the master lands when the session wraps, not before"
        );
        park_events(&mut game);
        game.process_turn(GameAction::Gig(0))
            .expect("a committed band's turn always resolves");
    }

    assert!(game.ongoing_activity.is_none(), "the session has wrapped");
    assert_eq!(
        game.week,
        booking_week + u32::from(quote.weeks),
        "{} booked weeks cost exactly {} weeks of calendar",
        quote.weeks,
        quote.weeks
    );
    assert_eq!(
        game.player.money,
        money_before - quote.cost,
        "and no gig, pressing or fee snuck in while the band was unavailable"
    );
    assert_eq!(game.vault().len(), 1, "one session, one master");
    assert!(
        game.just_released_music.is_empty(),
        "§D: finishing a record is not releasing it"
    );
}

/// §F: a session is a commitment like any other — a second booking on top of
/// it is refused rather than silently replacing it.
#[test]
fn a_session_in_progress_refuses_a_second_booking() {
    let mut game = session_band(102);
    game.process_turn(GameAction::RecordAlbum {
        songs: 10,
        studio: LONG_SESSION_STUDIO,
    })
    .expect("affordable");

    let err = game
        .action_record_single(2, TEST_STUDIO)
        .expect_err("the band is already in a room");
    assert!(
        err.contains("already committed"),
        "the refusal should name the commitment: {err}"
    );
}

/// The heart of §D: a finished session leaves a **master**, carrying the
/// record as the room made it — and leaves the shops empty.
#[test]
fn a_finished_session_leaves_a_master_in_the_vault_not_a_record_in_the_shops() {
    let mut game = well_funded_game();
    game.player.happiness = 50;
    // Deliberately worst-last, so "the tail" and "the best" disagree.
    bank_song(&mut game, "Masterpiece", 90);
    bank_song(&mut game, "Very Good", 80);
    bank_song(&mut game, "Filler", 20);

    let studio = RecordingStudio::Regional;
    let quote = game
        .quote_recording(ReleaseType::Single, 2, studio)
        .expect("a legal booking");
    let money_before = game.player.money;

    game.action_record_single(2, studio).expect("affordable");
    assert_eq!(
        game.player.money,
        money_before - quote.cost,
        "booking charges studio time and nothing else — no pressing, no distribution"
    );
    finish_activity_in_one_sitting(&mut game);

    assert!(
        game.just_released_music.is_empty() && game.band.singles_released.is_empty(),
        "§D: recording does not press, does not distribute, and does not release"
    );
    let master = game.vault().first().expect("the session left a master");
    assert_eq!(master.release_type, ReleaseType::Single);
    assert_eq!(
        master.studio, studio,
        "the master remembers the room it was cut in"
    );
    assert_eq!(master.songs.len(), 2);
    assert_eq!(
        master.songs_involved_quality_avg, 85,
        "the best two songs (90, 80), not the last two written"
    );
    assert_eq!(
        master.name, "Single: Masterpiece",
        "the A-side names the record"
    );
    assert!(
        (quote.quality_low..=quote.quality_high).contains(&master.release_quality),
        "the master ({}) must land inside the band the quote projected {}-{}",
        master.release_quality,
        quote.quality_low,
        quote.quality_high
    );
    assert_eq!(master.week_finished, game.week);

    let left: Vec<&str> = game
        .band
        .unreleased_songs
        .iter()
        .map(|song| song.name.as_str())
        .collect();
    assert_eq!(
        left,
        vec!["Filler"],
        "the chosen songs went into the session at booking and are in the can"
    );
}

/// §D: release day buys the pressing run and the reach — and never the studio
/// again. The record that comes out is the one the room made, weeks ago.
#[test]
fn releasing_a_master_charges_pressing_and_distribution_and_no_studio_time() {
    let mut game = well_funded_game();
    game.band.record_deal = None;
    game.current_distribution_channel = DistributionChannel::Regional;
    bank_songs(&mut game, 12, 65);

    let studio = RecordingStudio::Regional;
    game.action_record_album(10, studio).expect("affordable");
    finish_activity_in_one_sitting(&mut game);

    let quote = game
        .quote_release(0, Some(0))
        .expect("a master is in the vault");
    assert!(
        quote.pressing_cost > 0 && quote.distribution_fee > 0,
        "an unsigned act buys both, and this test is about being charged for them"
    );
    let master_quality = game.vault()[0].release_quality;
    let master_name = game.vault()[0].name.clone();
    let money_before = game.player.money;

    game.action_release_master(0, Some(0))
        .expect("the band can cover the pressing");

    assert_eq!(
        game.player.money,
        money_before - quote.total(),
        "release day charges exactly the pressing run plus the channel fee"
    );
    assert!(game.vault().is_empty(), "the master left the vault");
    let release = game.just_released_music.last().expect("it's out");
    assert_eq!(release.name, master_name);
    assert_eq!(
        release.release_quality, master_quality,
        "the record is what came out of the room, decided at booking"
    );
    assert_eq!(release.copies_pressed, quote.copies);
    assert_eq!(
        release.distribution_channel,
        Some(DistributionChannel::Regional),
        "the release remembers the channel it went out under"
    );
    assert_eq!(release.week_released, game.week);
}

/// §D: "A signed act's label still presses on release, as today."
#[test]
fn a_signed_acts_label_still_presses_on_release() {
    let mut game = well_funded_game();
    game.band.record_deal = Some(test_deal(70, 0.12));
    bank_songs(&mut game, 12, 60);

    game.action_record_album(10, TEST_STUDIO)
        .expect("affordable");
    finish_activity_in_one_sitting(&mut game);
    let money_before = game.player.money;

    game.action_release_master(0, None)
        .expect("the label covers a signed release");

    assert_eq!(
        game.player.money, money_before,
        "a signed act pays nothing on release day — the label presses"
    );
    let release = game.just_released_music.last().expect("it's out");
    assert!(release.copies_pressed > 0, "the label pressed a run");
    assert_eq!(
        release.distribution_channel, None,
        "a label release is channel-blind (§E-3)"
    );
    assert!(
        release.marketing_level_achieved > 0,
        "and the promo machine still comes with it"
    );
}

/// §D: "**Sitting on a master is legal** and sometimes right." Nothing expires
/// it, nothing in the world can change it, and when it finally goes out it
/// charts and ages from *that* week — which is the whole reason to wait.
#[test]
fn a_master_can_sit_unreleased_for_as_long_as_the_player_likes() {
    const SHELVED_WEEKS: u32 = 40;

    let mut game = session_band(202);
    game.process_turn(GameAction::RecordSingle {
        songs: 2,
        studio: TEST_STUDIO,
    })
    .expect("affordable");
    assert!(
        game.ongoing_activity.is_none(),
        "two songs in the demo room is a one-week session"
    );
    let finished_week = game.vault()[0].week_finished;
    let quality = game.vault()[0].release_quality;
    let name = game.vault()[0].name.clone();

    for _ in 0..SHELVED_WEEKS {
        park_events(&mut game);
        game.player.health = 100; // the shelf, not the band, is under test
        game.process_turn(GameAction::LazeAround)
            .expect("lazing is always allowed");
    }

    assert!(!game.is_game_over());
    assert_eq!(game.vault().len(), 1, "a master keeps — nothing expires it");
    assert_eq!(
        (
            game.vault()[0].release_quality,
            game.vault()[0].name.clone()
        ),
        (quality, name),
        "and nothing that happened out there could reach into the can"
    );
    assert!(
        game.just_released_music.is_empty() && game.band.singles_released.is_empty(),
        "forty weeks on and it is still not out"
    );

    let release_week = game.week;
    // The master is stamped inside the booking turn, before that turn's own
    // week ticks over — so the shelf is `SHELVED_WEEKS` idle turns plus the
    // remainder of the week it was finished in.
    assert_eq!(release_week, finished_week + SHELVED_WEEKS + 1);
    game.action_release_master(0, Some(0))
        .expect("still releasable, forty weeks later");
    let release = game.just_released_music.last().expect("out at last");
    assert_eq!(
        release.week_released, release_week,
        "a shelved record charts, sells and ages from the day it comes out, \
         not the day it was finished"
    );
    assert_eq!(release.release_quality, quality);
}

/// §F — Abandoning, in the studio: you keep what resolved, and a session that
/// never wrapped resolved nothing. The songs went in with the tapes and the
/// bill was paid at the gate.
#[test]
fn walking_out_of_a_session_forfeits_the_songs_and_the_bill() {
    let mut game = session_band(404);
    let songs_before = game.band.unreleased_songs.len();
    let money_before = game.player.money;
    let quote = game
        .quote_recording(ReleaseType::Album, 10, LONG_SESSION_STUDIO)
        .expect("a legal booking");

    game.process_turn(GameAction::RecordAlbum {
        songs: 10,
        studio: LONG_SESSION_STUDIO,
    })
    .expect("affordable");
    park_events(&mut game);
    game.process_turn(GameAction::LazeAround)
        .expect("a second week in the room");
    let week_before = game.week;

    game.process_turn(GameAction::AbandonActivity)
        .expect("walking out is always allowed");

    assert!(
        game.ongoing_activity.is_none(),
        "the session is off the books"
    );
    assert_eq!(
        game.week, week_before,
        "the week you walk out of is yours to spend — no week consumed"
    );
    assert!(game.vault().is_empty(), "an unfinished record is no record");
    assert_eq!(
        game.band.unreleased_songs.len(),
        songs_before - 10,
        "the ten songs went into the session and did not come back"
    );
    assert_eq!(
        game.player.money,
        money_before - quote.cost,
        "and there is no refund on studio time already booked"
    );

    // And the band is free again the very next turn.
    park_events(&mut game);
    game.process_turn(GameAction::WriteSongs)
        .expect("free to work again");
}

/// A save taken mid-session resumes into the same session and finishes the
/// same record — because the record was settled at booking and its rolls are
/// keyed by the booking week, not by a stream position that would have to
/// survive the trip. The §D sibling of
/// `time::a_save_taken_mid_tour_resumes_and_finishes_the_same_way`.
#[test]
fn a_save_taken_mid_session_resumes_and_finishes_the_same_way() {
    fn play_out(game: &mut Game, weeks: u8) {
        for _ in 0..weeks {
            park_events(game);
            game.process_turn(GameAction::LazeAround)
                .expect("a committed band's turn always resolves");
        }
    }

    let weeks = session_band(31)
        .quote_recording(ReleaseType::Album, 10, LONG_SESSION_STUDIO)
        .expect("a legal booking")
        .weeks;
    assert!(weeks > 2, "the save has to land in the middle of something");

    let uninterrupted = {
        let mut game = session_band(31);
        game.process_turn(GameAction::RecordAlbum {
            songs: 10,
            studio: LONG_SESSION_STUDIO,
        })
        .expect("affordable");
        play_out(&mut game, weeks - 1);
        game
    };

    let mut game = session_band(31);
    game.process_turn(GameAction::RecordAlbum {
        songs: 10,
        studio: LONG_SESSION_STUDIO,
    })
    .expect("affordable");
    play_out(&mut game, 1);

    let path = std::env::temp_dir().join("rocker-s-master-mid-session.sav");
    let path = path.to_str().expect("a utf-8 temp path");
    game.save_game(path)
        .expect("saving mid-session should work");
    let mut resumed = Game::load_game(path).expect("and loading it back should too");
    let _ = std::fs::remove_file(path);

    let activity = resumed
        .ongoing_activity
        .as_ref()
        .expect("the save was taken in the studio");
    assert!(matches!(activity, OngoingActivity::Recording(_)));
    assert_eq!(activity.weeks_done(), 2);
    assert_eq!(activity.label(), "Recording");
    assert!(
        activity.headline().contains(LONG_SESSION_STUDIO.label()),
        "the activity screen names the room: {}",
        activity.headline()
    );

    suppress_story_events(&mut resumed);
    play_out(&mut resumed, weeks - 2);

    assert!(resumed.ongoing_activity.is_none());
    assert_eq!(resumed.week, uninterrupted.week);
    assert_eq!(resumed.player.money, uninterrupted.player.money);
    let resumed_master = resumed.vault().first().expect("resumed session wraps");
    let reference_master = uninterrupted
        .vault()
        .first()
        .expect("so does the reference");
    assert_eq!(
        (
            resumed_master.name.as_str(),
            resumed_master.release_quality,
            resumed_master.songs.len(),
            resumed_master.week_finished
        ),
        (
            reference_master.name.as_str(),
            reference_master.release_quality,
            reference_master.songs.len(),
            reference_master.week_finished
        ),
        "a save in the middle of a session cannot change a note of the record"
    );
}

/// §I — calendar integrity, with the studio in the mix. The sibling of
/// `time::weeks_elapsed_equals_the_turns_that_consumed_a_week`, run against a
/// policy that lives in the studio rather than on the road: sessions of every
/// length, and the instant release action, which must never move the calendar.
#[test]
fn weeks_elapsed_equals_the_turns_that_consumed_a_week_with_a_studio_in_the_mix() {
    const HORIZON: u32 = 5 * constants::WEEKS_PER_YEAR;

    for seed in [3u64, 37] {
        let mut game = crate::game::sim::seeded_game(seed);
        suppress_story_events(&mut game);
        game.player.money = 5_000_000; // never let poverty end the run early
        let start_week = game.week;

        let mut week_consuming_turns: u32 = 0;
        let mut booked = 0u32;
        let mut released = 0u32;

        while !game.is_game_over() && game.week - start_week < HORIZON {
            let action = studio_probe_action(&game);
            match &action {
                GameAction::RecordSingle { .. } | GameAction::RecordAlbum { .. } => booked += 1,
                GameAction::ReleaseMaster { .. } => released += 1,
                _ => {}
            }

            let week_before = game.week;
            if game.process_turn(action).is_err() {
                game.process_turn(GameAction::LazeAround)
                    .expect("lazing is always allowed");
            }
            game.take_turn_log();
            park_events(&mut game);

            let elapsed = game.week - week_before;
            assert!(
                elapsed <= 1,
                "no turn may consume more than one week — {} weeks at week {} (seed {seed})",
                elapsed,
                week_before
            );
            week_consuming_turns += elapsed;
        }

        assert_eq!(
            game.week - start_week,
            week_consuming_turns,
            "weeks elapsed must equal the turns that consumed a week (seed {seed})"
        );
        assert!(booked > 0, "the probe should have booked (seed {seed})");
        assert!(released > 0, "the probe should have released (seed {seed})");
    }
}

/// The studio probe's policy: empty the vault the moment anything is in it
/// (an instant action, which is exactly the kind that must not move the
/// calendar), then book the longest session the notebook can fill in the
/// deepest room fame has opened — the bookings most likely to break a
/// one-week-at-a-time calendar.
fn studio_probe_action(game: &Game) -> GameAction {
    if !game.vault().is_empty() {
        return GameAction::ReleaseMaster {
            master: 0,
            pressing: Some(0),
        };
    }
    if game.player.stress >= 50 || game.player.health < 60 {
        return GameAction::TakeBreak;
    }
    let studio = RecordingStudio::ALL
        .iter()
        .copied()
        .rev()
        .find(|studio| game.studio_is_available(*studio))
        .unwrap_or_default();
    for release_type in [ReleaseType::Album, ReleaseType::Single] {
        if let Some(songs) = game.band.max_recordable_songs(&release_type)
            && game
                .quote_recording(release_type, songs, studio)
                .is_ok_and(|quote| game.player.can_afford(quote.cost))
        {
            return match release_type {
                ReleaseType::Album => GameAction::RecordAlbum { songs, studio },
                ReleaseType::Single => GameAction::RecordSingle { songs, studio },
            };
        }
    }
    if game.player.stress < STUDIO_STRESS_BLOCK {
        GameAction::WriteSongs
    } else {
        GameAction::LazeAround
    }
}
