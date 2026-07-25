//! S-TIME (design v0.9 §F): the calendar never skips.
//!
//! `Game::week` moves in exactly one place — `process_turn` — and by exactly
//! one. Tours, support slots and breaks are `OngoingActivity` state resolved
//! a week at a time, with the whole weekly tick running in between.
//!
//! Two properties carry this file. **Calendar integrity**: over a long
//! career, weeks elapsed equals the number of turns that consumed a week —
//! the assertion that makes §F true rather than aspirational. And
//! **economic neutrality**: a tour's money, fame and report rows are the
//! pre-split whole-tour loop's, *spread* rather than altered.

use crate::game::actions::TourRig;

use super::*;

// ---------------------------------------------------------------------------
// Fixtures
// ---------------------------------------------------------------------------

/// Silence both story-event taps so a run exercises only the calendar:
/// historical events and random incidents both hand out money and fame, and
/// a money-exact comparison can't survive either. (Same idiom as the sim
/// harness; the incident clock is re-parked after every turn by
/// `park_events`, since `try_trigger_event` writes to it.)
fn suppress_story_events(game: &mut Game) {
    let every_event: Vec<String> = game
        .timeline
        .eras
        .values()
        .flat_map(|era| era.major_events.iter().cloned())
        .collect();
    game.timeline.triggered_events.extend(every_event);
    park_events(game);
}

fn park_events(game: &mut Game) {
    game.events.last_event_week = u32::MAX;
}

/// The band the tour-equivalence tests book with: famous enough for a
/// four-week run on the Tour bus, rich enough that cost never gates the
/// booking, and living in a Squat so the weekly tick charges no rent — which
/// is what lets the money delta be compared to the penny.
///
/// The three archived albums are there for `live_fame_cap` alone: without a
/// catalogue the live ceiling is `LIVE_FAME_BASE_CAP` (35), a fame-60 band
/// is already past it, and the tour's fame settlement would be a no-op that
/// the equivalence test could not see. They were never pressed and never
/// charted, so the catalogue tail pays nothing and the money stays exact —
/// which the equivalence test itself would catch if it ever stopped being
/// true.
fn touring_band(seed: u64) -> Game {
    let mut game = crate::game::sim::seeded_game(seed);
    suppress_story_events(&mut game);
    game.band.fame = 60; // clears the 4-week length gate and the Bus rig gate
    game.band.peak_fame = 60;
    game.band.albums_released = vec![
        test_release(901, ReleaseType::Album),
        test_release(902, ReleaseType::Album),
        test_release(903, ReleaseType::Album),
    ];
    game.band.skill = 50;
    for member in &mut game.band.members {
        member.skill = 50;
    }
    game.band.reputation.live_performance = 40;
    game.player.stress = 20;
    game.player.health = 90;
    game.player.creativity = 40;
    game.player.money = 200_000;
    game
}

const TOUR_REGION: usize = 0;
const TOUR_RIG: TourRig = TourRig::Bus;
const TOUR_WEEKS: u8 = 4;

/// The economically meaningful part of a resolved night. `ShowReport::week`
/// is deliberately excluded: it is the one field the split *should* change,
/// because the whole-tour reference never moves the calendar while a real
/// run does (see `tour_rows_are_stamped_with_the_week_they_were_played`).
type Night = (String, String, u8, u32, u32, u32);

fn nights(report: &TourReport) -> Vec<Night> {
    report
        .rows
        .iter()
        .map(|row| {
            (
                row.venue_name.clone(),
                row.verdict.clone(),
                row.reception,
                row.attendance,
                row.capacity,
                row.take,
            )
        })
        .collect()
}

/// One tour, resolved however `drive` says, reduced to what §F promises not
/// to change: the money that moved, the fame that landed, the territory
/// built, and every night of the report.
struct TourOutcome {
    money_delta: i64,
    fame_delta: i16,
    regional_fame: u8,
    total_gross: u32,
    nights: Vec<Night>,
}

