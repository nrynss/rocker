//! Label world entities + the weekly wholesale ledger (design §A, L-ENTITY).
//! Worldgen seeds every JSON label as a `Label` with a balance sheet; a signed
//! player's sales book the label's wholesale margin to its `capital`, and the
//! money the label spends (advance, pressing, promo) leaves it. The player's
//! own income and recoupment are unchanged — this only adds the label's side.

use crate::data_loader::GameDataFiles;
use crate::game::genre::MusicGenre;
use crate::game::timeline::MusicTimeline;
use crate::game::world::{
    ChartRegion, GameWorld, Label, LabelStatus, LabelStrategy, LabelTier, RosterSlot,
    SCENE_MAX_BANDS, SCENE_MIN_BANDS,
};
use rand::SeedableRng;
use rand::rngs::StdRng;
use std::collections::BTreeMap;

use super::*;

/// The per-copy wholesale spread the label keeps before paying the artist —
/// the constants under test, so the assertions track them if they retune.
fn per_copy_spread() -> f32 {
    LABEL_WHOLESALE_PER_COPY - LABEL_MANUFACTURING_PER_COPY - LABEL_MECHANICAL_PER_COPY
}

// ---------------------------------------------------------------------------
// §A: worldgen seeds the industry deterministically.
// ---------------------------------------------------------------------------

#[test]
fn worldgen_seeds_labels_deterministically() {
    let data = GameDataFiles::load().expect("data files present");
    let world_a = GameWorld::new(&data, &mut StdRng::seed_from_u64(42));
    let world_b = GameWorld::new(&data, &mut StdRng::seed_from_u64(42));

    // Every major/independent/boutique from the JSON becomes a `Label`.
    let seed = data.get_record_labels_data();
    let expected =
        seed.major_labels.len() + seed.independent_labels.len() + seed.boutique_labels.len();
    assert_eq!(world_a.labels.len(), expected, "one entity per seed label");
    assert_eq!(expected, 16, "6 majors + 7 independents + 3 boutiques");

    // Same seed → byte-identical label maps (the seed is RNG-free anyway).
    let summary = |w: &GameWorld| {
        w.labels
            .iter()
            .map(|(id, l)| (*id, l.name.clone(), l.tier, l.capital, l.market_reach))
            .collect::<Vec<_>>()
    };
    assert_eq!(
        summary(&world_a),
        summary(&world_b),
        "worldgen seeds labels deterministically"
    );

    // Ids run in JSON order: majors first, so id 0 is the first major.
    let atlantic = &world_a.labels[&0];
    assert_eq!(atlantic.name, "Atlantic Records");
    assert_eq!(atlantic.tier, LabelTier::Major);
    assert_eq!(atlantic.founded_year, 1947);
    assert!(
        !atlantic.genres.is_empty(),
        "specialty genres parse onto the enum"
    );

    // Capital is the documented cube of financial_power.
    assert_eq!(
        atlantic.capital,
        i64::from(atlantic.financial_power).pow(3) * LABEL_CAPITAL_PER_FP_CUBED,
        "capital seeds from financial_power³"
    );

    // A major starts flush, a boutique lean — but the boutique still solvent.
    let boutique = world_a
        .labels
        .values()
        .find(|l| l.name == "Rough Trade Records")
        .expect("Rough Trade seeded");
    assert_eq!(boutique.tier, LabelTier::Boutique);
    assert!(
        atlantic.capital > boutique.capital * 50,
        "a major dwarfs a boutique: {} vs {}",
        atlantic.capital,
        boutique.capital
    );
    assert!(
        boutique.capital > i64::from(boutique.financial_power),
        "even a lean boutique has real working capital: {}",
        boutique.capital
    );

    // Fresh labels open with a clean, active balance sheet.
    assert_eq!(atlantic.ledger_ytd, 0);
    assert!(atlantic.roster.is_empty(), "roster is L-ROSTER's job (§D)");
    assert!(
        atlantic.hit_window.is_empty(),
        "hit_window is L-REACH's (§C)"
    );
}

// ---------------------------------------------------------------------------
// §A: signing debits the advance from the label's balance sheet.
// ---------------------------------------------------------------------------

#[test]
fn signing_debits_the_advance_from_label_capital() {
    let mut game = test_game();
    let offer = test_deal_offer(&game, None);
    let label_name = offer.label_name.clone();
    let advance = offer.advance;
    assert!(advance > 0, "the fixture offer carries an advance");

    let before = game
        .world
        .labels
        .values()
        .find(|l| l.name == label_name)
        .expect("the offer's label is a seeded world entity")
        .capital;

    game.pending_deal_offers.push(offer);
    game.action_accept_deal(0).expect("signing succeeds");

    // The deal is linked to the label entity by id.
    let label_id = game
        .band
        .current_deal()
        .expect("signed")
        .label_id
        .expect("the deal links to its label entity");
    let label = &game.world.labels[&label_id];
    assert_eq!(label.name, label_name, "linked to the right label");

    // The advance left the label's balance sheet (and its ytd ledger).
    assert_eq!(
        label.capital,
        before - i64::from(advance),
        "the advance debits the label's capital"
    );
    assert_eq!(label.ledger_ytd, -i64::from(advance));
}

// ---------------------------------------------------------------------------
// §A: pressing + promo outlay debits the label, matching the ledger accrual.
// ---------------------------------------------------------------------------

#[test]
fn pressing_and_promo_debit_the_label_capital() {
    let mut game = test_game();
    game.band.fame = 40;
    let offer = test_deal_offer(&game, None);
    game.pending_deal_offers.push(offer);
    game.action_accept_deal(0).expect("signing succeeds");
    let label_id = game.band.current_deal().unwrap().label_id.unwrap();

    let unrecouped_before = game.band.current_deal().unwrap().unrecouped;
    let capital_before = game.world.labels[&label_id].capital;

    // A release goes out; the label puts its promo machine (and a pressing
    // run) behind it — the same outlay that accrues to the recoupment ledger.
    game.just_released_music
        .push(test_release(1, ReleaseType::Single));
    game.apply_label_promo();

    let outlay = game.band.current_deal().unwrap().unrecouped - unrecouped_before;
    assert!(outlay > 0, "pressing + promo is a real outlay");
    assert_eq!(
        game.world.labels[&label_id].capital,
        capital_before - i64::from(outlay),
        "the pressing + promo outlay leaves the label's capital, matching the ledger accrual"
    );
}

// ---------------------------------------------------------------------------
// §A: a signed player's sales move their label's capital upward.
// ---------------------------------------------------------------------------

