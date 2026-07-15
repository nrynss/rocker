//! The v0.6 weekly lifestyle tick: stress economy, happiness/creativity
//! drains, and the lazing-streak health wear (`update_lifestyle` in
//! `lifestyle.rs`). See `docs/DESIGN-v0.6-life-cycle.md` §A. Also the
//! v0.7 lifestyle ladder built on top of it: weekly upkeep, the
//! happiness floor, image, moves, and broke eviction
//! (`docs/DESIGN-v0.7-money-cycle.md` §B).

use super::*;
use crate::game::player::LifestyleTier;

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

    // v0.7 §B: a save that predates the lifestyle ladder entirely must
    // default to the cheapest tier and a clean broke/tabloid clock.
    assert_eq!(game.player.lifestyle, LifestyleTier::Squat);
    assert_eq!(game.player.weeks_broke, 0);
    assert_eq!(game.player.tabloid_streak, 0);
}

// --- v0.7 §B: the lifestyle ladder ---

#[test]
fn upkeep_is_deducted_weekly_per_tier() {
    let mut game = test_game();
    game.player.lifestyle = LifestyleTier::Townhouse;
    game.player.money = 1_000;

    game.update_lifestyle(&GameAction::WriteSongs);

    assert_eq!(
        game.player.money,
        1_000 - LifestyleTier::Townhouse.upkeep_per_week() as i32
    );
}

#[test]
fn squat_carries_no_upkeep() {
    let mut game = test_game();
    game.player.money = 500;
    game.update_lifestyle(&GameAction::WriteSongs);
    assert_eq!(game.player.money, 500, "Squat's upkeep is $0/week");
}

#[test]
fn happiness_floor_clamps_the_weekly_drain() {
    let mut game = test_game();
    game.player.lifestyle = LifestyleTier::Townhouse; // floor 15, stress bonus 3
    game.player.money = 10_000; // upkeep never threatens broke here
    game.player.happiness = 16;
    game.player.stress = 100;

    game.update_lifestyle(&GameAction::WriteSongs);

    // stress_release = 3 (base) + 3 (tier) = 6 -> stress 94 -> drain 94/25 = 3.
    // 16 - 3 = 13, which is below the floor (15): clamped up to 15.
    assert_eq!(
        game.player.happiness, 15,
        "the drain cannot pull happiness below the Townhouse floor"
    );
}

#[test]
fn happiness_already_below_the_floor_from_an_event_is_left_alone() {
    // The floor guards this specific drain; it does not heal a happiness
    // hit taken elsewhere (an incident, an eviction, etc.).
    let mut game = test_game();
    game.player.lifestyle = LifestyleTier::Townhouse; // floor 15
    game.player.money = 10_000;
    game.player.stress = 0; // drain will be 0
    game.player.happiness = 5; // already below the floor

    game.update_lifestyle(&GameAction::WriteSongs);

    assert_eq!(
        game.player.happiness, 5,
        "already below floor and no drain this week: happiness is untouched, not raised to the floor"
    );
}

#[test]
fn moving_up_charges_the_deposit_and_grants_happiness() {
    let mut game = test_game();
    game.player.money = 1_000;
    game.player.happiness = 50;

    let cost = LifestyleTier::SharedFlat.move_up_cost();
    game.action_change_lifestyle(LifestyleTier::SharedFlat)
        .expect("affordable move up");

    assert_eq!(game.player.lifestyle, LifestyleTier::SharedFlat);
    assert_eq!(game.player.money, 1_000 - cost as i32);
    assert_eq!(game.player.happiness, 50 + LifestyleTier::MOVE_UP_HAPPINESS);
}

#[test]
fn moving_up_is_refused_when_the_deposit_is_unaffordable() {
    let mut game = test_game();
    game.player.money = 10;

    let result = game.action_change_lifestyle(LifestyleTier::Mansion);

    assert!(result.is_err());
    assert_eq!(
        game.player.lifestyle,
        LifestyleTier::Squat,
        "a rejected move leaves the tier unchanged"
    );
    assert_eq!(game.player.money, 10, "no money changes hands on a refusal");
}

#[test]
fn moving_down_is_free_and_costs_happiness() {
    let mut game = test_game();
    game.player.lifestyle = LifestyleTier::CityApartment;
    game.player.money = 100;
    game.player.happiness = 50;

    game.action_change_lifestyle(LifestyleTier::Squat)
        .expect("moving down is always allowed");

    assert_eq!(game.player.lifestyle, LifestyleTier::Squat);
    assert_eq!(game.player.money, 100, "moving down is free");
    assert_eq!(
        game.player.happiness,
        50 - LifestyleTier::MOVE_DOWN_HAPPINESS
    );
}

