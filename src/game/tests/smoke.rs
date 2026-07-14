//! Whole-turn smoke tests: no panics, break mechanics, and genre-trend press.

use super::*;

#[test]
fn a_full_season_of_turns_never_panics() {
    let mut game = test_game();
    game.initialize_player("Test", "The Tests", genre::MusicGenre::Rock);
    for i in 0..30 {
        let action = match i % 6 {
            0 => GameAction::WriteSongs,
            1 => GameAction::Gig(0),
            2 => GameAction::LazeAround,
            3 => GameAction::RecordSingle { pressing: Some(0) },
            4 => GameAction::Practice,
            _ => GameAction::TakeBreak,
        };
        // Rejected actions are fine; panics are not.
        let _ = game.process_turn(action);
        game.player.money = game.player.money.max(1_000);
        game.player.energy = 100;
        game.player.health = 100;
    }
}

#[test]
fn a_break_is_a_real_break() {
    let mut game = test_game();
    let week_before = game.week;
    game.player.health = 50;
    game.player.stress = 40;
    let happiness_before = game.player.happiness;
    let creativity_before = game.player.creativity;

    game.action_take_break().expect("a break always works");

    assert_eq!(
        game.week,
        week_before + BREAK_WEEKS - 1,
        "the turn itself adds the final week"
    );
    assert_eq!(game.player.health, 80);
    assert_eq!(game.player.stress, 0, "a break resets stress");
    assert_eq!(game.player.happiness, happiness_before + 10);
    assert_eq!(game.player.creativity, creativity_before + 10);
}

#[test]
fn the_press_calls_a_hot_genre_once_not_weekly() {
    let mut game = test_game();
    // Rock is the sound of 1970 in the era data — clearly hot.
    game.band.genre = genre::MusicGenre::Rock;

    game.process_turn(GameAction::LazeAround)
        .expect("lazing always works");
    game.process_turn(GameAction::LazeAround)
        .expect("lazing always works");

    let mentions = game
        .turn_log
        .iter()
        .filter(|line| line.contains("right scene at the right time"))
        .count();
    assert_eq!(mentions, 1, "the trend is news once, not every week");
}

#[test]
fn the_press_notices_a_genre_the_era_left_behind() {
    let mut game = test_game();
    // Punk is years ahead of 1970's tastes — out of fashion on day one.
    game.band.genre = genre::MusicGenre::Punk;

    game.process_turn(GameAction::LazeAround)
        .expect("lazing always works");

    assert!(
        game.turn_log
            .iter()
            .any(|line| line.contains("chasing a different sound")),
        "an off-trend band should hear about it"
    );
}

#[test]
fn rockstar_milestone_fires_once_and_game_continues() {
    let mut game = test_game();
    game.initialize_player("Test", "The Legends", genre::MusicGenre::Rock);

    // Manually set up rockstar condition: fame >= 90, albums >= 5
    game.band.fame = 90;
    game.band.albums_released = vec![
        test_release(1, ReleaseType::Album),
        test_release(2, ReleaseType::Album),
        test_release(3, ReleaseType::Album),
        test_release(4, ReleaseType::Album),
        test_release(5, ReleaseType::Album),
    ];

    // Suppress random incidents (L8): they now fire weekly and could nudge
    // fame off the exact 90 threshold this test pins. u32::MAX keeps the gate
    // shut regardless of cadence.
    game.events.last_event_week = u32::MAX;

    // Process a turn — should trigger the milestone
    let continue_playing = game
        .process_turn(GameAction::LazeAround)
        .expect("turn should succeed");

    // Game should continue, not end
    assert!(
        continue_playing,
        "game should continue after reaching rockstar"
    );
    assert!(!game.is_game_over(), "game_over should be false");
    assert!(game.rockstar_achieved, "rockstar_achieved should be set");

    // Check that the milestone log fired
    let milestone_logs = game
        .turn_log
        .iter()
        .filter(|line| line.contains("bona fide ROCKSTAR"))
        .count();
    assert_eq!(milestone_logs, 1, "milestone should log exactly once");

    // Process another turn — milestone should NOT fire again
    game.turn_log.clear();
    let _ = game
        .process_turn(GameAction::LazeAround)
        .expect("second turn should succeed");

    let second_milestone_logs = game
        .turn_log
        .iter()
        .filter(|line| line.contains("bona fide ROCKSTAR"))
        .count();
    assert_eq!(second_milestone_logs, 0, "milestone should only fire once");
}

#[test]
fn rockstar_achieved_flag_survives_save_load() {
    let mut game = test_game();
    game.initialize_player("Test", "The Legends", genre::MusicGenre::Rock);

    // Set rockstar achieved
    game.band.fame = 90;
    game.band.albums_released = vec![
        test_release(1, ReleaseType::Album),
        test_release(2, ReleaseType::Album),
        test_release(3, ReleaseType::Album),
        test_release(4, ReleaseType::Album),
        test_release(5, ReleaseType::Album),
    ];
    // Suppress random incidents (L8) so nothing nudges fame off 90 (see above).
    game.events.last_event_week = u32::MAX;
    game.process_turn(GameAction::LazeAround)
        .expect("turn should succeed");

    assert!(game.rockstar_achieved, "flag should be set");

    // Save to JSON
    let json = serde_json::to_string(&game).expect("should serialize");

    // Load from JSON
    let loaded: Game = serde_json::from_str(&json).expect("should deserialize");

    assert!(loaded.rockstar_achieved, "flag should survive save/load");
}

#[test]
fn death_ending_still_works() {
    let mut game = test_game();
    game.initialize_player("Test", "The Tests", genre::MusicGenre::Rock);

    // Set up health = 0 to trigger death ending
    game.player.health = 0;
    game.process_turn(GameAction::LazeAround)
        .expect("turn should succeed");

    assert!(game.is_game_over(), "game should end at health 0");
    let status = game.get_status_message();
    assert!(status.contains("died"), "status should mention death");
}

#[test]
fn broke_and_unknown_ending_still_works() {
    let mut game = test_game();
    game.initialize_player("Test", "The Tests", genre::MusicGenre::Rock);
    // An unseeded game: an incident or a historical event on the single turn
    // below can hand out money/fame and lift the band out of "broke and
    // unknown". Park the incident clock and mark every era event told, so the
    // ending logic is what's under test (same idiom as the sim harness).
    game.events.last_event_week = u32::MAX;
    let every_event: Vec<String> = game
        .timeline
        .eras
        .values()
        .flat_map(|era| era.major_events.iter().cloned())
        .collect();
    game.timeline.triggered_events.extend(every_event);

    // Set up broke + unknown to trigger the broke-and-unknown ending
    game.player.money = -100;
    game.band.fame = 5; // Below 10
    game.process_turn(GameAction::LazeAround)
        .expect("turn should succeed");

    assert!(
        game.is_game_over(),
        "game should end when broke and unknown"
    );
    let status = game.get_status_message();
    assert!(
        status.contains("broke"),
        "status should mention being broke"
    );
}
