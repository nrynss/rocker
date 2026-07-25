//! S-EVENTS (design v0.9 §G): things go wrong while you work.
//!
//! Before §F a four-week tour resolved inside a single action, so nothing
//! could ever happen *during* it. Now that commitments resolve one week at a
//! time, the week has a second incident tap: the **activity tap**, drawing
//! from an activity-gated slice of `data/incidents.json`.
//!
//! Four properties carry this file. **Gating**: a van cannot break down in
//! the studio, and a producer cannot clash on the road. **Cadence**: the tap
//! is modest (~12%/week) and shares `last_event_week` with the general tap,
//! so a tour is a place things can happen, not a slot machine. **Effects**:
//! an activity incident moves the same bars every other incident moves, via
//! the same machinery. And **determinism**: the whole thing rolls on the
//! action stream, so a seed replays its road stories exactly — and the
//! activity stream, which owns the tour's nights, is left strictly alone.

use rand::SeedableRng;
use rand::rngs::StdRng;

use crate::data_loader::{ActivityScope, IncidentsData};
use crate::game::actions::TourRig;

use super::*;

// ---------------------------------------------------------------------------
// Fixtures
// ---------------------------------------------------------------------------

/// A Van run of the shortest bookable length: no rig gate, no length gate, so
/// the driver below can keep re-booking regardless of what fame does to it.
const PROBE_RIG: TourRig = TourRig::Van;
const PROBE_WEEKS: u8 = 2;

/// The cheapest region the band can actually tour right now. Region 0 is not
/// a safe constant here: `get_sorted_regions` is fame-ordered, and a general
/// incident that costs a point of fame can put the front of the list out of
/// reach for the rest of the career — which is exactly the sort of thing this
/// file is here to let happen.
fn probe_region(game: &Game) -> Option<usize> {
    (0..game.get_sorted_regions().len())
        .find(|&index| game.quote_tour(index, PROBE_RIG, PROBE_WEEKS).is_ok())
}

/// A band that can afford to stay on the road indefinitely. Money, health and
/// stress are topped up every turn by the drivers so that the *only* thing
/// varying across the run is what the event taps do.
fn road_band(seed: u64) -> Game {
    let mut game = crate::game::sim::seeded_game(seed);
    game.band.fame = 35;
    game.band.peak_fame = 35;
    game.band.skill = 50;
    game.player.money = 500_000;
    game
}

fn keep_alive(game: &mut Game) {
    game.player.money = game.player.money.max(500_000);
    game.player.health = 100;
    game.player.stress = 0;
}

/// Every shipped message claimed by `scope`'s activity tap.
fn messages_for(game: &Game, scope: ActivityScope) -> Vec<String> {
    game.data_files
        .incidents_data
        .incidents
        .iter()
        .filter(|inc| inc.in_activity(scope))
        .map(|inc| inc.message.clone())
        .collect()
}

/// Every shipped message on the general (activity-free) tap.
fn general_messages(game: &Game) -> Vec<String> {
    game.data_files
        .incidents_data
        .incidents
        .iter()
        .filter(|inc| inc.is_general())
        .map(|inc| inc.message.clone())
        .collect()
}

fn any_line_matches(log: &[String], messages: &[String]) -> bool {
    log.iter()
        .any(|line| messages.iter().any(|m| line.contains(m.as_str())))
}

fn count_lines_matching(log: &[String], messages: &[String]) -> u32 {
    log.iter()
        .filter(|line| messages.iter().any(|m| line.contains(m.as_str())))
        .count() as u32
}

/// What a scripted career produced: the weeks it actually spent inside the
/// activity under test, and every log line it wrote.
struct Run {
    activity_weeks: u32,
    log: Vec<String>,
}

/// Book back-to-back tours and play every week of them out. Returns the count
/// of weeks genuinely spent on the road — booking weeks included, since §F
/// resolves the first week at booking — and the whole log.
fn tour_career(seed: u64, turns: u32, park_the_clock: bool) -> Run {
    let mut game = road_band(seed);
    let mut run = Run {
        activity_weeks: 0,
        log: Vec::new(),
    };
    for _ in 0..turns {
        keep_alive(&mut game);
        if park_the_clock {
            game.events.last_event_week = u32::MAX;
        }
        let busy = game.ongoing_activity.is_some();
        let action = match (busy, probe_region(&game)) {
            (true, _) => GameAction::LazeAround,
            (false, Some(region)) => GameAction::GoOnTour(region, PROBE_RIG, PROBE_WEEKS),
            (false, None) => GameAction::LazeAround,
        };
        let week_before = game.week;
        if game.process_turn(action).is_err() {
            game.process_turn(GameAction::LazeAround)
                .expect("lazing is always allowed");
        } else if game.week > week_before && (busy || game.ongoing_activity.is_some()) {
            // A week that resolved with the band already committed, or the
            // booking week whose first week §F plays immediately.
            run.activity_weeks += 1;
        }
        run.log.append(&mut game.take_turn_log());
        if game.is_game_over() {
            break;
        }
    }
    run
}