#[test]
fn signed_players_sales_move_their_labels_capital_up() {
    let mut game = test_game();
    game.band.fame = 60;
    // A touring act's presence so the record sells beyond the UK home floor.
    give_regional_presence(&mut game, 80);

    let offer = test_deal_offer(&game, None);
    game.pending_deal_offers.push(offer);
    game.action_accept_deal(0).expect("signing succeeds");
    let label_id = game.band.current_deal().unwrap().label_id.unwrap();
    let after_signing = game.world.labels[&label_id].capital;

    // A strong release runs through the first-run sales pass (uncapped, so it
    // neither sells out nor triggers an auto-repress — the margin is the only
    // capital move here).
    let mut release = test_release(1, ReleaseType::Single);
    release.release_quality = 90;
    release.week_released = 0;
    release.copies_pressed = 0;
    game.just_released_music.push(release);
    game.week = INITIAL_SALES_WINDOW_WEEKS;
    game.process_music_releases_and_marketing();

    let sold = game.band.singles_released[0].copies_sold;
    assert!(sold > 0, "the release sold copies");
    let after_sales = game.world.labels[&label_id].capital;
    assert!(
        after_sales > after_signing,
        "the label's wholesale margin lifted its capital: {} → {}",
        after_signing,
        after_sales
    );
}

/// The precise accounting: while the label is still owed (nothing reaches the
/// player), it keeps the full wholesale spread on every copy — the Albini
/// asymmetry as a live number. Booked capital == copies_sold × the spread.
#[test]
fn label_keeps_the_full_wholesale_spread_while_unrecouped() {
    let mut game = test_game();
    game.band.fame = 60;
    give_regional_presence(&mut game, 80);

    let offer = test_deal_offer(&game, None);
    game.pending_deal_offers.push(offer);
    game.action_accept_deal(0).expect("signing succeeds");
    let label_id = game.band.current_deal().unwrap().label_id.unwrap();

    // Deep in the red so this pass's whole royalty pays the ledger down and
    // $0 reaches the player — the label retains all of it as margin.
    game.band.record_deal.as_mut().unwrap().unrecouped = 100_000_000;
    let money_before = game.player.money;
    let capital_before = game.world.labels[&label_id].capital;

    // Year stays 1970 (pre-1973), so nothing certifies → no cert auto-repress;
    // uncapped run → no sold-out auto-repress. The margin is the only move.
    let mut release = test_release(1, ReleaseType::Single);
    release.release_quality = 90;
    release.week_released = 0;
    release.copies_pressed = 0;
    game.just_released_music.push(release);
    game.week = INITIAL_SALES_WINDOW_WEEKS;
    game.process_music_releases_and_marketing();

    let release = &game.band.singles_released[0];
    assert_eq!(release.certified, 0, "pre-1973: nothing certifies");
    assert_eq!(
        game.player.money, money_before,
        "not a cent reaches the player while the label is owed"
    );

    let expected_margin = (release.copies_sold as f32 * per_copy_spread()) as i64;
    assert!(expected_margin > 0);
    assert_eq!(
        game.world.labels[&label_id].capital - capital_before,
        expected_margin,
        "the label banked the full wholesale spread on every copy sold ({} copies)",
        release.copies_sold
    );
}

/// The catalog tail books margin too: a signed release selling on over many
/// weeks keeps lifting its label's capital.
#[test]
fn catalog_tail_sales_book_label_margin() {
    let mut game = test_game();
    game.band.fame = 70;
    give_regional_presence(&mut game, 80);

    let offer = test_deal_offer(&game, None);
    game.pending_deal_offers.push(offer);
    game.action_accept_deal(0).expect("signing succeeds");
    let label_id = game.band.current_deal().unwrap().label_id.unwrap();
    // Deep red so no royalty leaks to the player and the ledger never clears
    // mid-test — isolates the tail margin as the capital mover.
    game.band.record_deal.as_mut().unwrap().unrecouped = 100_000_000;

    let mut release = test_release(1, ReleaseType::Album);
    release.name = "Slow Burner".to_string();
    release.week_released = 0;
    release.initial_sales_score = 2000; // a genuine sustained hit
    release.copies_pressed = 5_000_000; // huge run so tail sells, never repress
    game.band.albums_released.push(release);

    let capital_before = game.world.labels[&label_id].capital;
    for wk in (INITIAL_SALES_WINDOW_WEEKS + 1)..(INITIAL_SALES_WINDOW_WEEKS + 30) {
        game.week = wk;
        game.process_music_releases_and_marketing();
    }

    let sold = game.band.albums_released[0].copies_sold;
    assert!(sold > 0, "the tail sold copies over the weeks");
    let booked = game.world.labels[&label_id].capital - capital_before;
    let ideal = (sold as f32 * per_copy_spread()) as i64;
    // The tail books margin week by week, so per-week `as i64` truncations
    // accumulate; the total lands within a dollar-per-week of the ideal.
    assert!(
        (ideal - booked).abs() <= 100 && booked > 0,
        "the catalog tail booked ~the wholesale spread on its {sold} copies: \
         booked {booked} vs ideal {ideal}"
    );
}

/// The gangbuster target (§A/§I, spot check): a Gold-volume run nets the label
/// low-six-figures of margin, a Platinum-volume run seven figures — at the
/// era-real L-SCALE scale.
#[test]
fn gold_and_platinum_volumes_net_era_real_margin() {
    // Directly exercise the per-copy spread at the certification volumes.
    let spread = per_copy_spread();
    let gold = (CERT_GOLD_THRESHOLD as f32 * spread) as i64;
    let platinum = (CERT_PLATINUM_THRESHOLD as f32 * spread) as i64;
    assert!(
        (100_000..1_000_000).contains(&gold),
        "a Gold record (150k copies) nets low-six-figures: ${gold}"
    );
    assert!(
        platinum >= 1_000_000,
        "a Platinum record (400k copies) nets seven figures: ${platinum}"
    );
}

// ---------------------------------------------------------------------------
// §A / serde: saves round-trip; pre-0.8 saves load with no labels.
// ---------------------------------------------------------------------------

#[test]
fn labels_and_deal_link_round_trip_through_a_save() {
    let data = GameDataFiles::load().expect("data files present");
    let world = GameWorld::new(&data, &mut StdRng::seed_from_u64(7));
    let json = serde_json::to_string(&world).expect("world serializes");
    let loaded: GameWorld = serde_json::from_str(&json).expect("world round-trips");

    let summary = |w: &GameWorld| {
        w.labels
            .iter()
            .map(|(id, l)| {
                (
                    *id,
                    l.name.clone(),
                    l.tier,
                    l.capital,
                    l.market_reach,
                    l.genres.clone(),
                    l.status,
                    l.strategy,
                )
            })
            .collect::<Vec<_>>()
    };
    assert_eq!(
        summary(&world),
        summary(&loaded),
        "labels survive the save round-trip"
    );
    assert!(!loaded.labels.is_empty());

    // A RecordDeal's label_id survives the trip too.
    let mut deal = test_deal(70, 0.12);
    deal.label_id = Some(3);
    let dj = serde_json::to_string(&deal).expect("deal serializes");
    let dloaded: band::RecordDeal = serde_json::from_str(&dj).expect("deal round-trips");
    assert_eq!(dloaded.label_id, Some(3));
}

