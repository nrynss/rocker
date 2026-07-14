//! Per-show engine integration tests (design §B). Pure reception/momentum
//! math is unit-tested alongside its implementation in `shows.rs`; these
//! exercise the real action guards, RNG wiring, and `last_tour_report`
//! storage through the shared harness (`actions/live.rs`).

use rand::SeedableRng;
use rand::rngs::StdRng;

use super::*;

#[test]
fn gig_guard_blocks_high_stress_and_low_health() {
    let mut game = test_game();
    let venue = best_open_venue(&game);
    let mut rng = StdRng::seed_from_u64(100);

    game.player.stress = GIG_STRESS_GUARD;
    assert!(
        game.action_play_gig(venue, &mut rng).is_err(),
        "a gig at the stress guard should be blocked"
    );

    game.player.stress = 0;
    game.player.health = GIG_HEALTH_GUARD - 1;
    assert!(
        game.action_play_gig(venue, &mut rng).is_err(),
        "a gig below the health guard should be blocked"
    );

    game.player.health = GIG_HEALTH_GUARD;
    assert!(
        game.action_play_gig(venue, &mut rng).is_ok(),
        "right at the health guard (not below it), a gig should be allowed"
    );
}

#[test]
fn tour_guard_blocks_high_stress_and_low_health() {
    let mut game = test_game();
    game.band.fame = 90; // clears every region's fame requirement
    game.player.money = 1_000_000;
    let mut rng = StdRng::seed_from_u64(101);

    game.player.stress = TOUR_STRESS_GUARD;
    assert!(
        game.action_go_on_tour(0, &mut rng).is_err(),
        "a tour at the stress guard should be blocked"
    );

    game.player.stress = 0;
    game.player.health = TOUR_HEALTH_GUARD - 1;
    assert!(
        game.action_go_on_tour(0, &mut rng).is_err(),
        "a tour below the health guard should be blocked"
    );
}

#[test]
fn a_seeded_tour_produces_five_shows_per_tour_week() {
    let mut game = test_game();
    game.band.fame = 90; // the top fame tier: 4 tour weeks
    game.player.money = 1_000_000;
    let mut rng = StdRng::seed_from_u64(102);

    game.action_go_on_tour(0, &mut rng)
        .expect("a well-off, famous band should be able to tour");

    let report = game.last_tour_report.expect("a tour populates the report");
    assert_eq!(
        report.rows.len(),
        4 * SHOWS_PER_TOUR_WEEK as usize,
        "a 4-tour-week outing should resolve 20 individual shows"
    );
    assert!(
        report.rows.iter().all(|row| row.capacity > 0),
        "every synthesized tour stop should have a real capacity"
    );
}

#[test]
fn a_gig_stores_a_single_row_report() {
    let mut game = test_game();
    let venue = best_open_venue(&game);
    let mut rng = StdRng::seed_from_u64(103);

    game.action_play_gig(venue, &mut rng)
        .expect("gig should succeed");

    let report = game
        .last_tour_report
        .expect("a gig populates the report too");
    assert_eq!(report.rows.len(), 1, "a one-off gig is a single-row report");
}

/// A band this good should land great-or-better nights often enough that a
/// handful of gigs feeds creativity at least once (design §A/§B).
#[test]
fn great_and_transcendent_shows_feed_creativity() {
    let mut game = test_game();
    for member in &mut game.band.members {
        member.skill = 100;
    }
    game.band.reputation.live_performance = 100;
    game.player.creativity = 50;
    let mut rng = StdRng::seed_from_u64(104);
    let venue = best_open_venue(&game);
    let creativity_before = game.player.creativity;

    let mut saw_a_boost = false;
    for _ in 0..5 {
        game.player.stress = 0;
        game.player.health = 100;
        game.action_play_gig(venue, &mut rng)
            .expect("gig should succeed");
        if game.player.creativity > creativity_before {
            saw_a_boost = true;
            break;
        }
    }
    assert!(
        saw_a_boost,
        "an excellent band should feed creativity from a great/transcendent night within 5 gigs"
    );
}

/// L12: `reputation.live_performance` is one of the two dominant reception
/// terms (design §B) and, before this fix, was never written after band
/// creation — a career's live show could never actually improve. Confirm
/// it now climbs with stage time.
#[test]
fn playing_shows_grows_live_performance_reputation() {
    let mut game = test_game();
    for member in &mut game.band.members {
        member.skill = 100;
    }
    game.band.reputation.live_performance = 50;
    game.player.stress = 0;
    game.player.health = 100;
    let venue = best_open_venue(&game);
    let mut rng = StdRng::seed_from_u64(105);
    let reputation_before = game.band.reputation.live_performance;

    for _ in 0..10 {
        game.player.stress = 0;
        game.player.health = 100;
        game.action_play_gig(venue, &mut rng)
            .expect("gig should succeed");
    }

    assert!(
        game.band.reputation.live_performance > reputation_before,
        "a run of gigs by a skilled band should raise live_performance from {reputation_before}, got {}",
        game.band.reputation.live_performance
    );
    assert!(
        game.band.reputation.live_performance <= 100,
        "live_performance must stay clamped at 100"
    );
}

/// L12: rehearsal is where individual musicianship — `average_member_skill()`,
/// the reception formula's other static term — was supposed to grow, and
/// didn't. Confirm Practice now raises it.
#[test]
fn practice_grows_average_member_skill() {
    let mut game = test_game();
    for member in &mut game.band.members {
        member.skill = 20;
    }
    game.player.stress = 0;
    let skill_before = game.band.average_member_skill();

    for _ in 0..5 {
        game.player.stress = 0;
        game.action_practice().expect("practice should succeed");
    }

    assert!(
        game.band.average_member_skill() > skill_before,
        "practice should raise average member skill from {skill_before}, got {}",
        game.band.average_member_skill()
    );
    assert!(
        game.band.members.iter().all(|m| m.skill <= 100),
        "member skill must stay clamped at 100"
    );
}

#[test]
fn old_save_defaults_last_tour_report_to_none() {
    let path = concat!(env!("CARGO_MANIFEST_DIR"), "/tests/fixtures/pre-0.5.sav");
    let game = Game::load_game(path).expect("a v0.4.0 save must keep loading");
    assert!(
        game.last_tour_report.is_none(),
        "a pre-0.6 save has no tour report on hand"
    );
}
