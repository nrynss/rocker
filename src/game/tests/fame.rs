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

#[test]
fn idle_weeks_erode_fame_after_a_grace_week() {
    let mut game = test_game();
    game.band.fame = 30;

    game.update_public_visibility(&GameAction::LazeAround, 1);
    assert_eq!(game.band.fame, 30, "the first quiet week is forgiven");

    game.update_public_visibility(&GameAction::LazeAround, 1);
    game.update_public_visibility(&GameAction::LazeAround, 1);
    assert_eq!(
        game.band.fame, 28,
        "every idle week past the grace costs fame"
    );

    game.update_public_visibility(&GameAction::Gig(0), 1);
    assert_eq!(game.idle_streak, 0, "a show resets the idle streak");
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
