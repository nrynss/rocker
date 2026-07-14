//! Data-driven random incidents (docs/DESIGN-v0.6-life-cycle.md §F): loader
//! validation, condition filtering, effect application/clamping, and seeded
//! triggering. Content lives in `data/incidents.json`; none is hardcoded.

use rand::SeedableRng;
use rand::rngs::StdRng;

use crate::data_loader::{Incident, IncidentConditions, IncidentEffects, IncidentsData};

use super::*;

/// A minimal test incident (weight 1) with the given conditions and effects.
fn incident(id: &str, conditions: IncidentConditions, effects: IncidentEffects) -> Incident {
    Incident {
        id: id.to_string(),
        category: "test".to_string(),
        weight: 1,
        conditions,
        effects,
        message: format!("🧪 test:{id}"),
    }
}

#[test]
fn validation_rejects_malformed_pools() {
    let parse = |json: &str| -> IncidentsData { serde_json::from_str(json).expect("parses") };

    // Empty pool.
    assert!(
        parse(r#"{"incidents":[]}"#).validate().is_err(),
        "an empty incident list must be rejected"
    );

    // Weight below 1.
    let w0 = r#"{"incidents":[{"id":"a","category":"c","weight":0,"effects":{},"message":"m"}]}"#;
    assert!(parse(w0).validate().is_err(), "weight 0 must be rejected");

    // Duplicate ids.
    let dup = r#"{"incidents":[
        {"id":"a","category":"c","weight":1,"effects":{},"message":"m"},
        {"id":"a","category":"c","weight":1,"effects":{},"message":"m"}
    ]}"#;
    assert!(
        parse(dup).validate().is_err(),
        "duplicate ids must be rejected"
    );

    // An effect range with lo > hi.
    let bad = r#"{"incidents":[{"id":"a","category":"c","weight":1,"effects":{"money":[10,-5]},"message":"m"}]}"#;
    assert!(
        parse(bad).validate().is_err(),
        "an inverted effect range must be rejected"
    );

    // A well-formed minimal pool passes.
    let ok = r#"{"incidents":[{"id":"a","category":"c","weight":1,"effects":{"stress":[-5,5]},"message":"m"}]}"#;
    assert!(parse(ok).validate().is_ok(), "a valid pool must pass");
}

#[test]
fn the_shipped_incident_pool_loads_and_validates() {
    let game = test_game();
    let data = &game.data_files.incidents_data;
    assert!(
        data.validate().is_ok(),
        "the shipped data/incidents.json must validate"
    );
    assert!(
        data.incidents.len() >= 25,
        "expected a full pool (~25–30), got {}",
        data.incidents.len()
    );
    // The drug/addiction system is deferred this cycle: nothing drug-related
    // should have migrated into the pool.
    assert!(
        !data.incidents.iter().any(|i| i.id.contains("drug")),
        "DrugOffer was dropped; no drug incident should ship"
    );
}

#[test]
fn eligible_incidents_filter_on_fame_and_signed() {
    let data = IncidentsData {
        incidents: vec![
            incident(
                "any",
                IncidentConditions::default(),
                IncidentEffects::default(),
            ),
            incident(
                "nobody",
                IncidentConditions {
                    max_fame: Some(20),
                    ..Default::default()
                },
                IncidentEffects::default(),
            ),
            incident(
                "star",
                IncidentConditions {
                    min_fame: Some(60),
                    ..Default::default()
                },
                IncidentEffects::default(),
            ),
            incident(
                "signed_only",
                IncidentConditions {
                    signed: Some(true),
                    ..Default::default()
                },
                IncidentEffects::default(),
            ),
            incident(
                "unsigned_only",
                IncidentConditions {
                    signed: Some(false),
                    ..Default::default()
                },
                IncidentEffects::default(),
            ),
        ],
    };
    let ids = |fame: u8, signed: bool| -> Vec<String> {
        data.eligible_incidents(fame, signed)
            .iter()
            .map(|i| i.id.clone())
            .collect()
    };

    // Unsigned nobody at fame 10.
    let low = ids(10, false);
    assert!(low.contains(&"any".into()));
    assert!(low.contains(&"nobody".into()));
    assert!(!low.contains(&"star".into()));
    assert!(!low.contains(&"signed_only".into()));
    assert!(low.contains(&"unsigned_only".into()));

    // Signed star at fame 80.
    let high = ids(80, true);
    assert!(high.contains(&"any".into()));
    assert!(!high.contains(&"nobody".into()));
    assert!(high.contains(&"star".into()));
    assert!(high.contains(&"signed_only".into()));
    assert!(!high.contains(&"unsigned_only".into()));

    // Fame bounds are inclusive at both ends.
    assert!(
        ids(20, false).contains(&"nobody".into()),
        "max_fame inclusive"
    );
    assert!(!ids(21, false).contains(&"nobody".into()));
    assert!(
        ids(60, false).contains(&"star".into()),
        "min_fame inclusive"
    );
    assert!(!ids(59, false).contains(&"star".into()));
}

