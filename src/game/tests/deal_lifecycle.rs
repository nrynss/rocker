//! Deal lifecycle (design §E-4, §E-5): contract term, breach, the
//! recoupment-dependent renewal window, label memos, and recoup pressure.

use rand::SeedableRng;
use rand::rngs::StdRng;

use crate::game::music::ReleaseType;
use crate::game::world::PotentialDealOffer;

use super::*;

/// A signed deal referencing a *real* label from the data files (needed for
/// the renewal window, which looks the label up by name/tier), rather than
/// `test_deal`'s placeholder "Test Records".
fn signed_deal_with_real_label(game: &Game, signed_week: u32, term_weeks: u16) -> band::RecordDeal {
    let label = game.data_files.get_record_labels_data().independent_labels[0].clone();
    band::RecordDeal {
        label_name: label.name,
        label_tier: "Independent".to_string(),
        advance: label.advance_range[0].max(1_000),
        royalty_rate: label.royalty_rate as f32 / 100.0,
        albums_required: 1,
        albums_delivered: 0,
        market_reach: label.market_reach,
        unrecouped: 0,
        signed_week,
        term_weeks,
    }
}

// ---------------------------------------------------------------------------
// §E-4: term stamped at signing, by tier.
// ---------------------------------------------------------------------------

#[test]
fn term_generated_within_tier_range_at_signing() {
    let mut game = test_game();
    game.band.fame = 90;
    game.band.albums_released = (0..5)
        .map(|i| test_release(i, ReleaseType::Album))
        .collect();
    game.band.singles_released = (0..5)
        .map(|i| test_release(100 + i, ReleaseType::Single))
        .collect();

    let mut seen: std::collections::HashMap<String, (u16, u16)> = std::collections::HashMap::new();
    let mut rng = StdRng::seed_from_u64(1);
    for _ in 0..300 {
        for offer in game
            .world
            .generate_deal_offers(&game.band, &game.data_files, &mut rng)
        {
            let entry = seen
                .entry(offer.label_tier.clone())
                .or_insert((u16::MAX, 0));
            entry.0 = entry.0.min(offer.term_weeks);
            entry.1 = entry.1.max(offer.term_weeks);
        }
    }
    assert!(
        !seen.is_empty(),
        "a fame-90 act with a deep catalog should draw offers across tiers"
    );
    for (tier, (min_seen, max_seen)) in seen {
        let (floor, ceiling) = match tier.as_str() {
            "Boutique" => (52, 78),
            "Independent" => (78, 104),
            "Major" => (104, 156),
            other => panic!("unexpected tier {other}"),
        };
        assert!(
            min_seen >= floor && max_seen <= ceiling,
            "{tier}: observed range [{min_seen},{max_seen}] outside design [{floor},{ceiling}]"
        );
    }
}

#[test]
fn signing_stamps_signed_week_and_the_offered_term() {
    let mut game = test_game();
    game.week = 42;
    let mut offer = test_deal_offer(&game, None);
    offer.term_weeks = 90;
    game.pending_deal_offers.push(offer);
    game.action_accept_deal(0).expect("signing succeeds");

    let deal = game.band.current_deal().expect("signed");
    assert_eq!(deal.signed_week, 42);
    assert_eq!(deal.term_weeks, 90);
}

// ---------------------------------------------------------------------------
// §E-4: free agency at the LATER of albums delivered and term served.
// ---------------------------------------------------------------------------

