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