/// Play a gig every single week.
fn gig_career(seed: u64, turns: u32) -> Run {
    let mut game = road_band(seed);
    let mut run = Run {
        activity_weeks: 0,
        log: Vec::new(),
    };
    for _ in 0..turns {
        keep_alive(&mut game);
        let week_before = game.week;
        if game
            .process_turn(GameAction::Gig(best_open_venue(&game)))
            .is_ok()
            && game.week > week_before
        {
            run.activity_weeks += 1;
        }
        run.log.append(&mut game.take_turn_log());
        if game.is_game_over() {
            break;
        }
    }
    run
}

// ---------------------------------------------------------------------------
// §G — the schema and the two pools
// ---------------------------------------------------------------------------

/// Every activity the §G table names ships content, and `Break` deliberately
/// ships none: downtime already has the general pool, and an empty activity
/// pool is a supported answer that costs no roll (`events_apply.rs`).
#[test]
fn every_activity_the_design_names_ships_a_pool() {
    let game = test_game();
    let data = &game.data_files.incidents_data;
    for scope in [
        ActivityScope::Recording,
        ActivityScope::Gig,
        ActivityScope::Tour,
        ActivityScope::SupportTour,
    ] {
        assert!(
            !data
                .eligible_activity_incidents(35, false, scope)
                .is_empty(),
            "§G names {scope:?} but nothing in incidents.json claims it"
        );
    }
    assert!(
        data.eligible_activity_incidents(35, false, ActivityScope::Break)
            .is_empty(),
        "a break is downtime — the general pool covers it"
    );
}

/// The two pools are disjoint by construction: an incident is either general
/// (no `activity`) or activity-scoped, never both. This is what keeps a free
/// week's draw identical to the pre-§G one.
#[test]
fn the_general_tap_and_the_activity_taps_draw_from_disjoint_pools() {
    let game = test_game();
    let data = &game.data_files.incidents_data;

    for inc in &data.incidents {
        assert_eq!(
            inc.is_general(),
            inc.activity.is_empty(),
            "'{}' must be general exactly when it claims no activity",
            inc.id
        );
    }

    // Nothing scoped can reach the general tap, at any fame, signed or not.
    for fame in [0u8, 20, 35, 60, 90] {
        for signed in [false, true] {
            for inc in data.eligible_incidents(fame, signed) {
                assert!(
                    inc.activity.is_empty(),
                    "'{}' is activity-scoped but reached the general tap",
                    inc.id
                );
            }
            for scope in ActivityScope::ALL {
                for inc in data.eligible_activity_incidents(fame, signed, scope) {
                    assert!(
                        inc.in_activity(scope),
                        "'{}' reached the {scope:?} tap without claiming it",
                        inc.id
                    );
                }
            }
        }
    }
}

/// The gate, stated at the level the design states it: a van breakdown cannot
/// fire in the studio, and a producer clash cannot fire on the road.
#[test]
fn a_studio_incident_is_never_eligible_on_the_road_and_vice_versa() {
    let game = test_game();
    let data = &game.data_files.incidents_data;
    let ids = |scope| -> Vec<&str> {
        data.eligible_activity_incidents(100, true, scope)
            .iter()
            .map(|inc| inc.id.as_str())
            .collect()
    };
    let studio = ids(ActivityScope::Recording);
    let road = ids(ActivityScope::Tour);
    let stage = ids(ActivityScope::Gig);

    assert!(studio.contains(&"studio_producer_clash"));
    assert!(road.contains(&"tour_van_breakdown"));
    for id in &studio {
        assert!(!road.contains(id), "'{id}' is eligible both places");
        assert!(!stage.contains(id), "'{id}' is eligible both places");
    }
    for id in &road {
        assert!(!studio.contains(id), "'{id}' is eligible both places");
    }
}