#[test]
fn pre_0_8_save_loads_with_no_labels() {
    // A GameWorld written before the `labels` field existed has no "labels"
    // key; serde(default) fills an empty map (the ledger then simply no-ops).
    let data = GameDataFiles::load().expect("data files present");
    let world = GameWorld::new(&data, &mut StdRng::seed_from_u64(7));
    let mut value = serde_json::to_value(&world).expect("serializes");
    value
        .as_object_mut()
        .expect("world is an object")
        .remove("labels");
    let loaded: GameWorld = serde_json::from_value(value).expect("a pre-0.8 world must still load");
    assert!(loaded.labels.is_empty(), "no labels key → empty map");

    // A RecordDeal from before label_id existed defaults it to None, and the
    // ledger then resolves the player's label by name instead.
    let json = r#"{
        "label_name": "Old Records",
        "label_tier": "Independent",
        "advance": 5000,
        "royalty_rate": 0.1,
        "albums_required": 1,
        "albums_delivered": 0
    }"#;
    let deal: band::RecordDeal = serde_json::from_str(json).expect("a legacy deal still loads");
    assert_eq!(
        deal.label_id, None,
        "pre-0.8 deals default label_id to None"
    );
}

// ===========================================================================
// §C (L-REACH): reach + tier drift, hit-window recording, and the live deal
// read-through. Reach and tier stop being constants and drift toward a target
// derived from capital + recent charting hits, bounded per period with a
// hysteresis dead-band; a new release freezes the label's *live* reach while
// each release keeps its own frozen reach for its catalogue tail.
// ===========================================================================

/// A well-capitalized label riding a hot streak drifts its reach UP over time,
/// moving no more than `REACH_DRIFT_STEP` points per period (a trajectory, not
/// a teleport), and never past the ceiling.
#[test]
fn a_hot_well_capitalized_label_drifts_reach_up() {
    let data = GameDataFiles::load().expect("data files present");
    let mut world = GameWorld::new(&data, &mut StdRng::seed_from_u64(1));

    // A lean boutique made suddenly flush and hot: its target is far above the
    // seed reach it starts at.
    let label = world
        .labels
        .values_mut()
        .find(|l| l.name == "SST Records")
        .expect("SST seeded");
    label.capital = 12_000_000;
    for id in 0..HIT_WINDOW_LEN as u32 {
        label.record_chart_hit(id);
    }
    let start = label.market_reach;
    let target = label.reach_target();
    assert!(
        target > start + REACH_DRIFT_DEADBAND,
        "flush + hot should target a reach well above the seed {start} (target {target})"
    );

    // One period moves toward target, bounded by the per-period step.
    label.drift_reach_and_tier();
    let after_one = label.market_reach;
    assert!(
        after_one > start && after_one - start <= REACH_DRIFT_STEP,
        "one period rises by at most {REACH_DRIFT_STEP}: {start} -> {after_one}"
    );

    // Many periods climb toward the target and never overshoot the ceiling.
    let mut prev = after_one;
    for _ in 0..200 {
        label.drift_reach_and_tier();
        assert!(
            label.market_reach >= prev && label.market_reach <= REACH_MAX,
            "reach rises monotonically and stays bounded: {prev} -> {}",
            label.market_reach
        );
        prev = label.market_reach;
    }
    assert!(
        label.market_reach >= target - REACH_DRIFT_DEADBAND,
        "reach settles at the target band: {} vs target {target}",
        label.market_reach
    );
}

/// A label bleeding money with a cold roster drifts its reach DOWN toward the
/// floor, bounded per period.
#[test]
fn a_cold_broke_label_drifts_reach_down() {
    let data = GameDataFiles::load().expect("data files present");
    let mut world = GameWorld::new(&data, &mut StdRng::seed_from_u64(1));

    // A major that has bled into the red with nothing charting: target ~0.
    let label = world
        .labels
        .values_mut()
        .find(|l| l.name == "Atlantic Records")
        .expect("Atlantic seeded");
    label.capital = -500_000;
    label.hit_window.clear();
    let start = label.market_reach;
    assert_eq!(
        label.reach_target(),
        0,
        "broke + cold targets the floor (from a start of {start})"
    );

    label.drift_reach_and_tier();
    let after_one = label.market_reach;
    assert!(
        after_one < start && start - after_one <= REACH_DRIFT_STEP,
        "one period falls by at most {REACH_DRIFT_STEP}: {start} -> {after_one}"
    );

    for _ in 0..200 {
        label.drift_reach_and_tier();
    }
    assert!(
        label.market_reach <= REACH_DRIFT_DEADBAND,
        "a cold, broke label cools to the floor band (within the {REACH_DRIFT_DEADBAND}-point \
         hysteresis dead-band of target 0), got {}",
        label.market_reach
    );
}

/// The drift pass is wired into the weekly world tick but only *acts* on its
/// period cadence, so the yearly movement stays bounded even though it's called
/// every week.
#[test]
fn the_drift_pass_acts_on_its_period_cadence() {
    let data = GameDataFiles::load().expect("data files present");
    let mut world = GameWorld::new(&data, &mut StdRng::seed_from_u64(1));
    let id = *world
        .labels
        .iter()
        .find(|(_, l)| l.name == "SST Records")
        .map(|(id, _)| id)
        .expect("SST seeded");
    world.labels.get_mut(&id).unwrap().capital = 12_000_000; // target well above seed

    let start = world.labels[&id].market_reach;
    // The weeks before a period boundary are no-ops.
    for _ in 0..(REACH_DRIFT_PERIOD_WEEKS - 1) {
        world.drift_labels();
        assert_eq!(
            world.labels[&id].market_reach, start,
            "reach holds between period boundaries"
        );
    }
    // The period-boundary week actually moves it.
    world.drift_labels();
    assert!(
        world.labels[&id].market_reach > start,
        "the cadence week drifts reach: {start} -> {}",
        world.labels[&id].market_reach
    );
}

/// Tier follows reach across the thresholds, but a promotion also needs
/// sustained capital, and demotion is pure reach (with a hysteresis gap).
#[test]
fn tier_follows_reach_but_promotion_needs_sustained_capital() {
    use LabelTier::*;

    // Reach clears the Major bar, capital does not → no promotion.
    assert_ne!(
        Label::tier_for(
            Independent,
            TIER_MAJOR_REACH_PROMOTE,
            TIER_MAJOR_MIN_CAPITAL - 1
        ),
        Major,
        "a reach spike without the balance sheet does not make a Major"
    );
    // Same reach, capital now sustained → promotes.
    assert_eq!(
        Label::tier_for(
            Independent,
            TIER_MAJOR_REACH_PROMOTE,
            TIER_MAJOR_MIN_CAPITAL
        ),
        Major,
        "reach + sustained capital together promote to Major"
    );
    // Just below the reach bar, ample capital → still no promotion.
    assert_ne!(
        Label::tier_for(Independent, TIER_MAJOR_REACH_PROMOTE - 1, i64::MAX),
        Major,
        "capital alone, below the reach bar, is not enough"
    );
    // Boutique → Independent has its own capital floor too.
    assert_eq!(
        Label::tier_for(
            Boutique,
            TIER_INDEPENDENT_REACH_PROMOTE,
            TIER_INDEPENDENT_MIN_CAPITAL
        ),
        Independent
    );
    assert_ne!(
        Label::tier_for(
            Boutique,
            TIER_INDEPENDENT_REACH_PROMOTE,
            TIER_INDEPENDENT_MIN_CAPITAL - 1
        ),
        Independent,
        "a boutique gets rich before it gets big"
    );
    // Demotion is pure reach — a Major that loses reach falls however rich.
    assert_eq!(
        Label::tier_for(Major, TIER_MAJOR_REACH_DEMOTE - 1, i64::MAX),
        Independent,
        "losing distribution demotes a Major regardless of cash"
    );
    // Hysteresis: a Major at the demote threshold (not below it) holds tier.
    assert_eq!(
        Label::tier_for(Major, TIER_MAJOR_REACH_DEMOTE, 0),
        Major,
        "the promote/demote gap stops boundary flip-flop"
    );
}