fn tour_outcome(game: &Game, money_before: i64, fame_before: u8) -> TourOutcome {
    let report = game
        .last_tour_report
        .as_ref()
        .expect("a tour always leaves a report");
    let regional_fame = game.regional_fame.values().copied().max().unwrap_or(0);
    TourOutcome {
        money_delta: game.player.money as i64 - money_before,
        fame_delta: game.band.fame as i16 - fame_before as i16,
        regional_fame,
        total_gross: report.total_gross,
        nights: nights(report),
    }
}

// ---------------------------------------------------------------------------
// §F — the economics are spread, not altered
// ---------------------------------------------------------------------------

/// **The task's central assertion.** Resolving a tour five nights a week for
/// four weeks, with the whole weekly tick running in between, must produce
/// exactly what resolving twenty nights in one sitting produced before the
/// split: the same money delta, the same fame, the same regional fame, and
/// the same twenty rows.
///
/// The reference run (`finish_activity_in_one_sitting`) *is* the pre-0.9
/// algorithm — twenty nights back to back off one momentum chain, settling
/// once at the end — so this compares the split against the thing it
/// replaced rather than against a remembered number. The pins in
/// `a_booked_tour_holds_its_numbers` freeze the values themselves.
///
/// It holds because of two decisions: `tour_pot` is evaluated once, at
/// booking, and frozen into the activity (cost, pot, per-show share, fame
/// and regional-fame gains all come from there); and the nights roll on the
/// **activity stream**, keyed by night index rather than by position in a
/// running stream, so the weekly world tick that now runs between groups
/// cannot shift them.
#[test]
fn a_split_tour_pays_exactly_what_the_whole_tour_paid() {
    for seed in [7u64, 11, 2025] {
        // Reference: the pre-split whole-tour loop, no week passing under it.
        let mut reference = touring_band(seed);
        let money_before = reference.player.money as i64;
        let fame_before = reference.band.fame;
        reference
            .action_go_on_tour(TOUR_REGION, TOUR_RIG, TOUR_WEEKS)
            .expect("the fixture band can afford this tour");
        finish_activity_in_one_sitting(&mut reference);
        let whole = tour_outcome(&reference, money_before, fame_before);

        // The real thing: one week per turn, weekly tick and all.
        let mut split = touring_band(seed);
        assert_eq!(split.player.money as i64, money_before);
        split
            .process_turn(GameAction::GoOnTour(TOUR_REGION, TOUR_RIG, TOUR_WEEKS))
            .expect("the fixture band can afford this tour");
        for _ in 1..TOUR_WEEKS {
            park_events(&mut split);
            split
                .process_turn(GameAction::LazeAround)
                .expect("a committed band's turn always resolves");
        }
        assert!(
            split.ongoing_activity.is_none(),
            "the last week settles the tour and clears it (seed {seed})"
        );
        let spread = tour_outcome(&split, money_before, fame_before);

        assert_eq!(
            spread.nights, whole.nights,
            "every night must resolve identically however the calendar chopped \
             the tour up (seed {seed})"
        );
        assert_eq!(
            spread.total_gross, whole.total_gross,
            "box office must be spread, not altered (seed {seed})"
        );
        assert_eq!(
            spread.money_delta, whole.money_delta,
            "the tour's total money delta must be untouched by the split (seed {seed})"
        );
        assert_eq!(
            spread.fame_delta, whole.fame_delta,
            "fame is a whole-tour settlement either way (seed {seed})"
        );
        assert_eq!(
            spread.regional_fame, whole.regional_fame,
            "the territory built must not depend on the calendar (seed {seed})"
        );
    }
}