#[test]
fn early_album_delivery_keeps_the_band_signed_until_the_term_is_served() {
    let mut game = test_game();
    let mut deal = signed_deal_with_real_label(&game, 10, 100); // term ends week 110
    deal.albums_required = 1;
    game.band.record_deal = Some(deal);

    // Week 50: albums met, term nowhere near served — stays signed.
    let outcome = game.band.fulfill_album_obligation(50);
    match outcome {
        band::DealCompletionOutcome::ObligationDelivered {
            term_end_week,
            label_name,
        } => {
            assert_eq!(term_end_week, 110);
            assert!(!label_name.is_empty());
        }
        other => panic!("expected ObligationDelivered, got {other:?}"),
    }
    assert!(
        game.band.current_deal().is_some(),
        "delivering early must not clear the deal"
    );

    // A further album delivered while the term runs on doesn't re-announce.
    let outcome2 = game.band.fulfill_album_obligation(60);
    assert_eq!(outcome2, band::DealCompletionOutcome::StillActive);
    assert!(game.band.current_deal().is_some());

    // Nothing is owed anymore, so there's no reason to expect another
    // release — the term running out on the calendar alone must free the
    // band (the weekly clock check, not `fulfill_album_obligation`).
    assert!(
        game.band.check_term_served_free_agency(109).is_none(),
        "one week early, still under contract"
    );
    let label_name = game
        .band
        .check_term_served_free_agency(110)
        .expect("the term is served at week 110");
    assert!(!label_name.is_empty());
    assert!(game.band.current_deal().is_none());
}

#[test]
fn term_served_before_albums_delivered_waits_for_the_albums() {
    let mut game = test_game();
    let mut deal = signed_deal_with_real_label(&game, 0, 20); // term ends week 20
    deal.albums_required = 2;
    game.band.record_deal = Some(deal);

    // Week 100: term long served, but only one of two albums delivered.
    let outcome = game.band.fulfill_album_obligation(100);
    assert_eq!(outcome, band::DealCompletionOutcome::StillActive);
    assert!(
        game.band.current_deal().is_some(),
        "albums still owed keeps the band signed even past term expiry"
    );

    // The second album finally clears it — free agency, not a breach.
    let outcome2 = game.band.fulfill_album_obligation(100);
    assert!(matches!(
        outcome2,
        band::DealCompletionOutcome::FreeAgent { .. }
    ));
}

// ---------------------------------------------------------------------------
// §E-4: breach.
// ---------------------------------------------------------------------------

#[test]
fn breach_on_term_expiry_with_albums_still_owed() {
    let mut game = test_game();
    let mut deal = signed_deal_with_real_label(&game, 0, 52); // ends week 52
    deal.albums_required = 3;
    deal.albums_delivered = 1;
    deal.unrecouped = 4_000;
    game.band.record_deal = Some(deal);
    let rep_before = game.band.reputation.commercial_success;

    let breach = game
        .band
        .check_term_breach(52)
        .expect("term expired with albums owed must breach");
    assert_eq!(
        breach.written_off, 4_000,
        "the remaining ledger is written off"
    );
    assert!(
        game.band.current_deal().is_none(),
        "the deal ends on breach"
    );
    assert_eq!(game.band.deal_cooldown, DEAL_BREACH_COOLDOWN_WEEKS);
    assert_eq!(
        game.band.reputation.commercial_success,
        rep_before.saturating_sub(DEAL_BREACH_REPUTATION_HIT)
    );
}

#[test]
fn no_breach_while_the_term_still_has_time_or_nothing_is_owed() {
    let mut game = test_game();
    // Term not yet up.
    let mut deal = signed_deal_with_real_label(&game, 0, 52);
    deal.albums_required = 3;
    deal.albums_delivered = 1;
    game.band.record_deal = Some(deal);
    assert!(game.band.check_term_breach(51).is_none());
    assert!(game.band.current_deal().is_some());

    // Term up, but nothing owed — that's free agency's job, not breach's.
    let mut deal2 = signed_deal_with_real_label(&game, 0, 52);
    deal2.albums_required = 1;
    deal2.albums_delivered = 1;
    game.band.record_deal = Some(deal2);
    assert!(game.band.check_term_breach(52).is_none());
}

