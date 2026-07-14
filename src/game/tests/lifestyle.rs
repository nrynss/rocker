//! The v0.6 weekly lifestyle tick: stress economy, happiness/creativity
//! drains, and the lazing-streak health wear (`update_lifestyle` in
//! `lifestyle.rs`). See `docs/DESIGN-v0.6-life-cycle.md` §A.

use super::*;

#[test]
fn happiness_drains_under_high_stress() {
    let mut game = test_game();
    game.player.stress = 100;
    game.player.money = 0; // not broke
    let happiness_before = game.player.happiness;

    game.update_lifestyle(&GameAction::WriteSongs);

    // Passive release fires first (100 -> 97), then happiness drains by
    // stress / HAPPINESS_STRESS_DIVISOR (97 / 25 = 3).
    assert_eq!(game.player.stress, 97);
    assert_eq!(game.player.happiness, happiness_before - 3);
}

#[test]
fn creativity_only_drains_above_the_stress_threshold() {
    // At the boundary: 43 -> 40 after passive release, not strictly
    // above the threshold, so creativity is untouched.
    let mut game = test_game();
    game.player.stress = 43;
    game.player.money = 0;
    let creativity_before = game.player.creativity;
    game.update_lifestyle(&GameAction::WriteSongs);
    assert_eq!(game.player.stress, 40);
    assert_eq!(
        game.player.creativity, creativity_before,
        "stress at exactly the threshold should not drain creativity"
    );

    // Clearly above: 90 -> 87 after passive release, drains (87-40)/20 = 2.
    let mut game = test_game();
    game.player.stress = 90;
    game.player.money = 0;
    let creativity_before = game.player.creativity;
    game.update_lifestyle(&GameAction::WriteSongs);
    assert_eq!(game.player.stress, 87);
    assert_eq!(game.player.creativity, creativity_before - 2);
}

#[test]
fn broke_adds_stress_on_top_of_passive_release() {
    let mut game = test_game();
    game.player.stress = 20;
    game.player.money = -50;

    game.update_lifestyle(&GameAction::WriteSongs);

    // Passive release: 20 -> 17, then +5 for being broke: 17 -> 22.
    assert_eq!(game.player.stress, 22);
}

#[test]
fn laze_streak_wears_health_only_past_four_consecutive_weeks_and_resets() {
    let mut game = test_game();
    game.player.health = 100;

    for week in 1..=4u32 {
        game.update_lifestyle(&GameAction::LazeAround);
        assert_eq!(game.player.laze_streak, week);
        assert_eq!(
            game.player.health, 100,
            "no health wear before the streak passes 4 weeks"
        );
    }

    game.update_lifestyle(&GameAction::LazeAround);
    assert_eq!(game.player.laze_streak, 5);
    assert_eq!(
        game.player.health, 99,
        "the 5th consecutive lazing week starts wearing health"
    );

    game.update_lifestyle(&GameAction::WriteSongs);
    assert_eq!(
        game.player.laze_streak, 0,
        "any other action resets the lazing streak"
    );
}

#[test]
fn old_save_defaults_the_new_bars() {
    // Mirrors saves_from_v0_4_still_load's fixture check but focused on
    // the new v0.6 fields specifically: a save written before happiness/
    // creativity/laze_streak existed must fill in the documented defaults.
    let path = concat!(env!("CARGO_MANIFEST_DIR"), "/tests/fixtures/pre-0.5.sav");
    let game = Game::load_game(path).expect("a v0.4.0 save must keep loading");

    assert_eq!(game.player.happiness, constants::DEFAULT_HAPPINESS);
    assert_eq!(game.player.creativity, constants::DEFAULT_CREATIVITY);
    assert_eq!(game.player.laze_streak, 0);
}