/// The quote is still the contract (§A): booking charges exactly what it
/// showed, up front, and nothing in the split re-prices it.
#[test]
fn a_booked_tour_charges_the_quote_up_front_and_pays_out_weekly() {
    let mut game = touring_band(2025);
    let quote = game
        .quote_tour(TOUR_REGION, TOUR_RIG, TOUR_WEEKS)
        .expect("the fixture band clears every gate");
    let money_before = game.player.money;

    game.process_turn(GameAction::GoOnTour(TOUR_REGION, TOUR_RIG, TOUR_WEEKS))
        .expect("booking should succeed");

    // After week one the whole cost is spent but only a quarter of the
    // nights have been played, so the balance is cost-minus-week-one.
    let week_one_gross: u32 = game
        .ongoing_activity
        .as_ref()
        .expect("still on the road")
        .rows_so_far()
        .iter()
        .map(|row| row.take)
        .sum();
    assert_eq!(
        game.player.money,
        money_before - quote.cost + week_one_gross as i32,
        "the whole booking cost is due at booking (§B), the box office arrives weekly"
    );

    for _ in 1..TOUR_WEEKS {
        park_events(&mut game);
        game.process_turn(GameAction::LazeAround)
            .expect("a committed band's turn always resolves");
    }
    let report = game.last_tour_report.expect("tour produces a report");
    assert_eq!(
        game.player.money,
        money_before - quote.cost + report.total_gross as i32,
        "and over the run the arithmetic is still cost against gross, nothing else"
    );
    assert_eq!(
        report.rows.len(),
        (TOUR_WEEKS as u32 * SHOWS_PER_TOUR_WEEK) as usize,
        "five nights a week, every week"
    );
}

/// Pinned values for one exact booking, so the split's arithmetic can't
/// drift silently under a later tuning pass without someone deciding to
/// change them. Seed 2025, region 0, Tour bus, four weeks, off the fixture
/// band above.
#[test]
fn a_booked_tour_holds_its_numbers() {
    let mut game = touring_band(2025);
    let money_before = game.player.money as i64;
    let fame_before = game.band.fame;

    game.process_turn(GameAction::GoOnTour(TOUR_REGION, TOUR_RIG, TOUR_WEEKS))
        .expect("booking should succeed");
    for _ in 1..TOUR_WEEKS {
        park_events(&mut game);
        game.process_turn(GameAction::LazeAround)
            .expect("a committed band's turn always resolves");
    }

    let outcome = tour_outcome(&game, money_before, fame_before);
    assert_eq!(outcome.total_gross, PINNED_TOUR_GROSS, "tour gross");
    assert_eq!(outcome.money_delta, PINNED_TOUR_MONEY_DELTA, "money delta");
    assert_eq!(outcome.fame_delta, PINNED_TOUR_FAME_DELTA, "fame delta");
    assert_eq!(
        outcome.regional_fame, PINNED_TOUR_REGIONAL_FAME,
        "regional fame"
    );
    let takes: Vec<u32> = outcome.nights.iter().map(|night| night.5).collect();
    assert_eq!(takes, PINNED_TOUR_TAKES, "per-night takes");
    let receptions: Vec<u8> = outcome.nights.iter().map(|night| night.2).collect();
    assert_eq!(receptions, PINNED_TOUR_RECEPTIONS, "per-night receptions");
}

// Captured 2026-07-25 from the whole-tour reference path — the pre-split
// algorithm, twenty nights off one momentum chain — and reproduced to the
// unit by the split run above. `money_delta` is `gross − cost` exactly
// (12,082 − 7,762): a Squat charges no rent, the archived catalogue sells
// nothing, and both story-event taps are silenced, so the tour is the only
// thing moving money over the four weeks.
const PINNED_TOUR_GROSS: u32 = 12_082;
const PINNED_TOUR_MONEY_DELTA: i64 = 4_320;
const PINNED_TOUR_FAME_DELTA: i16 = 11;
const PINNED_TOUR_REGIONAL_FAME: u8 = 22;
const PINNED_TOUR_TAKES: [u32; 20] = [
    586, 601, 593, 607, 605, 588, 582, 605, 593, 607, 591, 608, 610, 603, 605, 620, 612, 625, 627,
    614,
];
const PINNED_TOUR_RECEPTIONS: [u8; 20] = [
    55, 64, 59, 67, 66, 56, 53, 66, 59, 67, 58, 68, 69, 65, 66, 75, 60, 67, 68, 61,
];

