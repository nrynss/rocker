//! Support-tour offers: accepting, declining, arrival conditions, and expiry.

use rand::SeedableRng;
use rand::rngs::StdRng;

use super::*;

#[test]
fn accepting_a_support_tour_pays_and_advances_time() {
    let mut game = test_game();
    game.band.fame = 10;
    game.player.money = 500;
    game.player.energy = 100;
    game.pending_support_offer = Some(SupportTourOffer {
        host_band: "Big Stars".to_string(),
        host_fame: 60,
        weeks: 3,
        pay: 1000,
        fame_gain: 6,
        expires_week: 10,
    });
    let week_before = game.week;

    let mut rng = StdRng::seed_from_u64(0);
    game.action_accept_support_tour(&mut rng)
        .expect("offer should be acceptable");

    assert!(game.pending_support_offer.is_none());
    assert_eq!(game.player.money, 1500);
    assert_eq!(game.week, week_before + 3);
    assert!(game.band.fame >= 16, "fame should include the offered gain");
    assert_eq!(game.player.energy, 65);
}

#[test]
fn declining_a_support_tour_clears_it() {
    let mut game = test_game();
    game.pending_support_offer = Some(SupportTourOffer {
        host_band: "Big Stars".to_string(),
        host_fame: 60,
        weeks: 2,
        pay: 500,
        fame_gain: 4,
        expires_week: 10,
    });

    game.action_decline_support_tour()
        .expect("decline should succeed");
    assert!(game.pending_support_offer.is_none());
    assert!(
        game.action_decline_support_tour().is_err(),
        "no offer left to decline"
    );
}

#[test]
fn support_offers_arrive_when_bigger_acts_exist() {
    let mut game = test_game();
    game.band.fame = 20;
    // Guarantee at least one act big enough to headline over the player.
    game.world.bands[0].fame = 80;

    let mut offered = false;
    let mut rng = StdRng::seed_from_u64(1);
    for week in 1..=200 {
        game.week = week;
        game.update_support_tour_offer(&mut rng);
        if game.pending_support_offer.is_some() {
            offered = true;
            break;
        }
    }
    assert!(
        offered,
        "200 weeks alongside a big act should produce at least one offer"
    );

    let offer = game.pending_support_offer.as_ref().unwrap();
    assert!(offer.host_fame >= game.band.fame + SUPPORT_OFFER_FAME_GAP);
    assert!(offer.pay > 0);
}

#[test]
fn stale_support_offers_expire() {
    let mut game = test_game();
    game.pending_support_offer = Some(SupportTourOffer {
        host_band: "Big Stars".to_string(),
        host_fame: 60,
        weeks: 2,
        pay: 500,
        fame_gain: 4,
        expires_week: 3,
    });
    game.week = 5;

    game.update_support_tour_offer(&mut StdRng::seed_from_u64(0));
    assert!(game.pending_support_offer.is_none(), "offers should expire");
    assert!(
        game.turn_log
            .iter()
            .any(|m| m.contains("went to another band")),
        "expiry should be reported"
    );
}
