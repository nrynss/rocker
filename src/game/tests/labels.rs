//! Label world entities + the weekly wholesale ledger (design §A, L-ENTITY).
//! Worldgen seeds every JSON label as a `Label` with a balance sheet; a signed
//! player's sales book the label's wholesale margin to its `capital`, and the
//! money the label spends (advance, pressing, promo) leaves it. The player's
//! own income and recoupment are unchanged — this only adds the label's side.

use crate::data_loader::GameDataFiles;
use crate::game::world::{GameWorld, Label, LabelTier};
use rand::SeedableRng;
use rand::rngs::StdRng;

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