#[test]
fn effects_roll_within_range_and_clamp_to_bar_bounds() {
    // Every roll of a declared range lands inside it.
    let inc = incident(
        "h",
        IncidentConditions::default(),
        IncidentEffects {
            happiness: Some([5, 15]),
            ..Default::default()
        },
    );
    for seed in 0..64u64 {
        let mut game = test_game();
        game.player.happiness = 50;
        game.apply_incident(&inc, &mut StdRng::seed_from_u64(seed));
        let delta = i32::from(game.player.happiness) - 50;
        assert!(
            (5..=15).contains(&delta),
            "happiness delta {delta} outside declared [5, 15]"
        );
    }

    // Clamp high: a bar can't exceed 100.
    let mut game = test_game();
    game.player.happiness = 95;
    let up = incident(
        "up",
        IncidentConditions::default(),
        IncidentEffects {
            happiness: Some([50, 50]),
            ..Default::default()
        },
    );
    game.apply_incident(&up, &mut StdRng::seed_from_u64(1));
    assert_eq!(game.player.happiness, 100, "happiness clamps at 100");

    // Clamp low: a bar can't drop below 0.
    let mut game = test_game();
    game.player.stress = 10;
    let down = incident(
        "down",
        IncidentConditions::default(),
        IncidentEffects {
            stress: Some([-50, -50]),
            ..Default::default()
        },
    );
    game.apply_incident(&down, &mut StdRng::seed_from_u64(1));
    assert_eq!(game.player.stress, 0, "stress floors at 0");

    // Money is not a bar and may go negative (a cost).
    let mut game = test_game();
    game.player.money = 100;
    let bill = incident(
        "bill",
        IncidentConditions::default(),
        IncidentEffects {
            money: Some([-500, -500]),
            ..Default::default()
        },
    );
    game.apply_incident(&bill, &mut StdRng::seed_from_u64(1));
    assert_eq!(game.player.money, -400, "money can go negative");
}

#[test]
fn fame_gains_route_through_gain_fame_and_losses_saturate() {
    // At peak, a fame gain is applied verbatim (no comeback doubling).
    let mut game = test_game();
    game.band.fame = 20;
    game.band.peak_fame = 20;
    let gain = incident(
        "g",
        IncidentConditions::default(),
        IncidentEffects {
            fame: Some([2, 2]),
            ..Default::default()
        },
    );
    game.apply_incident(&gain, &mut StdRng::seed_from_u64(1));
    assert_eq!(game.band.fame, 22, "a fame gain at peak lands verbatim");

    // A loss saturates and never routes through the comeback ×2 path, even
    // while below peak (where a *gain* would double).
    let mut game = test_game();
    game.band.fame = 10;
    game.band.peak_fame = 90;
    let loss = incident(
        "l",
        IncidentConditions::default(),
        IncidentEffects {
            fame: Some([-30, -30]),
            ..Default::default()
        },
    );
    game.apply_incident(&loss, &mut StdRng::seed_from_u64(1));
    assert_eq!(game.band.fame, 0, "a fame loss saturates at 0");
}

#[test]
fn a_seeded_run_triggers_incidents_deterministically() {
    // Keep the act alive (money/health topped up between turns) so a full
    // 40-week idle stretch runs; field writes don't touch the action stream,
    // so two identical runs must replay line-for-line.
    fn run(seed: u64) -> (Vec<String>, u8, i32) {
        let mut game = crate::game::sim::seeded_game(seed);
        let mut log = Vec::new();
        for _ in 0..40 {
            game.player.money = game.player.money.max(10_000);
            game.player.health = 100;
            let _ = game.process_turn(GameAction::LazeAround);
            log.append(&mut game.take_turn_log());
        }
        (log, game.band.fame, game.player.money)
    }

    let (log_a, fame_a, money_a) = run(2025);
    let (log_b, fame_b, money_b) = run(2025);
    assert_eq!(log_a, log_b, "same seed replays the same incident stream");
    assert_eq!(fame_a, fame_b);
    assert_eq!(money_a, money_b);

    // Over 40 weeks at a 35% weekly chance, an incident is all but certain to
    // fire — prove it by matching a shipped incident message in the log.
    let messages: Vec<String> = {
        let game = test_game();
        game.data_files
            .incidents_data
            .incidents
            .iter()
            .map(|i| i.message.clone())
            .collect()
    };
    let fired = log_a
        .iter()
        .any(|line| messages.iter().any(|m| line.contains(m.as_str())));
    assert!(
        fired,
        "a 40-week idle run should surface at least one incident"
    );
}