/// §C requirement 1: when a signed player's release charts, it is recorded in
/// the player's label's rolling hit window.
#[test]
fn a_signed_players_charting_release_feeds_its_labels_hit_window() {
    let mut game = test_game();
    game.band.fame = 70;
    give_regional_presence(&mut game, 80);

    let offer = test_deal_offer(&game, None);
    game.pending_deal_offers.push(offer);
    game.action_accept_deal(0).expect("signing succeeds");
    let label_id = game.band.current_deal().unwrap().label_id.unwrap();
    assert!(
        game.world.labels[&label_id].hit_window.is_empty(),
        "a fresh label has an empty hit window"
    );

    // A strong release charts (fresh world → empty boards → it enters at #1).
    let mut release = test_release(7, ReleaseType::Single);
    release.release_quality = 95;
    release.week_released = 0;
    release.copies_pressed = 0;
    game.just_released_music.push(release);
    game.week = INITIAL_SALES_WINDOW_WEEKS;
    game.process_music_releases_and_marketing();

    assert!(
        game.band.singles_released[0].peak_chart_position.is_some(),
        "the release charted"
    );
    assert_eq!(
        game.world.labels[&label_id].hit_window,
        vec![7],
        "the charting release id fed the label's hit window"
    );
}

/// §C requirement 4: a NEW signed release freezes the label's *live* reach —
/// not the value frozen into the deal at signing.
#[test]
fn a_new_signed_release_freezes_the_labels_live_reach() {
    let mut game = test_game();
    game.band.fame = 50;
    let offer = test_deal_offer(&game, None);
    let signing_reach = offer.original_label_data.market_reach;
    game.pending_deal_offers.push(offer);
    game.action_accept_deal(0).expect("signing succeeds");
    let label_id = game.band.current_deal().unwrap().label_id.unwrap();
    assert_eq!(
        game.band.current_deal().unwrap().market_reach,
        signing_reach,
        "the deal froze the label's reach at signing"
    );
    // At signing, the live reach equals the frozen one.
    assert_eq!(game.new_release_frozen_reach(), Some(signing_reach));

    // The label rises (§C): its live reach drifts well above the signing value.
    let live_reach = signing_reach.saturating_add(25).min(REACH_MAX);
    assert_ne!(live_reach, signing_reach);
    game.world.labels.get_mut(&label_id).unwrap().market_reach = live_reach;

    // Recording a NEW single freezes the label's CURRENT reach onto it.
    game.band.unreleased_songs.push(music::Song {
        id: 0,
        name: "New Cut".to_string(),
        songwriting_quality: 50,
    });
    game.player.money = 1_000_000;
    let mut rng = StdRng::seed_from_u64(0);
    game.action_record_single(None, &mut rng)
        .expect("the label covers a signed release");
    let release = game.just_released_music.last().expect("recorded");
    assert_eq!(
        release.label_market_reach,
        Some(live_reach),
        "a new release reads the label's live reach ({live_reach}), not the deal's frozen {signing_reach}"
    );
}

/// §C requirement 4 (invariant): an older release's catalogue tail keeps its
/// own frozen reach when the label's live reach drifts on — the sales-fix
/// invariant that each release freezes `label_market_reach` for its tail.
#[test]
fn an_old_releases_tail_keeps_its_frozen_reach_when_the_label_drifts() {
    let mut game = test_game();
    game.band.fame = 50;
    let offer = test_deal_offer(&game, None);
    let signing_reach = offer.original_label_data.market_reach;
    game.pending_deal_offers.push(offer);
    game.action_accept_deal(0).expect("signing succeeds");
    let label_id = game.band.current_deal().unwrap().label_id.unwrap();

    // A release cut earlier froze the label's reach as it was then.
    let mut old = test_release(1, ReleaseType::Album);
    old.label_market_reach = Some(signing_reach);
    game.band.albums_released.push(old);

    // The label's live reach moves on.
    let live_reach = signing_reach.saturating_add(25).min(REACH_MAX);
    assert_ne!(live_reach, signing_reach);
    game.world.labels.get_mut(&label_id).unwrap().market_reach = live_reach;

    // A new release would read the live reach...
    assert_eq!(game.new_release_frozen_reach(), Some(live_reach));
    // ...but the old release's tail reach is untouched by the drift.
    assert_eq!(
        game.band.albums_released[0].label_market_reach,
        Some(signing_reach),
        "each release keeps its own frozen reach for its catalogue tail"
    );
}

/// Regression: the read-through falls back to the deal's signing reach when the
/// label is not a seeded world entity (the "Test Records" placeholder / a
/// pre-0.8 save), so nothing changes for an unseeded deal.
#[test]
fn frozen_reach_falls_back_to_the_deal_for_an_unseeded_label() {
    let mut game = test_game();
    game.band.record_deal = Some(test_deal(70, 0.12)); // "Test Records", not seeded
    assert_eq!(
        game.new_release_frozen_reach(),
        Some(70),
        "an unseeded label falls back to the deal's frozen reach"
    );
    // And an unsigned band freezes nothing (an indie release).
    game.band.record_deal = None;
    assert_eq!(game.new_release_frozen_reach(), None);
}

// ---------------------------------------------------------------------------
// §D (L-ROSTER): labels scout/sign scene acts by fit + appetite, and a signed
// act's chart presence books wholesale margin to its label's capital.
// ---------------------------------------------------------------------------

/// A minimal Independent-tier candidate cloned off a real seed, so every field
/// is valid; only the levers under test (genres, financial_power, capital) are
/// set. Empty roster, `Active`.
fn candidate_label(base: &Label, id: u32, genres: Vec<MusicGenre>, fp: u8, capital: i64) -> Label {
    let mut label = base.clone();
    label.id = id;
    label.tier = LabelTier::Independent;
    label.status = LabelStatus::Active;
    label.genres = genres;
    label.financial_power = fp;
    label.capital = capital;
    label.roster.clear();
    label
}

