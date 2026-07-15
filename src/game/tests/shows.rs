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
        game.action_go_on_tour(0, TourRig::Van, 1, &mut rng)
            .is_err(),
        "a tour at the stress guard should be blocked"
    );

    game.player.stress = 0;
    game.player.health = TOUR_HEALTH_GUARD - 1;
    assert!(
        game.action_go_on_tour(0, TourRig::Van, 1, &mut rng)
            .is_err(),
        "a tour below the health guard should be blocked"
    );
}

#[test]
fn a_seeded_tour_produces_five_shows_per_tour_week() {
    let mut game = test_game();
    game.band.fame = 90; // clears every rig gate and the 4-week length gate
    game.player.money = 1_000_000;
    let mut rng = StdRng::seed_from_u64(102);

    game.action_go_on_tour(0, TourRig::Full, 4, &mut rng)
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

// --- M1: tour economics — rig picker, length picker, itemized up-front
// quote (design §A). Fame never re-prices a tour; it only gates rigs,
// lengths, and how many seats a tour fills. ---

/// The core guarantee of §A: same region + rig + length must quote (and
/// charge) identically no matter how famous the band is.
#[test]
fn tour_quote_cost_is_fame_independent_for_fixed_rig_and_length() {
    let mut game = test_game();
    let region_index = game
        .get_sorted_regions()
        .iter()
        .position(|(_, _, _, _, _, fame_req)| *fame_req <= 25)
        .expect("at least one region open at fame 25");

    game.band.fame = 25;
    let quote_low_fame = game
        .quote_tour(region_index, TourRig::Bus, 2)
        .expect("fame 25 clears both the region and the Tour bus gate");

    game.band.fame = 95;
    let quote_high_fame = game
        .quote_tour(region_index, TourRig::Bus, 2)
        .expect("fame 95 clears the same region and rig too");

    assert_eq!(
        quote_low_fame.cost, quote_high_fame.cost,
        "same region + rig + length must cost the same at any fame (design §A)"
    );
    assert_eq!(
        quote_low_fame.shows, quote_high_fame.shows,
        "shows booked depend only on length, never on fame"
    );
}

/// A bigger rig books bigger rooms, so its quoted gross ceiling must rise
/// with the capacity multiplier (design §A). Without this the capacity mult
/// would only scale reported attendance and a Full-production rig would
/// gross exactly what a van does for 50× the cost.
#[test]
fn a_bigger_rig_quotes_a_bigger_gross() {
    let mut game = test_game();
    game.band.fame = 95; // clears every rig gate
    let region_index = game
        .get_sorted_regions()
        .iter()
        .position(|(_, _, _, _, _, fame_req)| *fame_req <= 95)
        .expect("a region open at fame 95");

    let van = game
        .quote_tour(region_index, TourRig::Van, 2)
        .expect("van quote");
    let full = game
        .quote_tour(region_index, TourRig::Full, 2)
        .expect("full-production quote");

    assert!(
        full.gross_high > van.gross_high && full.gross_low > van.gross_low,
        "full production (capacity ×1.7) must out-gross the van (×0.8): \
         van {}–{}, full {}–{}",
        van.gross_low,
        van.gross_high,
        full.gross_low,
        full.gross_high
    );
    assert!(
        full.cost > van.cost,
        "and it must cost more, so the bigger gross is a real trade-off"
    );
}

/// Fame gates which rigs and lengths are selectable; it never re-prices one
/// (design §A table: Van —, Bus 25, Truck 55, Full 75; 3wk needs 40, 4wk
/// needs 60).
#[test]
fn rigs_and_lengths_are_fame_gated_not_priced() {
    let mut game = test_game();

    game.band.fame = 0;
    assert!(game.rig_is_available(TourRig::Van), "the van has no gate");
    assert!(!game.rig_is_available(TourRig::Bus));
    assert!(!game.rig_is_available(TourRig::Truck));
    assert!(!game.rig_is_available(TourRig::Full));
    assert!(game.tour_length_is_available(1));
    assert!(game.tour_length_is_available(2));
    assert!(!game.tour_length_is_available(3));
    assert!(!game.tour_length_is_available(4));

    game.band.fame = 40;
    assert!(game.tour_length_is_available(3), "3 weeks opens at fame 40");
    assert!(!game.tour_length_is_available(4));

    game.band.fame = 60;
    assert!(game.rig_is_available(TourRig::Bus));
    assert!(game.rig_is_available(TourRig::Truck));
    assert!(!game.rig_is_available(TourRig::Full));
    assert!(game.tour_length_is_available(4), "4 weeks opens at fame 60");

    game.band.fame = 75;
    assert!(
        game.rig_is_available(TourRig::Full),
        "full production opens at fame 75"
    );
}

/// The quote is never a guess: booking must charge exactly the cost the
/// quote showed, with the rest of the money movement accounted for by the
/// realized gross (design §A — "never a surprise").
#[test]
fn booking_a_tour_charges_exactly_the_quoted_cost() {
    let mut game = test_game();
    game.band.fame = 90;
    game.player.money = 1_000_000;
    let mut rng = StdRng::seed_from_u64(200);

    let region_index = 0;
    let quote = game
        .quote_tour(region_index, TourRig::Truck, 3)
        .expect("quote should resolve at fame 90");

    let money_before = game.player.money;
    game.action_go_on_tour(region_index, TourRig::Truck, 3, &mut rng)
        .expect("a well-off, famous band should be able to book this tour");

    let report = game.last_tour_report.expect("tour produces a report");
    let gross_sum: i32 = report.rows.iter().map(|r| r.take as i32).sum();

    assert_eq!(
        game.player.money,
        money_before - quote.cost + gross_sum,
        "money actually charged/paid must match the quote's cost exactly"
    );
}

/// Booking a tour that is projected to lose money is allowed — it buys
/// fame and regional fame — as long as the player can afford the cost
/// (design §A).
#[test]
fn a_money_losing_tour_can_still_be_booked() {
    let mut game = test_game();
    game.band.fame = 90;
    game.player.money = 1_000_000;
    let mut rng = StdRng::seed_from_u64(201);

    // Smallest-population region + the priciest rig/length combo: cost
    // should swamp the projected gross.
    let sorted_regions = game.get_sorted_regions();
    let region_index = sorted_regions
        .iter()
        .enumerate()
        .min_by_key(|(_, (_, _, _, population, _, _))| *population)
        .map(|(i, _)| i)
        .expect("at least one region exists");

    let quote = game
        .quote_tour(region_index, TourRig::Full, 4)
        .expect("quote should resolve");
    assert!(
        quote.cost as u32 > quote.gross_high,
        "test setup should pick a money-losing scenario, got cost {} vs gross_high {}",
        quote.cost,
        quote.gross_high
    );

    assert!(
        game.action_go_on_tour(region_index, TourRig::Full, 4, &mut rng)
            .is_ok(),
        "booking a money-losing tour must still be allowed (design §A)"
    );
}

/// The projected gross range widens by exactly the reception-driven
/// attendance spread (`RECEPTION_ATTENDANCE_MIN_FACTOR`/`MAX_FACTOR`) —
/// the quote's "at momentum 1.0, ± the reception spread" (design §A).
#[test]
fn quote_gross_range_matches_the_reception_attendance_spread() {
    let mut game = test_game();
    game.band.fame = 90;
    let quote = game
        .quote_tour(0, TourRig::Full, 4)
        .expect("quote should resolve");

    assert!(
        quote.gross_low > 0,
        "a high-fame tour should quote a nonzero gross floor"
    );
    let ratio = quote.gross_high as f32 / quote.gross_low as f32;
    let expected_ratio = RECEPTION_ATTENDANCE_MAX_FACTOR / RECEPTION_ATTENDANCE_MIN_FACTOR;
    assert!(
        (ratio - expected_ratio).abs() < 0.01,
        "gross range should widen by the reception attendance spread: got {ratio}, expected ~{expected_ratio}"
    );
}