#[test]
fn legacy_deal_never_breaches_and_frees_agency_the_instant_albums_are_met() {
    // term_weeks == 0 is the legacy sentinel: no term system existed yet.
    let mut deal = test_deal(50, 0.12);
    deal.albums_required = 1;
    let mut band = band::Band {
        record_deal: Some(deal),
        ..band::Band::default()
    };
    let outcome = band.fulfill_album_obligation(999_999);
    assert!(
        matches!(outcome, band::DealCompletionOutcome::FreeAgent { .. }),
        "legacy deals clear the instant the album count is met, exactly as before M9"
    );

    let mut deal2 = test_deal(50, 0.12);
    deal2.albums_required = 5;
    deal2.albums_delivered = 1; // owed
    let mut band2 = band::Band {
        record_deal: Some(deal2),
        ..band::Band::default()
    };
    assert!(
        band2.check_term_breach(999_999).is_none(),
        "a legacy deal (term_weeks == 0) can never breach"
    );
    assert!(band2.current_deal().is_some());
}

#[test]
fn old_save_with_no_signed_week_or_term_weeks_loads_as_legacy() {
    let json = r#"{
        "label_name": "Old Records",
        "label_tier": "Independent",
        "advance": 5000,
        "royalty_rate": 0.1,
        "albums_required": 1,
        "albums_delivered": 0
    }"#;
    let deal: band::RecordDeal = serde_json::from_str(json).expect("a legacy deal still loads");
    assert_eq!(deal.signed_week, 0);
    assert_eq!(deal.term_weeks, 0);
    assert!(
        deal.term_served(1_000_000),
        "legacy term is always considered served"
    );
    assert!(
        !deal.term_expired(1_000_000),
        "a legacy deal can never expire into a breach"
    );
}

// ---------------------------------------------------------------------------
// §E-4: the cooldown blocks new offers.
// ---------------------------------------------------------------------------

#[test]
fn deal_cooldown_ticks_down_and_floors_at_zero() {
    let mut band = band::Band {
        deal_cooldown: 2,
        ..band::Band::default()
    };
    band.tick_deal_cooldown();
    assert_eq!(band.deal_cooldown, 1);
    band.tick_deal_cooldown();
    assert_eq!(band.deal_cooldown, 0);
    band.tick_deal_cooldown();
    assert_eq!(band.deal_cooldown, 0, "the cooldown never wraps below zero");
}

#[test]
fn cooldown_blocks_new_offers_until_it_clears() {
    let mut game = test_game();
    game.band.fame = 90;
    game.band.albums_released = (0..5)
        .map(|i| test_release(i, ReleaseType::Album))
        .collect();
    game.band.singles_released = (0..5)
        .map(|i| test_release(100 + i, ReleaseType::Single))
        .collect();
    game.band.deal_cooldown = 5;

    let mut rng = StdRng::seed_from_u64(1);
    for _ in 0..100 {
        assert!(
            game.world
                .generate_deal_offers(&game.band, &game.data_files, &mut rng)
                .is_empty(),
            "no offer should surface while a breach cooldown is active"
        );
    }

    game.band.deal_cooldown = 0;
    let mut rng2 = StdRng::seed_from_u64(1);
    let mut resumed = false;
    for _ in 0..100 {
        if !game
            .world
            .generate_deal_offers(&game.band, &game.data_files, &mut rng2)
            .is_empty()
        {
            resumed = true;
            break;
        }
    }
    assert!(resumed, "offers resume once the cooldown clears");
}

// ---------------------------------------------------------------------------
// §E-4: the renewal window — new contract / extension / silence.
// ---------------------------------------------------------------------------

#[test]
fn renewal_window_offers_a_new_contract_when_recouped_with_decent_sales() {
    let mut game = test_game();
    game.week = 90;
    let mut deal = signed_deal_with_real_label(&game, 0, 100); // window opens week 74
    deal.albums_delivered = 1; // fully delivered
    deal.unrecouped = 0; // recouped
    let old_royalty = deal.royalty_rate;
    game.band.record_deal = Some(deal);
    game.band.reputation.commercial_success = 50; // decent sales

    let mut rng = StdRng::seed_from_u64(7);
    let offer: PotentialDealOffer = game
        .world
        .generate_renewal_offer(&game.band, &game.data_files, &mut rng, game.week)
        .expect("recouped + decent sales should produce a new-contract offer");
    assert_eq!(
        offer.carry_forward_unrecouped, 0,
        "a new contract starts a fresh ledger"
    );
    assert!(
        offer.royalty_rate > old_royalty,
        "a new contract bumps the royalty rate"
    );
    assert!(offer.term_weeks > 0);
}