/// A report row's `week` is the week the show was actually played — which is
/// the whole point of §F. Five nights land in each of the four weeks.
#[test]
fn tour_rows_are_stamped_with_the_week_they_were_played() {
    let mut game = touring_band(11);
    let booking_week = game.week;
    game.process_turn(GameAction::GoOnTour(TOUR_REGION, TOUR_RIG, TOUR_WEEKS))
        .expect("booking should succeed");
    for _ in 1..TOUR_WEEKS {
        park_events(&mut game);
        game.process_turn(GameAction::LazeAround)
            .expect("a committed band's turn always resolves");
    }

    let report = game.last_tour_report.expect("tour produces a report");
    let weeks: Vec<u32> = report.rows.iter().map(|row| row.week).collect();
    let expected: Vec<u32> = (0..TOUR_WEEKS as u32)
        .flat_map(|offset| std::iter::repeat_n(booking_week + offset, SHOWS_PER_TOUR_WEEK as usize))
        .collect();
    assert_eq!(weeks, expected);
}

// ---------------------------------------------------------------------------
// §F — one week, always
// ---------------------------------------------------------------------------

/// A four-week tour takes four turns and four weeks — not one turn and five
/// weeks, which is what `week += weeks` plus `process_turn`'s own increment
/// used to cost.
#[test]
fn a_tour_plays_one_week_per_turn() {
    let mut game = touring_band(7);
    let booking_week = game.week;

    game.process_turn(GameAction::GoOnTour(TOUR_REGION, TOUR_RIG, TOUR_WEEKS))
        .expect("booking should succeed");
    for week in 1..TOUR_WEEKS {
        assert_eq!(
            game.week,
            booking_week + week as u32,
            "each turn of a tour is exactly one week"
        );
        let activity = game.ongoing_activity.as_ref().expect("still on the road");
        assert_eq!(activity.weeks_done(), week);
        assert_eq!(activity.week_number(), week + 1);
        assert_eq!(activity.progress(), format!("week {} of 4", week + 1));
        park_events(&mut game);
        game.process_turn(GameAction::LazeAround)
            .expect("a committed band's turn always resolves");
    }

    assert!(game.ongoing_activity.is_none(), "the tour has settled");
    assert_eq!(
        game.week,
        booking_week + TOUR_WEEKS as u32,
        "four booked weeks cost exactly four weeks of calendar"
    );
}

#[test]
fn a_break_plays_its_weeks_one_at_a_time() {
    let mut game = touring_band(3);
    let booking_week = game.week;
    game.player.stress = 60;
    game.player.health = 50;

    game.process_turn(GameAction::TakeBreak)
        .expect("a break always works");
    for week in 1..BREAK_WEEKS {
        assert!(
            game.ongoing_activity.is_some(),
            "still away in week {}",
            week + 1
        );
        assert!(
            game.player.stress > 0,
            "the recovery lands on the way back, not on the way out"
        );
        park_events(&mut game);
        game.process_turn(GameAction::WriteSongs)
            .expect("a committed band's turn always resolves");
    }

    assert!(game.ongoing_activity.is_none(), "the break has ended");
    assert_eq!(game.week, booking_week + BREAK_WEEKS);
    assert_eq!(game.player.stress, 0, "a break still resets stress");
    assert!(
        game.player.health > 50,
        "and still heals — {} health",
        game.player.health
    );
}

#[test]
fn a_support_tour_plays_its_weeks_one_at_a_time() {
    let mut game = touring_band(5);
    let booking_week = game.week;
    game.pending_support_offer = Some(SupportTourOffer {
        host_band: "Big Stars".to_string(),
        host_fame: 90,
        weeks: 3,
        pay: 1_000,
        fame_gain: 6,
        expires_week: booking_week + 10,
    });
    let money_before = game.player.money;
    let fame_before = game.band.fame;

    game.process_turn(GameAction::AcceptSupportTour)
        .expect("the offer is on the table");
    assert_eq!(
        game.player.money,
        money_before + 1_000,
        "the guarantee is banked on acceptance"
    );
    assert_eq!(
        game.band.fame, fame_before,
        "the exposure is what finishing the run buys"
    );

    for _ in 1..3 {
        park_events(&mut game);
        game.process_turn(GameAction::LazeAround)
            .expect("a committed band's turn always resolves");
    }

    assert!(game.ongoing_activity.is_none());
    assert_eq!(game.week, booking_week + 3);
    assert!(game.band.fame > fame_before, "the run paid its exposure");
}

