//! Label world entities + the weekly wholesale ledger (design §A, L-ENTITY).
//! Worldgen seeds every JSON label as a `Label` with a balance sheet; a signed
//! player's sales book the label's wholesale margin to its `capital`, and the
//! money the label spends (advance, pressing, promo) leaves it. The player's
//! own income and recoupment are unchanged — this only adds the label's side.

use crate::data_loader::GameDataFiles;
use crate::game::world::{GameWorld, LabelTier};
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