/// The loader contract §G inherits: an entry that predates the field — i.e.
/// every entry the file shipped before this task — still loads, and lands in
/// the general pool.
#[test]
fn incidents_without_an_activity_field_still_load_as_general() {
    let json = r#"{"incidents":[
        {"id":"legacy","category":"c","weight":2,"effects":{"stress":[1,2]},"message":"m"},
        {"id":"scoped","category":"c","weight":2,"activity":["tour"],"effects":{},"message":"n"}
    ]}"#;
    let data: IncidentsData = serde_json::from_str(json).expect("a pre-§G entry must still parse");
    assert!(data.validate().is_ok());
    assert!(data.incidents[0].is_general());
    assert!(!data.incidents[1].is_general());
    assert!(data.incidents[1].in_activity(ActivityScope::Tour));
    assert!(!data.incidents[1].in_activity(ActivityScope::Gig));

    let general: Vec<&str> = data
        .eligible_incidents(50, false)
        .iter()
        .map(|i| i.id.as_str())
        .collect();
    assert_eq!(general, vec!["legacy"]);
}

/// A scope listed twice is always a copy-paste slip; the loader says so
/// rather than shipping it.
#[test]
fn validation_rejects_a_duplicated_activity_scope() {
    let json = r#"{"incidents":[
        {"id":"a","category":"c","weight":1,"activity":["tour","tour"],"effects":{},"message":"m"}
    ]}"#;
    let data: IncidentsData = serde_json::from_str(json).expect("parses");
    assert!(
        data.validate().is_err(),
        "a repeated scope must be rejected"
    );
}

// ---------------------------------------------------------------------------
// §G — gating, played out
// ---------------------------------------------------------------------------

/// The gate as the player meets it: a band that spends its whole career on
/// the road hears road stories and the ordinary weekly ones, and never a note
/// from a studio it isn't in or a stage it isn't on.
#[test]
fn a_touring_band_never_draws_a_studio_or_a_stage_incident() {
    let game = test_game();
    let studio = messages_for(&game, ActivityScope::Recording);
    let stage = messages_for(&game, ActivityScope::Gig);
    let support = messages_for(&game, ActivityScope::SupportTour);
    let road = messages_for(&game, ActivityScope::Tour);

    let mut saw_road = false;
    for seed in [3u64, 19, 101] {
        let run = tour_career(seed, 160, false);
        assert!(
            run.activity_weeks > 100,
            "the probe should have spent its career touring (seed {seed}, {} weeks)",
            run.activity_weeks
        );
        assert!(
            !any_line_matches(&run.log, &studio),
            "a studio incident fired on the road (seed {seed})"
        );
        assert!(
            !any_line_matches(&run.log, &stage),
            "a one-off gig's incident fired during a tour (seed {seed})"
        );
        // `tour_van_breakdown` and `tour_road_exhaustion` claim both road
        // scopes, so only the support-*only* entries prove the gate here.
        let support_only: Vec<String> = support
            .iter()
            .filter(|m| !road.contains(m))
            .cloned()
            .collect();
        assert!(
            !any_line_matches(&run.log, &support_only),
            "a support-slot incident fired on a headline tour (seed {seed})"
        );
        saw_road |= any_line_matches(&run.log, &road);
    }
    assert!(
        saw_road,
        "three touring careers should have produced at least one road incident"
    );
}

/// And the mirror: a band that only ever plays one-off gigs draws stage
/// incidents, never road or studio ones.
#[test]
fn a_gigging_band_never_draws_a_road_or_studio_incident() {
    let game = test_game();
    let studio = messages_for(&game, ActivityScope::Recording);
    let stage = messages_for(&game, ActivityScope::Gig);
    let road = messages_for(&game, ActivityScope::Tour);
    let support = messages_for(&game, ActivityScope::SupportTour);

    let mut saw_stage = false;
    for seed in [5u64, 23, 77] {
        let run = gig_career(seed, 160);
        assert!(
            run.activity_weeks > 100,
            "the probe should have spent its career gigging (seed {seed}, {} weeks)",
            run.activity_weeks
        );
        assert!(
            !any_line_matches(&run.log, &studio),
            "a studio incident fired at a gig (seed {seed})"
        );
        assert!(
            !any_line_matches(&run.log, &road),
            "a tour incident fired at a one-off gig (seed {seed})"
        );
        assert!(
            !any_line_matches(&run.log, &support),
            "a support-slot incident fired at a one-off gig (seed {seed})"
        );
        saw_stage |= any_line_matches(&run.log, &stage);
    }
    assert!(
        saw_stage,
        "three gigging careers should have produced at least one stage incident"
    );
}