/// §F: while a commitment runs it owns the turn. A stale keypress can
/// neither do something else nor start a second commitment on top.
#[test]
fn a_committed_band_ignores_the_submitted_action() {
    let mut game = touring_band(13);
    game.process_turn(GameAction::GoOnTour(TOUR_REGION, TOUR_RIG, TOUR_WEEKS))
        .expect("booking should succeed");

    let songs_before = game.band.unreleased_songs.len();
    let week_before = game.week;
    park_events(&mut game);
    game.process_turn(GameAction::WriteSongs)
        .expect("the turn resolves — as another week of the tour");

    assert_eq!(
        game.band.unreleased_songs.len(),
        songs_before,
        "nobody writes songs from the back of the van"
    );
    assert_eq!(game.week, week_before + 1);
    assert!(game.ongoing_activity.is_some(), "still on the road");
}

/// The world genuinely keeps turning: rent falls due every week of a tour,
/// not once for the whole run. This is the bug §F exists to fix, stated as
/// an assertion.
#[test]
fn the_rent_falls_due_every_week_of_a_tour() {
    let mut game = touring_band(17);
    game.player.lifestyle = crate::game::player::LifestyleTier::CityApartment;
    let upkeep = game.player.lifestyle.upkeep_per_week() as i64;
    assert!(upkeep > 0, "test setup: a tier that actually charges rent");

    let money_before = game.player.money as i64;
    game.process_turn(GameAction::GoOnTour(TOUR_REGION, TOUR_RIG, TOUR_WEEKS))
        .expect("booking should succeed");
    for _ in 1..TOUR_WEEKS {
        park_events(&mut game);
        game.process_turn(GameAction::LazeAround)
            .expect("a committed band's turn always resolves");
    }

    let report = game
        .last_tour_report
        .clone()
        .expect("tour produces a report");
    let quote_cost = i64::from(
        game.quote_tour(TOUR_REGION, TOUR_RIG, TOUR_WEEKS)
            .expect("the quote still resolves")
            .cost,
    );
    let expected =
        money_before - quote_cost + i64::from(report.total_gross) - upkeep * i64::from(TOUR_WEEKS);
    assert_eq!(
        game.player.money as i64, expected,
        "four weeks away should cost four weeks of rent, not one"
    );
}

// ---------------------------------------------------------------------------
// §F — abandoning
// ---------------------------------------------------------------------------

#[test]
fn walking_out_of_a_tour_keeps_what_resolved_and_forfeits_the_rest() {
    let mut game = touring_band(19);
    let fame_before = game.band.fame;
    game.process_turn(GameAction::GoOnTour(TOUR_REGION, TOUR_RIG, TOUR_WEEKS))
        .expect("booking should succeed");
    park_events(&mut game);
    game.process_turn(GameAction::LazeAround)
        .expect("a second week on the road");

    let week_before = game.week;
    let money_before = game.player.money;
    game.process_turn(GameAction::AbandonActivity)
        .expect("walking out is always allowed");

    assert!(game.ongoing_activity.is_none(), "the tour is off the books");
    assert_eq!(
        game.week, week_before,
        "the week you walk out of is yours to spend — no week consumed"
    );
    assert_eq!(
        game.player.money, money_before,
        "no refund, and no further box office"
    );
    let report = game
        .last_tour_report
        .clone()
        .expect("the partial run still reports");
    assert_eq!(
        report.rows.len(),
        2 * SHOWS_PER_TOUR_WEEK as usize,
        "ten nights played, ten nights kept"
    );
    assert_eq!(
        game.band.fame, fame_before,
        "the run's fame was the reward for finishing it"
    );
    assert!(
        game.regional_fame.is_empty(),
        "and so was the territory it would have built"
    );

    // And the band is free again the very next turn.
    park_events(&mut game);
    game.process_turn(GameAction::WriteSongs)
        .expect("free to work again");
    assert!(!game.band.unreleased_songs.is_empty());
}

