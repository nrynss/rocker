//! Save/serde compatibility: pre-genre bands and real v0.4.0 saves keep loading.

use crate::game::band::Band;

use super::*;

#[test]
fn bands_saved_before_genres_existed_load_as_rock() {
    assert_eq!(Band::default().genre, genre::MusicGenre::Rock);

    // A pre-genre save is a Band JSON object with no "genre" key at all.
    let mut saved = serde_json::to_value(Band::default()).expect("bands serialize");
    saved
        .as_object_mut()
        .expect("a band serializes to an object")
        .remove("genre");
    let loaded: Band = serde_json::from_value(saved).expect("old saves must keep loading");
    assert_eq!(loaded.genre, genre::MusicGenre::Rock);
}

#[test]
fn saves_from_v0_4_still_load() {
    // A real save written by the v0.4.0 binary (f8e5eb9): a 13-week
    // career with one single released and songs in the drawer. It
    // predates idle_streak, genre_trend_reported, band genre, pressing
    // runs, and offer expiry — loading must fill every gap in.
    let path = concat!(env!("CARGO_MANIFEST_DIR"), "/tests/fixtures/pre-0.5.sav");
    let mut game = Game::load_game(path).expect("a v0.4.0 save must keep loading");

    // What the old binary wrote survives the trip.
    assert_eq!(game.week, 13);
    assert_eq!(game.player.money, 1029);
    assert_eq!(game.band.fame, 3);
    assert_eq!(game.band.singles_released.len(), 1);

    // Fields born in the 0.5 cycle take their documented defaults.
    assert_eq!(game.idle_streak, 0, "no idle history: decay starts fresh");
    assert_eq!(game.genre_trend_reported, 0);
    assert_eq!(
        game.band.genre,
        genre::MusicGenre::Rock,
        "pre-genre bands load as Rock"
    );
    let single = &game.band.singles_released[0];
    assert_eq!(single.copies_pressed, 0, "legacy releases stay uncapped");
    assert_eq!(single.copies_sold, 0);
    assert!(game.pending_deal_offers.is_empty());

    // And the loaded game is playable, not merely parseable.
    game.process_turn(GameAction::LazeAround)
        .expect("a loaded v0.4.0 game must take a turn");
    assert_eq!(game.week, 14);
}