// ---------------------------------------------------------------------------
// §G — cadence and the shared cooldown
// ---------------------------------------------------------------------------

/// "[tune: ~12%/week]". Measured over the weeks that are actually in scope,
/// across three seeds, with a band wide enough to be a property rather than a
/// re-statement of one sample.
#[test]
fn the_activity_tap_fires_at_roughly_the_designed_weekly_chance() {
    let game = test_game();
    let road = messages_for(&game, ActivityScope::Tour);
    let support = messages_for(&game, ActivityScope::SupportTour);
    let scoped: Vec<String> = road.into_iter().chain(support).collect();

    let mut weeks = 0u32;
    let mut fired = 0u32;
    for seed in [2u64, 13, 61] {
        let run = tour_career(seed, 300, false);
        weeks += run.activity_weeks;
        fired += count_lines_matching(&run.log, &scoped);
    }
    assert!(weeks > 500, "not enough sample: {weeks} road weeks");
    let rate = fired * 100 / weeks;
    println!(
        "§G activity tap fired in {fired}/{weeks} road weeks ({rate}%); design target {}%",
        crate::game::events::ACTIVITY_INCIDENT_WEEKLY_CHANCE_PERCENT
    );
    assert!(
        (7..=18).contains(&rate),
        "activity-event cadence {rate}% is far from the designed {}%",
        crate::game::events::ACTIVITY_INCIDENT_WEEKLY_CHANCE_PERCENT
    );
}

/// The two taps share `last_event_week`, so a week fires at most one
/// incident between them — a tour is a place things can happen, not a slot
/// machine. Stated where it lives: once a week is stamped, *neither* gate
/// opens again inside it, whatever the roll would have been.
#[test]
fn both_taps_share_the_weekly_cooldown() {
    for seed in 0..64u64 {
        let mut events = crate::game::events::EventManager::new();
        let mut rng = StdRng::seed_from_u64(seed);
        events.last_event_week = 10;
        assert!(
            !events.try_trigger_event(10, &mut rng),
            "the general tap must not fire twice in one week"
        );
        assert!(
            !events.try_trigger_activity_event(10, &mut rng),
            "the activity tap must not fire in a week already spent"
        );
        assert_eq!(events.last_event_week, 10, "and neither may re-stamp it");
    }
}

/// The sim lab's and `time.rs`'s idiom for silencing incidents —
/// `last_event_week = u32::MAX` — must silence the *activity* tap too, or
/// every money-exact test in the cycle would start drifting.
#[test]
fn parking_the_incident_clock_silences_the_activity_tap() {
    let game = test_game();
    let mut scoped: Vec<String> = Vec::new();
    for scope in ActivityScope::ALL {
        scoped.extend(messages_for(&game, scope));
    }
    let general = general_messages(&game);

    for seed in [7u64, 31] {
        let run = tour_career(seed, 120, true);
        assert!(run.activity_weeks > 80, "the probe should have toured");
        assert!(
            !any_line_matches(&run.log, &scoped),
            "a parked clock must silence the activity tap (seed {seed})"
        );
        assert!(
            !any_line_matches(&run.log, &general),
            "a parked clock must silence the general tap too (seed {seed})"
        );
    }
}

/// An empty activity pool must not *consume* the week's incident. A break
/// claims nothing, so a band on a break keeps drawing the ordinary weekly
/// pool exactly as it did before §G.
#[test]
fn an_activity_with_no_pool_leaves_the_general_tap_alone() {
    let game = test_game();
    let general = general_messages(&game);

    let mut saw_general_on_a_break = false;
    for seed in [4u64, 44, 444] {
        let mut game = road_band(seed);
        let mut log = Vec::new();
        for _ in 0..200 {
            keep_alive(&mut game);
            let action = if game.ongoing_activity.is_some() {
                GameAction::LazeAround
            } else {
                GameAction::TakeBreak
            };
            let on_a_break = game.ongoing_activity.is_some();
            let _ = game.process_turn(action);
            let mut turn = game.take_turn_log();
            if on_a_break {
                saw_general_on_a_break |= any_line_matches(&turn, &general);
            }
            log.append(&mut turn);
        }
    }
    assert!(
        saw_general_on_a_break,
        "a break has no pool of its own, so the general tap must keep firing through it"
    );
}

