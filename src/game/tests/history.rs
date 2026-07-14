//! Historical events reward or penalize by the band's actual genre.

use rand::SeedableRng;
use rand::rngs::StdRng;

use super::*;

/// Regression: `dominant_genres_match` once ignored the band's genre and
/// returned true for any non-empty list, so every dated event fired for
/// every band — the Beatles windfall paid everyone and the grunge shake-up
/// rewarded everyone while its hair-metal penalty was unreachable. Events
/// must now touch only the genres they name.
#[test]
fn historical_genre_events_respect_the_band_genre() {
    // Fame delta from applying `event` to a band playing `g` (fame moves are
    // deterministic; the money side draws from rng and isn't asserted).
    let delta = |event: &str, g: genre::MusicGenre| {
        let mut game = test_game();
        game.band.genre = g;
        game.band.fame = 50;
        let before = game.band.fame;
        game.apply_historical_event(event, &mut StdRng::seed_from_u64(0))
            .expect("historical events always apply");
        game.band.fame as i16 - before as i16
    };

    // "Grunge emerges": alternative acts surge, the hair-metal crowd sinks,
    // and an unrelated genre is left alone.
    assert!(
        delta("Grunge emerges", genre::MusicGenre::Alternative) > 0,
        "grunge should lift an alternative act"
    );
    assert!(
        delta("Grunge emerges", genre::MusicGenre::Metal) < 0,
        "grunge should bury the hair-metal crowd (penalty was unreachable)"
    );
    assert_eq!(
        delta("Grunge emerges", genre::MusicGenre::Pop),
        0,
        "an unrelated genre should be untouched by the grunge shake-up"
    );

    // The Beatles' break-up rewards rock, not everyone.
    assert!(
        delta("The Beatles break up", genre::MusicGenre::Rock) > 0,
        "a rock act should ride the post-Beatles moment"
    );
    assert_eq!(
        delta("The Beatles break up", genre::MusicGenre::Pop),
        0,
        "a pop act should not be paid by the Beatles event"
    );
}
