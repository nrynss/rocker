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
    assert_eq!(
        single.distribution_channel, None,
        "pre-M6 releases have no chosen channel — reach falls back to Mail order & gigs"
    );
    assert_eq!(
        game.current_distribution_channel,
        crate::game::music::DistributionChannel::MailOrder,
        "a save from before M6 has no chosen channel — defaults to Mail order & gigs"
    );
    assert!(game.pending_deal_offers.is_empty());

    // And the loaded game is playable, not merely parseable.
    game.process_turn(GameAction::LazeAround)
        .expect("a loaded v0.4.0 game must take a turn");
    assert_eq!(game.week, 14);
}

// ---------------------------------------------------------------------------
// Atomic saves (PR #52, CodeRabbit): Quit's save-first prompt writes over the
// file the player last loaded, so a save must replace the target whole or
// not at all, and must never touch any file but the target.
// ---------------------------------------------------------------------------

/// A fresh scratch directory for one test, so leftover-file checks can list
/// it without seeing other tests' files.
fn scratch_dir(name: &str) -> std::path::PathBuf {
    let dir = std::env::temp_dir().join(format!("rocker-atomic-{}-{name}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("scratch dir");
    dir
}

fn entries(dir: &std::path::Path) -> Vec<String> {
    let mut names: Vec<String> = std::fs::read_dir(dir)
        .expect("scratch dir lists")
        .map(|entry| {
            entry
                .expect("entry")
                .file_name()
                .to_string_lossy()
                .into_owned()
        })
        .collect();
    names.sort();
    names
}

/// A write that dies part-way leaves the old save byte for byte, and the
/// half-written temp file is cleaned up.
#[test]
fn a_save_that_fails_mid_write_leaves_the_old_save_intact() {
    use std::io::Write;
    let dir = scratch_dir("mid-write");
    let target = dir.join("career.sav");
    std::fs::write(&target, b"the old career").expect("seed the old save");

    let result = crate::game::core::write_atomically(&target, |file| {
        file.write_all(b"the new car")?;
        Err(std::io::Error::other("disk full"))
    });
    assert!(result.is_err(), "the failure is reported");
    assert_eq!(
        std::fs::read(&target).expect("old save still there"),
        b"the old career"
    );
    assert_eq!(
        entries(&dir),
        vec!["career.sav".to_string()],
        "no temp left"
    );
    let _ = std::fs::remove_dir_all(&dir);
}

/// The temp name is never derived into someone else's file: a career saved
/// as `career.sav.tmp` survives a save to `career.sav`, and a good save
/// leaves nothing behind but the two saves.
#[test]
fn a_save_never_touches_a_neighbouring_file() {
    let dir = scratch_dir("neighbour");
    let target = dir.join("career.sav");
    let neighbour = dir.join("career.sav.tmp");
    std::fs::write(&neighbour, b"another career").expect("seed the neighbour");

    let game = Game::new().expect("data files present");
    game.save_game(target.to_str().expect("utf-8 temp path"))
        .expect("the save works");
    assert_eq!(
        std::fs::read(&neighbour).expect("neighbour still there"),
        b"another career"
    );
    Game::load_game(target.to_str().unwrap()).expect("the new save loads");
    assert_eq!(
        entries(&dir),
        vec!["career.sav".to_string(), "career.sav.tmp".to_string()],
        "nothing else is left behind"
    );
    let _ = std::fs::remove_dir_all(&dir);
}