// ---------------------------------------------------------------------------
// §G — effects and determinism
// ---------------------------------------------------------------------------

/// Activity incidents reuse the incident-effect machinery wholesale — the
/// same six channels, the same clamps, the same fixed roll order. Nothing
/// about §G is a parallel system.
#[test]
fn an_activity_incident_applies_through_the_ordinary_effect_machinery() {
    let mut game = test_game();
    let breakdown = game
        .data_files
        .incidents_data
        .incidents
        .iter()
        .find(|inc| inc.id == "tour_van_breakdown")
        .expect("the shipped road pool has a van breakdown")
        .clone();

    game.player.money = 10_000;
    game.player.stress = 20;
    game.player.happiness = 60;
    game.apply_incident(&breakdown, &mut StdRng::seed_from_u64(9));

    assert!(game.player.money < 10_000, "a tow truck costs money");
    assert!(
        game.player.stress > 20,
        "and a night nobody slept costs more"
    );
    assert!(game.player.happiness < 60);
    assert!(
        game.turn_log.iter().any(|l| l.contains(&breakdown.message)),
        "and the incident's own line hits the log"
    );
}

/// Clamping holds for the scoped pool exactly as for the general one: a
/// bar-moving road incident can never push a bar outside 0–100.
#[test]
fn activity_incident_effects_stay_inside_their_declared_ranges() {
    let game = test_game();
    let scoped: Vec<_> = game
        .data_files
        .incidents_data
        .incidents
        .iter()
        .filter(|inc| !inc.is_general())
        .cloned()
        .collect();
    assert!(!scoped.is_empty());

    for inc in &scoped {
        for seed in 0..8u64 {
            let mut probe = test_game();
            probe.player.health = 50;
            probe.player.stress = 50;
            probe.player.happiness = 50;
            probe.player.creativity = 50;
            probe.apply_incident(inc, &mut StdRng::seed_from_u64(seed));
            for (name, bar) in [
                ("health", probe.player.health),
                ("stress", probe.player.stress),
                ("happiness", probe.player.happiness),
                ("creativity", probe.player.creativity),
            ] {
                assert!(bar <= 100, "'{}' pushed {name} to {bar}", inc.id);
            }
        }
    }
}

/// **The determinism assertion for this task.** The activity tap rolls on the
/// action stream — gate, weighted pick and every effect range — and never on
/// the activity stream, whose steps belong to the tour's nights. So a seed
/// replays its road stories line for line, and the money and fame they moved
/// land in the same places.
#[test]
fn a_seeded_touring_career_replays_its_activity_events_exactly() {
    for seed in [8u64, 2025] {
        let first = tour_career(seed, 120, false);
        let second = tour_career(seed, 120, false);
        assert_eq!(
            first.log, second.log,
            "same seed, same choices, same road (seed {seed})"
        );
        assert_eq!(first.activity_weeks, second.activity_weeks);
    }
}

/// And the split's own guarantee still holds *underneath* §G: the nights of a
/// tour are the activity stream's, so adding a weekly event tap on the action
/// stream cannot move a single show's take or reception.
#[test]
fn the_activity_tap_does_not_disturb_a_tours_nights() {
    let mut with_events = road_band(37);
    let region = probe_region(&with_events).expect("the fixture band can tour somewhere");
    with_events
        .process_turn(GameAction::GoOnTour(region, PROBE_RIG, PROBE_WEEKS))
        .expect("the fixture band can afford this tour");
    with_events
        .process_turn(GameAction::LazeAround)
        .expect("a committed band's turn always resolves");

    let mut silenced = road_band(37);
    silenced.events.last_event_week = u32::MAX;
    silenced
        .process_turn(GameAction::GoOnTour(region, PROBE_RIG, PROBE_WEEKS))
        .expect("the fixture band can afford this tour");
    silenced.events.last_event_week = u32::MAX;
    silenced
        .process_turn(GameAction::LazeAround)
        .expect("a committed band's turn always resolves");

    let takes = |game: &Game| -> Vec<(u32, u8)> {
        game.last_tour_report
            .as_ref()
            .expect("a tour always leaves a report")
            .rows
            .iter()
            .map(|row| (row.take, row.reception))
            .collect()
    };
    assert_eq!(
        takes(&with_events),
        takes(&silenced),
        "every night of the tour rolls on the activity stream — the event tap cannot reach it"
    );
}