/// The advance a signing costs, mirrored from `world::labels::scene_signing_cost`
/// (private there) so these assertions track the constants if they retune.
fn signing_cost(fame: u8) -> i64 {
    ROSTER_SIGNING_BASE_COST + i64::from(fame) * ROSTER_SIGNING_COST_PER_FAME
}

#[test]
fn labels_sign_scene_acts_by_fit_and_appetite() {
    let data = GameDataFiles::load().expect("data files present");
    let mut world = GameWorld::new(&data, &mut StdRng::seed_from_u64(1));
    let base = world.labels[&0].clone();

    // Two otherwise-identical indies at the same weight class, both flush enough
    // to front the advance: only their genre specialty differs.
    let fame = 30u8;
    let fp = 30u8; // weight class sits right on fame 30
    let flush = 5_000_000i64;
    world.labels.clear();
    world.labels.insert(
        0,
        candidate_label(&base, 0, vec![MusicGenre::Rock], fp, flush),
    );
    world.labels.insert(
        1,
        candidate_label(&base, 1, vec![MusicGenre::Punk], fp, flush),
    );

    // Fit decides: a Punk act goes to the Punk-specialty label, a Rock act to
    // the Rock one — genre specialty overlap is the tiebreak between equals.
    assert_eq!(world.pick_signing_label(fame, &MusicGenre::Punk), Some(1));
    assert_eq!(world.pick_signing_label(fame, &MusicGenre::Rock), Some(0));
    // Neither's specialty (Jazz): fit ties at 0, so the deterministic lowest-id
    // tiebreak picks id 0 — a definite, reproducible choice, never a panic.
    assert_eq!(world.pick_signing_label(fame, &MusicGenre::Jazz), Some(0));

    // Appetite: a broke label signs less. Push the Punk label deep into the red
    // — now even a Punk act (its own specialty) can't sign there, and the flush
    // Rock label takes it instead.
    world.labels.get_mut(&1).unwrap().capital = -1;
    assert_eq!(
        world.pick_signing_label(fame, &MusicGenre::Punk),
        Some(0),
        "a label in the red is skipped even for a perfect-fit act"
    );

    // A label that cannot comfortably cover the advance (capital below the
    // safety margin) also stays out — with both labels lean, nobody signs.
    world.labels.get_mut(&0).unwrap().capital = signing_cost(fame); // < safety × cost
    assert_eq!(
        world.pick_signing_label(fame, &MusicGenre::Punk),
        None,
        "no label with the means to sign → the act stays unsigned"
    );

    // A boutique can't reach a marquee act: fame far above its financial_power
    // is out of appetite range entirely.
    world.labels.clear();
    world.labels.insert(
        0,
        candidate_label(&base, 0, vec![MusicGenre::Rock], 15, flush),
    );
    assert_eq!(
        world.pick_signing_label(90, &MusicGenre::Rock),
        None,
        "a fame-90 act is beyond a financial_power-15 label's appetite"
    );
}

#[test]
fn a_signed_scene_acts_chart_activity_lifts_its_labels_capital() {
    let data = GameDataFiles::load().expect("data files present");
    let mut world = GameWorld::new(&data, &mut StdRng::seed_from_u64(2));

    // Sign a scene band to a label: register it on the roster and mark the band
    // (the scene's signed-to source of truth) so a reconcile keeps the slot.
    let band_name = world.bands[0].name.clone();
    let label_id = 0u32;
    let label_name = world.labels[&label_id].name.clone();
    world.bands[0].label = Some(label_name.clone());
    world
        .labels
        .get_mut(&label_id)
        .unwrap()
        .roster
        .push(RosterSlot {
            band_name: band_name.clone(),
            signed_week: 0,
        });

    // The act charts on two boards. (Scene entries are never `is_player`.)
    world.submit_chart_entry(
        ChartRegion::Local,
        "Breakout".into(),
        band_name.clone(),
        false,
        1000,
    );
    world.submit_chart_entry(
        ChartRegion::Uk,
        "Breakout".into(),
        band_name.clone(),
        false,
        800,
    );

    let presence: u64 = world
        .regional_charts
        .values()
        .flat_map(|entries| entries.iter())
        .filter(|entry| entry.band_name == band_name)
        .map(|entry| u64::from(entry.score))
        .sum();
    assert!(presence > 0, "the act has real chart presence");

    let before = world.labels[&label_id].capital;
    world.book_scene_roster_margin();
    let after = world.labels[&label_id].capital;

    assert!(
        after > before,
        "the roster act's chart activity lifted its label's capital: {before} → {after}"
    );
    assert_eq!(
        after - before,
        (presence as f32 * ROSTER_MARGIN_PER_CHART_SCORE) as i64,
        "capital rose by the summed chart presence × the roster margin scale"
    );

    // A label with no charting roster act books nothing.
    let idle = world
        .labels
        .values()
        .find(|l| l.roster.is_empty())
        .unwrap()
        .id;
    let idle_before = world.labels[&idle].capital;
    world.book_scene_roster_margin();
    assert_eq!(
        world.labels[&idle].capital, idle_before,
        "an empty roster earns nothing"
    );
}

#[test]
fn rosters_activate_the_industry_while_the_scene_stays_bounded() {
    let data = GameDataFiles::load().expect("data files present");
    let timeline = MusicTimeline::new(&data);
    let mut world = GameWorld::new(&data, &mut StdRng::seed_from_u64(3));
    let mut rng = StdRng::seed_from_u64(4);

    let seeded: BTreeMap<u32, i64> = world
        .labels
        .iter()
        .map(|(id, l)| (*id, l.capital))
        .collect();

    // Signing is pure bookkeeping: it sets `label`/`roster`, never the roster
    // of *bands*. The scene population is the existing sim's job, untouched.
    let before = world.bands.len();
    world.scout_and_sign_scene_acts(&mut StdRng::seed_from_u64(9), &mut Vec::new());
    assert_eq!(
        world.bands.len(),
        before,
        "the signing pass must not add or drop scene bands"
    );

    for _ in 0..200 {
        world.update_week(&timeline, &data, &mut rng);
        let n = world.bands.len();
        assert!(
            (SCENE_MIN_BANDS..=SCENE_MAX_BANDS).contains(&n),
            "scene size {n} left its designed ~180 band bounds"
        );
    }
    // Still bounded at the end — the ~180 scene never blew up or collapsed.
    assert!((SCENE_MIN_BANDS..=SCENE_MAX_BANDS).contains(&world.bands.len()));

    // The industry is alive: labels signed scene acts and capital moved.
    assert!(
        world.labels.values().any(|l| !l.roster.is_empty()),
        "labels should have signed scene acts to their rosters"
    );
    assert!(
        world.labels.iter().any(|(id, l)| l.capital != seeded[id]),
        "at least one label's capital should have moved off its seed"
    );
    // The point of L-ROSTER: a major now carries a roster instead of sitting
    // static at seed capital.
    let major_active = world
        .labels
        .values()
        .any(|l| l.tier == LabelTier::Major && !l.roster.is_empty());
    assert!(major_active, "a major should be active with a roster");
}

