//! Fame dynamics: live-show ceilings, outgrown venues, and idle decay.

use super::*;

#[test]
fn gigging_alone_cannot_make_you_a_star() {
    let mut game = test_game();
    game.band.fame = 0;

    for _ in 0..300 {
        game.player.energy = 100;
        let venue = best_open_venue(&game);
        game.action_play_gig(venue).expect("gig should succeed");
    }

    assert_eq!(
        game.band.fame, LIVE_FAME_BASE_CAP,
        "with no records, live shows should stall at the base cap"
    );
}

#[test]
fn records_raise_the_live_fame_cap() {
    let mut game = test_game();
    game.band.fame = LIVE_FAME_BASE_CAP;
    game.player.energy = 100;

    let venue = best_open_venue(&game);
    game.action_play_gig(venue).expect("gig should succeed");
    assert_eq!(
        game.band.fame, LIVE_FAME_BASE_CAP,
        "at the cap, another gig adds nothing"
    );

    game.band
        .albums_released
        .push(test_release(1, ReleaseType::Album));
    game.band
        .singles_released
        .push(test_release(2, ReleaseType::Single));
    game.player.energy = 100;
    game.action_play_gig(venue).expect("gig should succeed");
    assert!(
        game.band.fame > LIVE_FAME_BASE_CAP,
        "records should lift the live ceiling"
    );
}

#[test]
fn an_outgrown_venue_adds_no_fame() {
    let mut game = test_game();
    for id in 0..6 {
        game.band
            .albums_released
            .push(test_release(id, ReleaseType::Album));
    }
    game.band.fame = 30; // past the pub's ceiling of prestige 10 + headroom 15
    game.player.energy = 100;

    let smallest = (0..game.world.venues.len())
        .min_by_key(|&i| game.world.venues[i].capacity)
        .expect("venues exist");
    game.action_play_gig(smallest).expect("gig should succeed");

    assert_eq!(game.band.fame, 30, "an outgrown stage draws no new fans");
}

// Adapted for v0.6 fame gravity (§C): the flat 1-week grace / −1-per-week
// model is gone. Fame 30 sits in the 30–49 tier — eight quiet weeks are
// forgiven — and its floor (earned at peak 30) is 10.
#[test]
fn idle_weeks_erode_fame_after_a_grace_week() {
    let mut game = test_game();
    game.band.fame = 30;

    game.update_public_visibility(&GameAction::LazeAround, 8);
    assert_eq!(
        game.band.fame, 30,
        "the whole grace window (8 weeks at this fame) is forgiven"
    );

    game.update_public_visibility(&GameAction::LazeAround, 1);
    assert_eq!(
        game.band.fame, 29,
        "the first week past grace costs one fame"
    );

    game.update_public_visibility(&GameAction::Gig(0), 1);
    assert_eq!(game.idle_streak, 0, "a show resets the idle streak");
}

// The decided worked example (§C — The ramp): fame 15, fully idle, nothing on
// the shelves. Two quiet weeks are forgiven, then the ramp bites −1, −2, −3,
// −4, −5 — reaching 0 at the end of week 7.
#[test]
fn the_worked_example_fame_fifteen_fades_to_zero_by_week_seven() {
    let mut game = test_game();
    game.band.fame = 15;

    let expected = [15, 15, 14, 12, 9, 5, 0];
    for (week, want) in expected.iter().enumerate() {
        game.update_public_visibility(&GameAction::LazeAround, 1);
        assert_eq!(
            game.band.fame,
            *want,
            "after idle week {} fame should be {}",
            week + 1,
            want
        );
    }
    assert_eq!(
        game.band.fame, 0,
        "fully idle from fame 15 lands on 0 at week 7"
    );
}

// Floors are permanent and earned at peak (§C — Floors). A band that once
// reached 75 keeps a floor of 45 no matter how long it disappears.
#[test]
fn a_peak_band_never_falls_below_its_floor() {
    let mut game = test_game();
    game.band.fame = 75; // peak 75 → floor 45

    // A year and a half of total silence — well past any grace and ramp.
    game.update_public_visibility(&GameAction::LazeAround, 78);
    assert_eq!(
        game.band.fame, 45,
        "decay stops dead at the floor earned at peak 75"
    );

    // Still nothing: the floor holds.
    game.update_public_visibility(&GameAction::LazeAround, 52);
    assert_eq!(game.band.fame, 45, "the floor is permanent");
}

// Comeback rule (§C): while below the peak already stood on, every gain
// doubles; at or above the peak, gains are normal and the peak tracks up.
#[test]
fn comeback_gains_are_doubled_below_peak() {
    use crate::game::band::Band;

    let mut band = Band {
        fame: 40,
        peak_fame: 60,
        ..Band::default()
    };

    band.gain_fame(5);
    assert_eq!(band.fame, 50, "below the peak, a +5 gain counts double");

    band.gain_fame(5);
    assert_eq!(band.fame, 60, "still doubled until fame catches the peak");

    band.gain_fame(5);
    assert_eq!(band.fame, 65, "at the peak, gains are normal again");
    assert_eq!(band.peak_fame, 65, "new highs raise the peak");
}

// Establishment rule (§C — Activity, rule 3): at fame ≥ 60 a recent release
// keeps the idle clock frozen — no decay while the record is fresh.
#[test]
fn the_establishment_rule_freezes_the_idle_clock() {
    let mut game = test_game();
    game.band.fame = 60;
    let mut album = test_release(1, ReleaseType::Album);
    album.week_released = game.week; // fresh off the presses
    game.band.albums_released.push(album);

    game.update_public_visibility(&GameAction::LazeAround, 30);
    assert_eq!(
        game.band.fame, 60,
        "an established act with a recent release stays in the picture"
    );
    assert_eq!(game.idle_streak, 0, "the idle clock never started");
}

#[test]
fn a_release_on_the_shelves_keeps_the_band_visible() {
    let mut game = test_game();
    game.band.fame = 30;
    game.just_released_music
        .push(test_release(1, ReleaseType::Single));

    game.update_public_visibility(&GameAction::LazeAround, 5);

    assert_eq!(
        game.band.fame, 30,
        "a record in its sales window counts as visibility"
    );
    assert_eq!(game.idle_streak, 0);
}
