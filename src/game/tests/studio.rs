//! Writing and recording: songwriting quality (creativity-driven, happiness
//! multiplier), recording quality, stress economy, and streak tracking.

use super::*;
use rand::SeedableRng;
use rand::rngs::StdRng;

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

    // Add a song to record.
    game.band.unreleased_songs.push(music::Song {
        id: 0,
        name: "Test Song".to_string(),
        songwriting_quality: 50,
    });

    // At stress 89, recording should work.
    game.player.stress = 89;
    let mut rng = StdRng::seed_from_u64(500);
    let result = game.action_record_single(Some(0), &mut rng);
    assert!(
        result.is_ok(),
        "recording at stress 89 should succeed: {:?}",
        result
    );

    // Re-add the song since we recorded it.
    game.band.unreleased_songs.push(music::Song {
        id: 1,
        name: "Test Song 2".to_string(),
        songwriting_quality: 50,
    });

    // At stress 90, recording should fail.
    game.player.stress = 90;
    let mut rng = StdRng::seed_from_u64(501);
    let result = game.action_record_single(Some(0), &mut rng);
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
    let quality_low_stress = game.calculate_release_quality(song_quality, &mut rng_low_stress);

    game.player.stress = 80; // Above threshold, should incur penalty.
    let mut rng_high_stress = StdRng::seed_from_u64(seed);
    let quality_high_stress = game.calculate_release_quality(song_quality, &mut rng_high_stress);

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