#[test]
fn scene_roster_signing_and_booking_is_deterministic() {
    let data = GameDataFiles::load().expect("data files present");
    let timeline = MusicTimeline::new(&data);
    let run = || {
        let mut world = GameWorld::new(&data, &mut StdRng::seed_from_u64(5));
        let mut rng = StdRng::seed_from_u64(6);
        for _ in 0..120 {
            world.update_week(&timeline, &data, &mut rng);
        }
        world
    };
    let summary = |w: &GameWorld| {
        w.labels
            .iter()
            .map(|(id, l)| {
                (
                    *id,
                    l.capital,
                    l.roster
                        .iter()
                        .map(|s| s.band_name.clone())
                        .collect::<Vec<_>>(),
                )
            })
            .collect::<Vec<_>>()
    };
    assert_eq!(
        summary(&run()),
        summary(&run()),
        "same seed → identical rosters and label capital, signing and booking alike"
    );
}

// ===========================================================================
// §F (L-MNA): distress, acquisition, folding. A label whose capital stays in
// the red long enough goes Distressed; a distressed label with a roster worth
// buying is acquired by a flush major (roster + the player's contract transfer,
// terms carrying), and one nobody buys folds (active deals void via the breach
// write-off path). All rolls derive from the injected world seed.
// ===========================================================================

/// §F requirement 1: a label held below the distress floor for N consecutive
/// weeks flips to `Distressed`; climbing back out restores `Active`.
#[test]
fn a_label_below_the_floor_for_n_weeks_goes_distressed_then_can_recover() {
    let data = GameDataFiles::load().expect("data files present");
    let mut world = GameWorld::new(&data, &mut StdRng::seed_from_u64(1));
    let mut rng = StdRng::seed_from_u64(2);
    let mut news = Vec::new();

    // A seeded label pushed into the red, empty roster (so no acquirer bids —
    // this isolates the distress clock from the acquisition/fold resolution).
    let id = *world.labels.keys().next().expect("labels seeded");
    {
        let label = world.labels.get_mut(&id).unwrap();
        label.capital = -50_000;
        label.roster.clear();
    }
    assert_eq!(world.labels[&id].status, LabelStatus::Active);

    // It does not distress until the clock crosses the threshold.
    for wk in 1..DISTRESS_WEEKS {
        world.run_label_mna(&mut rng, &mut news);
        assert_eq!(
            world.labels[&id].status,
            LabelStatus::Active,
            "still Active after {wk} weeks in the red (threshold is {DISTRESS_WEEKS})"
        );
    }
    world.run_label_mna(&mut rng, &mut news);
    assert_eq!(
        world.labels[&id].status,
        LabelStatus::Distressed,
        "{DISTRESS_WEEKS} consecutive weeks below the floor flips the label Distressed"
    );
    assert!(
        news.iter().any(|n| n.contains("distress")),
        "a label going distressed is a news line, got {news:?}"
    );

    // Recovery (§F: a label that can be saved is not lost): back above the floor
    // restores Active and resets the clock.
    world.labels.get_mut(&id).unwrap().capital = 5_000_000;
    world.run_label_mna(&mut rng, &mut news);
    assert_eq!(
        world.labels[&id].status,
        LabelStatus::Active,
        "climbing back out of the red recovers the label to Active"
    );
    assert_eq!(world.labels[&id].weeks_below_floor, 0, "the clock resets");
}

/// §F requirement 2: an acquisition transfers the distressed label's roster to
/// the acquirer AND retags the player's `label_id` onto the acquirer — terms
/// carry, the acquirer's reach applies going forward.
#[test]
fn acquisition_transfers_the_roster_and_retags_the_players_deal() {
    let mut game = test_game();
    game.band.fame = 55;

    // Sign the player to a seeded (Independent) label — the acquisition target.
    let offer = test_deal_offer(&game, None);
    game.pending_deal_offers.push(offer);
    game.action_accept_deal(0).expect("signing succeeds");
    let target_id = game.band.current_deal().unwrap().label_id.unwrap();
    let deal_before = game.band.current_deal().unwrap().clone();

    // Put a real scene band on the target's roster (the acquirer will absorb it).
    let scene_band = game.world.bands[0].name.clone();
    let target_name = game.world.labels[&target_id].name.clone();
    game.world.bands[0].label = Some(target_name.clone());
    game.world
        .labels
        .get_mut(&target_id)
        .unwrap()
        .roster
        .push(RosterSlot {
            band_name: scene_band.clone(),
            signed_week: 0,
        });

    // Force the target into distress (a flush major acquirer is seeded already).
    {
        let label = game.world.labels.get_mut(&target_id).unwrap();
        label.status = LabelStatus::Distressed;
        label.capital = -100_000;
    }

    // Drive the world M&A until the acquisition bid lands.
    let mut rng = StdRng::seed_from_u64(7);
    let mut news = Vec::new();
    let mut acquirer_id = None;
    for _ in 0..500 {
        game.world.run_label_mna(&mut rng, &mut news);
        if game.world.labels[&target_id].status == LabelStatus::Acquired {
            acquirer_id = game.world.labels[&target_id].acquired_by;
            break;
        }
    }
    let acquirer_id =
        acquirer_id.expect("a flush major should acquire a distressed rostered label");
    assert_ne!(acquirer_id, target_id);
    assert_eq!(
        game.world.labels[&acquirer_id].tier,
        LabelTier::Major,
        "only a flush major acquires"
    );

    // Roster transferred to the acquirer; the scene act is retagged; the
    // acquired label is dormant with an empty roster.
    let acquirer_name = game.world.labels[&acquirer_id].name.clone();
    assert!(
        game.world.labels[&acquirer_id]
            .roster
            .iter()
            .any(|s| s.band_name == scene_band),
        "the acquired roster moved to the acquirer"
    );
    assert_eq!(
        game.world.bands[0].label.as_deref(),
        Some(acquirer_name.as_str()),
        "the scene act now records for the acquirer"
    );
    assert!(
        game.world.labels[&target_id].roster.is_empty(),
        "the acquired label's roster is emptied"
    );

    // The player's contract retags onto the acquirer; terms carry unchanged.
    game.reconcile_player_label_mna();
    let deal_after = game
        .band
        .current_deal()
        .expect("still under contract after an acquisition");
    assert_eq!(
        deal_after.label_id,
        Some(acquirer_id),
        "the deal's label_id retags to the acquirer"
    );
    assert_eq!(deal_after.label_name, acquirer_name, "and its label name");
    assert_eq!(
        deal_after.market_reach, game.world.labels[&acquirer_id].market_reach,
        "the acquirer's reach now applies going forward"
    );
    assert_eq!(deal_after.advance, deal_before.advance, "advance carries");
    assert_eq!(
        deal_after.royalty_rate, deal_before.royalty_rate,
        "royalty carries"
    );
    assert_eq!(
        deal_after.albums_required, deal_before.albums_required,
        "albums owed carry"
    );
    assert_eq!(
        deal_after.unrecouped, deal_before.unrecouped,
        "the recoupment ledger carries"
    );
    assert_eq!(
        deal_after.term_weeks, deal_before.term_weeks,
        "the term carries"
    );
    assert_eq!(
        deal_after.signed_week, deal_before.signed_week,
        "the signing week carries"
    );
    assert!(
        game.take_turn_log()
            .iter()
            .any(|l| l.contains("acquired by") && l.contains("contract moves")),
        "the player sees the acquisition on their own deal"
    );
}