#[test]
fn renewal_window_offers_an_extension_when_not_yet_recouped() {
    let mut game = test_game();
    game.week = 90;
    let mut deal = signed_deal_with_real_label(&game, 0, 100);
    deal.albums_delivered = 1;
    deal.advance = 10_000;
    deal.unrecouped = 3_000; // owed, but not deep in the red
    let old_royalty = deal.royalty_rate;
    game.band.record_deal = Some(deal);
    game.band.reputation.commercial_success = 15; // middling, not "weak"

    let mut rng = StdRng::seed_from_u64(7);
    let offer = game
        .world
        .generate_renewal_offer(&game.band, &game.data_files, &mut rng, game.week)
        .expect("not-yet-recouped should produce an extension offer");
    assert_eq!(
        offer.carry_forward_unrecouped, 3_000,
        "the extension carries the old balance forward"
    );
    assert_eq!(
        offer.royalty_rate, old_royalty,
        "royalty unchanged on an extension"
    );
    assert_eq!(offer.albums_required, DEAL_EXTENSION_ALBUMS);
    assert_eq!(offer.term_weeks, DEAL_EXTENSION_TERM_WEEKS);
}

#[test]
fn renewal_window_stays_silent_when_deep_in_the_red_with_weak_sales() {
    let mut game = test_game();
    game.week = 90;
    let mut deal = signed_deal_with_real_label(&game, 0, 100);
    deal.albums_delivered = 1;
    deal.unrecouped = 50_000; // deep in the red
    game.band.record_deal = Some(deal);
    game.band.reputation.commercial_success = 2; // weak sales

    let mut rng = StdRng::seed_from_u64(7);
    let offer =
        game.world
            .generate_renewal_offer(&game.band, &game.data_files, &mut rng, game.week);
    assert!(
        offer.is_none(),
        "deep in the red with weak sales gets silence, not an offer"
    );
}

#[test]
fn renewal_window_closed_with_albums_still_owed_or_outside_the_window() {
    let mut game = test_game();
    let mut rng = StdRng::seed_from_u64(7);

    // Albums still owed: the window never opens, however close the term.
    let mut deal = signed_deal_with_real_label(&game, 0, 100);
    deal.albums_required = 2;
    deal.albums_delivered = 1;
    game.band.record_deal = Some(deal);
    game.week = 90;
    assert!(
        game.world
            .generate_renewal_offer(&game.band, &game.data_files, &mut rng, game.week)
            .is_none()
    );

    // Fully delivered, but the window hasn't opened yet.
    game.band.record_deal.as_mut().unwrap().albums_delivered = 2;
    game.band.record_deal.as_mut().unwrap().albums_required = 2;
    game.week = 10;
    assert!(
        game.world
            .generate_renewal_offer(&game.band, &game.data_files, &mut rng, game.week)
            .is_none()
    );
}

// ---------------------------------------------------------------------------
// §E-5: label memos.
// ---------------------------------------------------------------------------

#[test]
fn write_songs_memo_fires_when_nothing_written_and_the_band_is_idle() {
    let mut game = test_game();
    let mut deal = signed_deal_with_real_label(&game, 0, 200); // far from any deadline
    deal.albums_required = 5;
    game.band.record_deal = Some(deal);
    game.band.unreleased_songs.clear();
    game.idle_streak = DEAL_MEMO_IDLE_WEEKS;
    game.week = 10;

    let mut fired = false;
    for seed in 0..300u64 {
        game.turn_log.clear();
        let mut rng = StdRng::seed_from_u64(seed);
        game.label_weekly_deal_check(&mut rng);
        if game
            .turn_log
            .iter()
            .any(|m| m.contains("We need songs on tape"))
        {
            fired = true;
            break;
        }
    }
    assert!(
        fired,
        "the write-songs memo should fire across enough rolls"
    );
}