#[test]
fn abandoning_nothing_is_an_error() {
    let mut game = touring_band(23);
    assert!(
        game.process_turn(GameAction::AbandonActivity).is_err(),
        "there is nothing to walk out of"
    );
}

/// §F — Abandoning: "health/stress emergencies force it". A career that ends
/// mid-tour must not leave a commitment on the books for a reloaded save to
/// resume into.
#[test]
fn an_ending_forces_the_walk_out() {
    let mut game = touring_band(29);
    game.process_turn(GameAction::GoOnTour(TOUR_REGION, TOUR_RIG, TOUR_WEEKS))
        .expect("booking should succeed");
    assert!(game.ongoing_activity.is_some());

    game.player.health = 0;
    park_events(&mut game);
    game.process_turn(GameAction::LazeAround)
        .expect("the turn still resolves");

    assert!(game.is_game_over(), "health zero ends the career");
    assert!(
        game.ongoing_activity.is_none(),
        "and takes the tour off the books with it"
    );
}

// ---------------------------------------------------------------------------
// §F — saves
// ---------------------------------------------------------------------------

/// A save taken mid-tour resumes into the same tour and finishes it
/// identically — the activity carries its own progress, and its RNG is keyed
/// by booking week and night index rather than by a stream position that
/// would have to survive the trip.
#[test]
fn a_save_taken_mid_tour_resumes_and_finishes_the_same_way() {
    let uninterrupted = {
        let mut game = touring_band(31);
        game.process_turn(GameAction::GoOnTour(TOUR_REGION, TOUR_RIG, TOUR_WEEKS))
            .expect("booking should succeed");
        for _ in 1..TOUR_WEEKS {
            park_events(&mut game);
            game.process_turn(GameAction::LazeAround)
                .expect("a committed band's turn always resolves");
        }
        game
    };

    let mut game = touring_band(31);
    game.process_turn(GameAction::GoOnTour(TOUR_REGION, TOUR_RIG, TOUR_WEEKS))
        .expect("booking should succeed");
    park_events(&mut game);
    game.process_turn(GameAction::LazeAround)
        .expect("a second week on the road");

    let path = std::env::temp_dir().join("rocker-s-time-mid-tour.sav");
    let path = path.to_str().expect("a utf-8 temp path");
    game.save_game(path).expect("saving mid-tour should work");
    let mut resumed = Game::load_game(path).expect("and loading it back should too");
    let _ = std::fs::remove_file(path);

    let activity = resumed
        .ongoing_activity
        .as_ref()
        .expect("the save was taken on the road");
    assert_eq!(activity.weeks_done(), 2);
    assert_eq!(
        activity.headline(),
        format!("Tour bus of {}", game.get_sorted_regions()[TOUR_REGION].2)
    );

    suppress_story_events(&mut resumed);
    for _ in 2..TOUR_WEEKS {
        park_events(&mut resumed);
        resumed
            .process_turn(GameAction::LazeAround)
            .expect("a committed band's turn always resolves");
    }

    assert!(resumed.ongoing_activity.is_none());
    assert_eq!(resumed.week, uninterrupted.week);
    assert_eq!(resumed.player.money, uninterrupted.player.money);
    assert_eq!(resumed.band.fame, uninterrupted.band.fame);
    assert_eq!(
        nights(
            resumed
                .last_tour_report
                .as_ref()
                .expect("resumed tour reports")
        ),
        nights(
            uninterrupted
                .last_tour_report
                .as_ref()
                .expect("uninterrupted tour reports")
        ),
        "a save in the middle of a run cannot change a single night of it"
    );
}