#[test]
fn broke_two_consecutive_weeks_triggers_eviction() {
    let mut game = test_game();
    game.player.lifestyle = LifestyleTier::CityApartment;
    game.player.money = 100;
    game.player.happiness = 50;

    // Week 1: upkeep (180) pushes money to -80 — one broke week, no eviction yet.
    game.update_lifestyle(&GameAction::WriteSongs);
    assert_eq!(game.player.weeks_broke, 1);
    assert_eq!(game.player.lifestyle, LifestyleTier::CityApartment);

    // Week 2: still in the red — the second consecutive broke week evicts.
    game.update_lifestyle(&GameAction::WriteSongs);
    assert_eq!(
        game.player.lifestyle,
        LifestyleTier::SharedFlat,
        "two consecutive broke weeks evicts one tier down"
    );
    assert_eq!(
        game.player.weeks_broke, 0,
        "the eviction resets the broke clock"
    );
    assert_eq!(
        game.player.happiness, 30,
        "eviction costs 20 happiness (50 - 20), no other drain fires this week"
    );
}

#[test]
fn broke_streak_resets_once_money_recovers() {
    let mut game = test_game();
    game.player.lifestyle = LifestyleTier::CityApartment;
    game.player.money = 100;

    game.update_lifestyle(&GameAction::WriteSongs); // -> -80, weeks_broke = 1
    assert_eq!(game.player.weeks_broke, 1);

    game.player.money = 1_000; // recovered before the next tick
    game.update_lifestyle(&GameAction::WriteSongs); // -> 820, not broke

    assert_eq!(game.player.weeks_broke, 0);
    assert_eq!(game.player.lifestyle, LifestyleTier::CityApartment);
}

#[test]
fn broke_eviction_has_nowhere_to_go_below_squat() {
    let mut game = test_game();
    game.player.money = -10; // already the cheapest tier, already broke

    game.update_lifestyle(&GameAction::WriteSongs);
    game.update_lifestyle(&GameAction::WriteSongs);

    assert_eq!(
        game.player.lifestyle,
        LifestyleTier::Squat,
        "nothing below Squat"
    );
}

#[test]
fn tabloids_penalize_high_fame_low_rent_living_once_per_streak() {
    let mut game = test_game();
    game.band.fame = 70;
    game.player.lifestyle = LifestyleTier::Squat;
    game.player.happiness = 50;
    game.player.money = 1_000;

    game.update_lifestyle(&GameAction::WriteSongs);
    assert_eq!(game.player.tabloid_streak, 1);
    assert_eq!(game.player.happiness, 48, "image penalty: -2 happiness");

    game.update_lifestyle(&GameAction::WriteSongs);
    assert_eq!(
        game.player.tabloid_streak, 2,
        "the streak continues while still low-rent and famous"
    );
    assert_eq!(game.player.happiness, 46);
}

#[test]
fn a_mansion_draws_no_image_penalty_regardless_of_fame() {
    let mut game = test_game();
    game.band.fame = 0; // low fame is the case the design calls out explicitly
    game.player.lifestyle = LifestyleTier::Mansion;
    game.player.happiness = 50;
    game.player.money = 10_000;

    game.update_lifestyle(&GameAction::WriteSongs);

    assert_eq!(game.player.tabloid_streak, 0);
}

#[test]
fn rest_healing_bonus_boosts_laze_around() {
    let mut game = test_game();
    game.player.lifestyle = LifestyleTier::Mansion; // rest-healing bonus +4
    game.player.stress = 50;
    game.player.health = 50;

    game.action_laze_around()
        .expect("laze around is always available");

    assert_eq!(
        game.player.stress,
        50 - (LAZE_STRESS_RELIEF + 4),
        "the Mansion's rest bonus adds to the stress relief"
    );
    assert_eq!(game.player.health, 50 + 4, "and tops up health too");
}

#[test]
fn rest_healing_bonus_boosts_take_break() {
    let mut game = test_game();
    game.player.lifestyle = LifestyleTier::Townhouse; // rest-healing bonus +3
    game.player.health = 50;

    game.action_take_break()
        .expect("take break is always available");

    assert_eq!(game.player.health, (50 + BREAK_HEALTH_GAIN + 3).min(100));
}