/// §F requirement 3: a fold voids the player's deal into free agency, writing
/// off any `unrecouped` balance (reusing the breach write-off path) WITHOUT the
/// punitive breach cooldown — a fold is not the band's breach — and it is news.
#[test]
fn a_fold_voids_the_players_deal_into_clean_free_agency() {
    let mut game = test_game();
    game.band.fame = 40;
    let offer = test_deal_offer(&game, None);
    game.pending_deal_offers.push(offer);
    game.action_accept_deal(0).expect("signing succeeds");
    let target_id = game.band.current_deal().unwrap().label_id.unwrap();

    // A real balance still owed, to be written off; capture the money + cooldown.
    game.band.record_deal.as_mut().unwrap().unrecouped = 25_000;
    let money_before = game.player.money;
    let cooldown_before = game.band.deal_cooldown;

    // Force the label to fold: distressed, empty roster (nobody buys it), and
    // already at the fold horizon.
    {
        let label = game.world.labels.get_mut(&target_id).unwrap();
        label.status = LabelStatus::Distressed;
        label.capital = -100_000;
        label.roster.clear();
        label.weeks_below_floor = FOLD_WEEKS;
    }
    let mut rng = StdRng::seed_from_u64(3);
    let mut news = Vec::new();
    game.world.run_label_mna(&mut rng, &mut news);
    assert_eq!(
        game.world.labels[&target_id].status,
        LabelStatus::Folding,
        "a distressed label nobody buys, past the fold horizon, folds"
    );
    assert!(
        news.iter().any(|n| n.contains("folded")),
        "a folding label is a news line, got {news:?}"
    );

    // The player's deal voids into free agency.
    game.reconcile_player_label_mna();
    assert!(
        game.band.current_deal().is_none(),
        "the player is a free agent after the fold"
    );
    assert_eq!(
        game.player.money, money_before,
        "the write-off is never charged to the player"
    );
    assert_eq!(
        game.band.deal_cooldown, cooldown_before,
        "a fold is not the band's breach — no punitive cooldown"
    );
    let log = game.take_turn_log();
    assert!(
        log.iter().any(|l| l.contains("free agent")),
        "the player sees a free-agent news line, got {log:?}"
    );
    assert!(
        log.iter().any(|l| l.contains("written off")),
        "the $25,000 still owed is written off, and it is announced, got {log:?}"
    );
}

/// §F / determinism: the whole M&A pass replays identically for a seed — the
/// same distress transitions, the same acquisition/fold outcomes, the same news
/// line for line (the one roll is the injected world rng, labels iterate in
/// BTreeMap id order).
#[test]
fn the_mna_pass_is_deterministic_per_seed() {
    let data = GameDataFiles::load().expect("data files present");
    let run = || {
        let mut world = GameWorld::new(&data, &mut StdRng::seed_from_u64(9));
        // Push every non-major label into distress with a roster, so the pass
        // exercises acquisition rolls (and, for any nobody buys, folding).
        let ids: Vec<u32> = world.labels.keys().copied().collect();
        for (n, id) in ids.iter().enumerate() {
            let label = world.labels.get_mut(id).unwrap();
            if label.tier != LabelTier::Major {
                label.status = LabelStatus::Distressed;
                label.capital = -10_000;
                label.weeks_below_floor = DISTRESS_WEEKS;
                label.roster.push(RosterSlot {
                    band_name: format!("Roster Act {n}"),
                    signed_week: 0,
                });
            }
        }
        let mut rng = StdRng::seed_from_u64(4);
        let mut news = Vec::new();
        for _ in 0..120 {
            world.run_label_mna(&mut rng, &mut news);
        }
        (world, news)
    };
    let summary = |w: &GameWorld| {
        w.labels
            .iter()
            .map(|(id, l)| {
                (
                    *id,
                    l.status,
                    l.capital,
                    l.acquired_by,
                    l.weeks_below_floor,
                    l.roster.len(),
                )
            })
            .collect::<Vec<_>>()
    };
    let (world_a, news_a) = run();
    let (world_b, news_b) = run();
    assert_eq!(
        summary(&world_a),
        summary(&world_b),
        "same seed → identical M&A outcomes across every label"
    );
    assert_eq!(
        news_a, news_b,
        "same seed → identical M&A news, line for line"
    );
    // The setup must actually have exercised M&A for the determinism to mean
    // something: at least one label was acquired.
    assert!(
        world_a
            .labels
            .values()
            .any(|l| l.status == LabelStatus::Acquired),
        "the pass should have resolved some distressed labels by acquisition"
    );
}

/// §F / serde: the L-MNA fields default cleanly on a save written before they
/// existed, and round-trip once present.
#[test]
fn mna_fields_default_on_pre_l_mna_saves_and_round_trip() {
    let data = GameDataFiles::load().expect("data files present");
    let mut world = GameWorld::new(&data, &mut StdRng::seed_from_u64(7));
    let id = *world.labels.keys().next().unwrap();
    {
        let label = world.labels.get_mut(&id).unwrap();
        label.status = LabelStatus::Distressed;
        label.weeks_below_floor = 12;
        label.acquired_by = Some(3);
    }
    // Round-trip preserves the new fields.
    let json = serde_json::to_string(&world).expect("serializes");
    let loaded: GameWorld = serde_json::from_str(&json).expect("round-trips");
    assert_eq!(loaded.labels[&id].weeks_below_floor, 12);
    assert_eq!(loaded.labels[&id].acquired_by, Some(3));
    assert_eq!(loaded.labels[&id].status, LabelStatus::Distressed);

    // A label object written before the L-MNA fields existed loads with the
    // defaults (0 / None / Active) via serde(default).
    let mut value = serde_json::to_value(&world).expect("serializes");
    let label_val = &mut value["labels"][id.to_string()];
    let obj = label_val.as_object_mut().expect("label is an object");
    obj.remove("weeks_below_floor");
    obj.remove("acquired_by");
    obj.remove("status");
    let loaded: GameWorld =
        serde_json::from_value(value).expect("a pre-L-MNA label must still load");
    assert_eq!(loaded.labels[&id].weeks_below_floor, 0);
    assert_eq!(loaded.labels[&id].acquired_by, None);
    assert_eq!(loaded.labels[&id].status, LabelStatus::Active);
}