#[test]
fn cut_single_memo_fires_when_cuttable_material_sits_idle() {
    let mut game = test_game();
    let deal = signed_deal_with_real_label(&game, 0, 200);
    game.band.record_deal = Some(deal);
    let mut album = test_release(1, ReleaseType::Album);
    album.singles_cut = 0;
    game.band.albums_released.push(album);
    game.idle_streak = DEAL_MEMO_IDLE_WEEKS;
    game.week = 10;

    let mut fired = false;
    for seed in 0..300u64 {
        game.turn_log.clear();
        let mut rng = StdRng::seed_from_u64(seed);
        game.label_weekly_deal_check(&mut rng);
        if game.turn_log.iter().any(|m| m.contains("Cut a single")) {
            fired = true;
            break;
        }
    }
    assert!(fired, "the cut-single memo should fire across enough rolls");
}

#[test]
fn deadline_memo_fires_and_stress_bites_every_week_in_the_final_window() {
    let mut game = test_game();
    let mut deal = signed_deal_with_real_label(&game, 0, 100); // ends week 100
    deal.albums_required = 3;
    deal.albums_delivered = 1; // owed
    game.band.record_deal = Some(deal);
    game.week = 95; // 5 weeks left — inside the final-12-week window

    let stress_before = game.player.stress;
    let mut rng = StdRng::seed_from_u64(0);
    game.label_weekly_deal_check(&mut rng);
    assert_eq!(
        game.player.stress,
        stress_before
            .saturating_add(DEAL_MEMO_DEADLINE_STRESS_PER_WEEK)
            .min(MAX_STRESS),
        "the deadline's stress bite applies even if this week's message didn't roll"
    );

    let mut fired = false;
    for seed in 0..300u64 {
        game.turn_log.clear();
        let mut rng = StdRng::seed_from_u64(seed);
        game.label_weekly_deal_check(&mut rng);
        if game
            .turn_log
            .iter()
            .any(|m| m.contains("The contract says"))
        {
            fired = true;
            break;
        }
    }
    assert!(fired, "the deadline memo should fire across enough rolls");
}

#[test]
fn no_memos_fire_once_unsigned() {
    let mut game = test_game();
    game.band.record_deal = None;
    game.idle_streak = 20;
    game.week = 10;
    let mut rng = StdRng::seed_from_u64(0);
    game.label_weekly_deal_check(&mut rng);
    assert!(
        !game.turn_log.iter().any(|m| m.contains('📠')),
        "no label memo should ever fire for an unsigned act"
    );
}

// ---------------------------------------------------------------------------
// §E-5: recoup pressure scales the single-cut odds.
// ---------------------------------------------------------------------------

#[test]
fn recoup_pressure_drops_the_idle_gate_from_three_to_two() {
    let mut game = test_game();
    let mut album = test_release(1, ReleaseType::Album);
    album.week_released = 0;
    game.band.albums_released.push(album);
    game.idle_streak = 2; // below the un-pressured gate (3), at the pressured one (2)
    game.week = 100; // well past the release cooldown

    // Without pressure (unrecouped == 0), idle_streak == 2 must never cut.
    game.band.record_deal = Some(test_deal(50, 0.12));
    for seed in 0..200u64 {
        let mut rng = StdRng::seed_from_u64(seed);
        game.just_released_music.clear();
        game.label_single_cut_check(&mut rng);
        assert!(
            game.just_released_music.is_empty(),
            "idle_streak=2 must never cut without recoup pressure"
        );
    }

    // Under pressure (unrecouped > 0), idle_streak == 2 clears the gate.
    let mut deal = test_deal(50, 0.12);
    deal.unrecouped = 5_000;
    game.band.record_deal = Some(deal);
    game.band.albums_released[0].singles_cut = 0;
    let mut fired = false;
    for seed in 0..200u64 {
        let mut rng = StdRng::seed_from_u64(seed);
        game.just_released_music.clear();
        game.label_single_cut_check(&mut rng);
        if !game.just_released_music.is_empty() {
            fired = true;
            break;
        }
    }
    assert!(
        fired,
        "under recoup pressure, idle_streak=2 should eventually cut"
    );
}