#[test]
fn saves_from_before_the_studio_cycle_load_with_an_empty_calendar() {
    let path = concat!(env!("CARGO_MANIFEST_DIR"), "/tests/fixtures/pre-0.5.sav");
    let game = Game::load_game(path).expect("a v0.4.0 save must keep loading");
    assert!(
        game.ongoing_activity.is_none(),
        "nobody was mid-tour before §F existed"
    );
    assert!(!game.is_busy());
}

// ---------------------------------------------------------------------------
// §I — calendar integrity
// ---------------------------------------------------------------------------

/// **The assertion that makes §F true rather than aspirational.** Over a
/// long career that exercises every multi-week commitment there is, weeks
/// elapsed equals the number of `process_turn` calls that consumed a week —
/// and no single call ever consumed more than one.
#[test]
fn weeks_elapsed_equals_the_turns_that_consumed_a_week() {
    const HORIZON: u32 = 6 * constants::WEEKS_PER_YEAR;

    for seed in [2u64, 41] {
        let mut game = crate::game::sim::seeded_game(seed);
        suppress_story_events(&mut game);
        game.player.money = 5_000_000; // never let poverty end the run early
        let start_week = game.week;

        let mut week_consuming_turns: u32 = 0;
        let mut toured = 0u32;
        let mut broke_off = 0u32;
        let mut supported = 0u32;

        while !game.is_game_over() && game.week - start_week < HORIZON {
            let busy_before = game.ongoing_activity.is_some();
            let action = calendar_probe_action(&game);
            match &action {
                GameAction::GoOnTour(..) => toured += 1,
                GameAction::TakeBreak => broke_off += 1,
                GameAction::AcceptSupportTour => supported += 1,
                _ => {}
            }

            let week_before = game.week;
            if game.process_turn(action).is_err() {
                game.process_turn(GameAction::LazeAround)
                    .expect("lazing is always allowed");
            }
            game.take_turn_log();
            park_events(&mut game);

            let elapsed = game.week - week_before;
            assert!(
                elapsed <= 1,
                "no turn may consume more than one week — {} weeks at week {} (seed {seed}, \
                 busy: {busy_before})",
                elapsed,
                week_before
            );
            week_consuming_turns += elapsed;
        }

        assert_eq!(
            game.week - start_week,
            week_consuming_turns,
            "weeks elapsed must equal the turns that consumed a week (seed {seed})"
        );
        // A calendar nobody stress-tested proves nothing: the probe must
        // actually have booked the multi-week commitments.
        assert!(toured > 0, "the probe should have toured (seed {seed})");
        assert!(broke_off > 0, "the probe should have rested (seed {seed})");
        assert!(
            supported > 0,
            "the probe should have opened for somebody (seed {seed})"
        );
    }
}

/// A deliberately restless policy for the calendar probe: it takes every
/// multi-week commitment it is offered and mixes in the instant, no-week
/// actions that must *not* move the calendar. Correctness of play is beside
/// the point — coverage of the calendar's paths is not.
fn calendar_probe_action(game: &Game) -> GameAction {
    if game.pending_support_offer.is_some()
        && game.player.stress < TOUR_STRESS_GUARD
        && game.player.health >= TOUR_HEALTH_GUARD
    {
        return GameAction::AcceptSupportTour;
    }
    if game.player.stress >= 50 || game.player.health < 60 {
        return GameAction::TakeBreak;
    }
    // An instant action every so often: these must leave the week alone.
    if game.week.is_multiple_of(11) && !game.pending_deal_offers.is_empty() {
        return GameAction::RejectDeal(0);
    }
    if let Some(region) = (0..game.get_sorted_regions().len())
        .find(|&index| game.quote_tour(index, TourRig::Van, 2).is_ok())
        && game
            .quote_tour(region, TourRig::Van, 2)
            .is_ok_and(|quote| game.player.can_afford(quote.cost))
        && game.week.is_multiple_of(3)
    {
        return GameAction::GoOnTour(region, TourRig::Van, 2);
    }
    if game.band.unreleased_songs.len() < 3 {
        GameAction::WriteSongs
    } else {
        GameAction::Gig(best_open_venue(game))
    }
}