/// L-SIM preview (§F/§I): with M&A active, distress resolves into acquisition or
/// folding rather than leaving a label sitting deep in the red. The
/// L-ENTITY/L-ROSTER economy keeps labels solidly solvent (the companion
/// `game::sim::label_capital_trajectories_over_fifteen_years` sweep bottoms in
/// the high six figures, zero insolvent), so M&A is a *safety valve* natural
/// play rarely trips. To exercise the valve deterministically this sweep bleeds
/// the two leanest labels into the red at the start of each run (one is also
/// handed a roster outright), then plays 15 years of world ticks. A label with a
/// roster a flush major wants is acquired; a label left with no roster nobody
/// buys folds — so the run naturally produces acquisitions in the majority and
/// foldings as the rarer outcome, matching §F's "majors consolidated the
/// independents". It confirms every red label is resolved within the fold
/// horizon and none lingers past it. Prints the tally; the calibrated
/// population bounds are L-SIM's to own.
#[test]
#[ignore = "L-MNA safety-valve sweep: cargo test -- --ignored --nocapture"]
fn mna_resolves_injected_distress_within_the_fold_horizon() {
    let data = GameDataFiles::load().expect("data files present");
    let timeline = MusicTimeline::new(&data);
    const SEEDS: u64 = 24;
    let horizon = 15 * WEEKS_PER_YEAR;

    let mut acquisitions = 0u32;
    let mut foldings = 0u32;
    let mut distress_events = 0u32;
    let mut deepest_capital = i64::MAX;
    let mut max_weeks_in_red = 0u16;

    for seed in 1..=SEEDS {
        let mut world = GameWorld::new(&data, &mut StdRng::seed_from_u64(seed));
        let mut rng = StdRng::seed_from_u64(seed.wrapping_mul(2).wrapping_add(100));

        // Bleed the two leanest labels into the red to exercise the valve.
        let mut by_capital: Vec<(u32, i64)> = world
            .labels
            .iter()
            .map(|(id, l)| (*id, l.capital))
            .collect();
        by_capital.sort_by_key(|&(_, cap)| cap);
        let rostered = by_capital[0].0;
        let orphan = by_capital[1].0;
        // Give the rostered one a real scene band so a major has something to buy.
        let band_name = world.bands[0].name.clone();
        world.bands[0].label = Some(world.labels[&rostered].name.clone());
        {
            let label = world.labels.get_mut(&rostered).unwrap();
            label.capital = -200_000;
            label.roster.push(RosterSlot {
                band_name,
                signed_week: 0,
            });
        }
        world.labels.get_mut(&orphan).unwrap().capital = -200_000;

        for _ in 0..horizon {
            let news = world.update_week(&timeline, &data, &mut rng);
            for line in &news {
                if line.contains("has folded") {
                    foldings += 1;
                } else if line.contains("acquires the distressed") {
                    acquisitions += 1;
                } else if line.contains("is in financial distress") {
                    distress_events += 1;
                }
            }
            for label in world.labels.values() {
                deepest_capital = deepest_capital.min(label.capital);
                max_weeks_in_red = max_weeks_in_red.max(label.weeks_below_floor);
            }
        }
    }

    println!("L-MNA safety valve over 15 yrs × {SEEDS} seeds (two labels bled red per seed):");
    println!(
        "  distress events {distress_events}, acquisitions {acquisitions}, foldings {foldings}"
    );
    println!("  deepest capital any label reached: ${deepest_capital}");
    println!(
        "  longest any label sat in the red: {max_weeks_in_red} weeks (fold horizon {FOLD_WEEKS})"
    );

    // Every seed bled two labels red — M&A must have resolved them, not left
    // them bleeding: at least one resolution per seed (acquisition or folding).
    assert!(
        acquisitions + foldings >= SEEDS as u32,
        "M&A should resolve the injected distress (acquisitions {acquisitions} + foldings {foldings} \
         over {SEEDS} seeds)"
    );
    // No label sits in the red past the fold horizon — the valve always fires.
    assert!(
        max_weeks_in_red <= FOLD_WEEKS + REACH_DRIFT_PERIOD_WEEKS as u16,
        "a label sat in the red for {max_weeks_in_red} weeks, past the fold horizon {FOLD_WEEKS} — \
         the valve failed to fire"
    );
}

// ---------------------------------------------------------------------------
// §G (L-CHAR): label character — strategy assignment + creative-freedom effect.
// ---------------------------------------------------------------------------

#[test]
fn worldgen_assigns_real_strategies_not_all_balanced() {
    let data = GameDataFiles::load().expect("data files present");
    let world = GameWorld::new(&data, &mut StdRng::seed_from_u64(42));

    // §G: strategy is load-bearing now — it is no longer the dormant `Balanced`
    // stub L-ENTITY seeded. The seed roster earns a spread of real strategies.
    let strategies: Vec<LabelStrategy> = world.labels.values().map(|l| l.strategy).collect();
    let count = |s: LabelStrategy| strategies.iter().filter(|&&x| x == s).count();

    assert!(
        strategies.iter().any(|&s| s != LabelStrategy::Balanced),
        "not all labels should be the neutral Balanced stub"
    );
    // Each named strategy the design calls for is represented on the roster
    // (so the personalities actually surface in play).
    for s in [
        LabelStrategy::HitFactory,
        LabelStrategy::ArtistFriendly,
        LabelStrategy::GenrePurist,
    ] {
        assert!(
            count(s) > 0,
            "expected at least one {s:?} label on the roster"
        );
    }

    // The assignment is a pure function of the (RNG-free) JSON personality, so a
    // different world seed produces the identical per-label strategy.
    let world_b = GameWorld::new(&data, &mut StdRng::seed_from_u64(999));
    for (id, label) in &world.labels {
        assert_eq!(
            label.strategy, world_b.labels[id].strategy,
            "strategy assignment must be deterministic for {}",
            label.name
        );
    }
}

#[test]
fn low_creative_freedom_raises_single_cut_aggression() {
    let data = GameDataFiles::load().expect("data files present");
    let world = GameWorld::new(&data, &mut StdRng::seed_from_u64(1));

    // Hold the strategy constant (Balanced) and vary ONLY creative_freedom, so
    // the effect under test is creative-freedom's alone (§G — "low freedom raises
    // single-cut aggression").
    let mut meddling = world.labels[&0].clone();
    meddling.strategy = LabelStrategy::Balanced;
    meddling.creative_freedom = 10;
    let mut hands_off = meddling.clone();
    hands_off.creative_freedom = 95;

    assert!(
        meddling.single_cut_aggression() > hands_off.single_cut_aggression(),
        "low creative freedom ({:.3}) must be more cut-aggressive than high ({:.3})",
        meddling.single_cut_aggression(),
        hands_off.single_cut_aggression()
    );

    // The midpoint is exactly neutral — a Balanced label there behaves precisely
    // as it did before L-CHAR (the invariant that keeps the v0.7 tests passing).
    let mut midpoint = meddling.clone();
    midpoint.creative_freedom = 50;
    assert!(
        (midpoint.single_cut_aggression() - 1.0).abs() < 1e-9,
        "Balanced at the creative-freedom midpoint must be exactly neutral (1.0)"
    );
}
