//! Record-deal offer lifecycle: expiry, the resuming stream, and legacy saves.

use rand::SeedableRng;
use rand::rngs::StdRng;

use super::*;

#[test]
fn ignored_deal_offers_expire_and_the_stream_resumes() {
    let mut game = test_game();
    game.pending_deal_offers = vec![test_deal_offer(&game, Some(8))];
    let unsigned_before = game
        .world
        .bands
        .iter()
        .filter(|b| b.label.is_none())
        .count();

    // Before the deadline the offer stays on the table.
    game.week = 7;
    game.check_and_generate_deal_offers(&mut StdRng::seed_from_u64(0));
    assert_eq!(
        game.pending_deal_offers.len(),
        1,
        "a live offer survives to its deadline"
    );

    // At the deadline it quietly leaves — with a line in the log...
    game.week = 8;
    game.check_and_generate_deal_offers(&mut StdRng::seed_from_u64(0));
    assert!(
        game.pending_deal_offers.is_empty(),
        "an ignored offer should expire"
    );
    let log = game.take_turn_log().join("\n");
    assert!(
        log.contains("interest has cooled"),
        "expiry is told in-fiction, got: {log}"
    );
    // ...and, unlike a rejection, nobody poaches the vacated deal.
    let unsigned_after = game
        .world
        .bands
        .iter()
        .filter(|b| b.label.is_none())
        .count();
    assert_eq!(
        unsigned_before, unsigned_after,
        "expiry must not hand the deal to a scene act"
    );

    // With the slate clear and a catalog worth scouting, the stream
    // resumes on the next 4-week beat instead of staying silent forever.
    game.band.fame = 30;
    game.band
        .singles_released
        .push(test_release(1, ReleaseType::Single));
    let mut resumed = false;
    for attempt in 0..80 {
        game.check_and_generate_deal_offers(&mut StdRng::seed_from_u64(attempt));
        if !game.pending_deal_offers.is_empty() {
            resumed = true;
            break;
        }
    }
    assert!(resumed, "new offers should arrive once the slate is clear");
    assert!(
        game.pending_deal_offers
            .iter()
            .all(|offer| offer.expires_week == Some(game.week + DEAL_OFFER_LIFETIME_WEEKS)),
        "fresh offers should carry a deadline"
    );
}

#[test]
fn deal_offers_from_old_saves_never_expire() {
    // Offers already pending when an old save was written carry no
    // deadline; they stay on the table however late it gets.
    let mut game = test_game();
    game.pending_deal_offers = vec![test_deal_offer(&game, None)];
    game.week = 501;
    game.check_and_generate_deal_offers(&mut StdRng::seed_from_u64(0));
    assert_eq!(
        game.pending_deal_offers.len(),
        1,
        "legacy offers must never expire"
    );

    // And the on-disk shape old builds wrote — no expires_week key at
    // all — must deserialize to exactly that.
    let mut on_disk = serde_json::to_value(test_deal_offer(&game, Some(9))).unwrap();
    on_disk.as_object_mut().unwrap().remove("expires_week");
    let loaded: PotentialDealOffer = serde_json::from_value(on_disk).unwrap();
    assert_eq!(loaded.expires_week, None);
}
