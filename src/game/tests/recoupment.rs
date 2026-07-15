//! Label recoupment + label auto-repress (design §E-2 and the §E-1 label
//! half). The label's money — the advance at signing, pressing and promo at
//! every release — is a loan on a ledger; royalties pay it down before the
//! player sees a cent, and a sold-out or newly-certified release makes the
//! label press a fresh (also-recouped) run.

use crate::game::music::ReleaseType;

use super::*;

/// The advance is banked by the player AND joins the deal's `unrecouped`
/// ledger the moment the deal is signed (design §E-2).
#[test]
fn advance_joins_the_recoupment_ledger_at_signing() {
    let mut game = test_game();
    let money_before = game.player.money;

    let offer = test_deal_offer(&game, None);
    let advance = offer.advance;
    assert!(advance > 0, "the fixture offer carries an advance");
    game.pending_deal_offers.push(offer);
    game.action_accept_deal(0).expect("signing succeeds");

    // Still banked as spending money...
    assert_eq!(
        game.player.money,
        money_before + advance as i32,
        "the player still banks the advance"
    );
    // ...but now owed right back to the label.
    let deal = game.band.current_deal().expect("signed to a label");
    assert_eq!(
        deal.unrecouped, advance as i32,
        "the advance seeds the recoupment ledger"
    );
}

/// Every release under the deal accrues the label's outlay — the pressing run
/// at $/copy plus the promo push at $/point — on top of the ledger.
#[test]
fn each_release_accrues_pressing_and_promo_to_the_ledger() {
    let mut game = test_game();
    game.band.fame = 40;
    game.band.record_deal = Some(test_deal(70, 0.12)); // ledger starts at 0
    game.just_released_music
        .push(test_release(1, ReleaseType::Single));

    game.apply_label_promo();

    let pressing = 70 * LABEL_PRESSING_PER_REACH + 40 * LABEL_PRESSING_PER_FAME;
    let push = (70u8 / 2).clamp(10, 45) as i32;
    let expected = (pressing as f32 * LABEL_RECOUP_PRESSING_PER_COPY) as i32
        + push * LABEL_RECOUP_PROMO_PER_PUSH;
    assert!(expected > 0, "the outlay is a real cost");
    assert_eq!(
        game.band.current_deal().unwrap().unrecouped,
        expected,
        "pressing + promo joined the ledger at release"
    );
}

/// Royalty income pays the ledger down before it reaches the player: while in
/// the red the bank balance never moves, and the ledger falls by the gross.
#[test]
fn royalties_pay_the_ledger_down_before_the_player_is_paid() {
    let mut game = test_game();
    game.band.fame = 30;
    let mut deal = test_deal(70, 0.12);
    deal.unrecouped = 1_000_000; // deep in the red
    game.band.record_deal = Some(deal);

    let money_before = game.player.money;
    let mut release = test_release(1, ReleaseType::Single);
    release.release_quality = 40; // modest — no sell-out, no certification
    release.week_released = 0;
    release.copies_pressed = 0; // uncapped, so it never sells out
    game.just_released_music.push(release);

    game.week = INITIAL_SALES_WINDOW_WEEKS;
    game.process_music_releases_and_marketing();

    let gross = game.band.singles_released[0].total_income_generated;
    assert!(gross > 0, "the record generated royalty");
    assert_eq!(
        game.player.money, money_before,
        "not a cent reaches the player while the label is owed"
    );
    assert_eq!(
        game.band.current_deal().unwrap().unrecouped,
        1_000_000 - gross as i32,
        "the whole royalty went to recoupment"
    );
    assert!(
        game.turn_log
            .iter()
            .any(|m| m.contains("⚖️ Label recouping:")),
        "the weekly recoup line fires while in the red"
    );
    assert!(
        !game
            .turn_log
            .iter()
            .any(|m| m.contains("Catalog royalties trickle in")),
        "no player-facing income line while nothing reaches the player"
    );
}

/// The moment the ledger clears, the recoup line stops, a one-shot "recouped
/// in full" line fires, and the remainder of the royalty reaches the player.
#[test]
fn recoup_log_stops_and_player_earns_once_cleared() {
    let mut game = test_game();
    game.band.fame = 30;
    // M10: a signed, touring act — regional presence so royalties are large
    // enough to overshoot the tiny ledger balance (else demand pins to the UK
    // home floor).
    give_regional_presence(&mut game, 80);
    let mut deal = test_deal(70, 0.12);
    deal.unrecouped = 100; // almost paid off
    game.band.record_deal = Some(deal);

    let money_before = game.player.money;
    let mut release = test_release(1, ReleaseType::Single);
    release.release_quality = 40;
    release.week_released = 0;
    release.copies_pressed = 0;
    game.just_released_music.push(release);

    game.week = INITIAL_SALES_WINDOW_WEEKS;
    game.process_music_releases_and_marketing();

    let gross = game.band.singles_released[0].total_income_generated;
    assert!(gross > 100, "the royalty overshoots the tiny balance");
    assert_eq!(
        game.band.current_deal().unwrap().unrecouped,
        0,
        "the ledger is cleared"
    );
    assert_eq!(
        game.player.money,
        money_before + (gross - 100) as i32,
        "the player pockets what is left after recoupment"
    );
    assert!(
        game.turn_log.iter().any(|m| m.contains("recouped in full")),
        "clearing the ledger is announced once"
    );
    assert!(
        !game
            .turn_log
            .iter()
            .any(|m| m.contains("⚖️ Label recouping:")),
        "the recoup line stops the week the ledger clears"
    );
}

/// A signed act's sold-out release makes the label press a fresh run:
/// `copies_pressed` grows by a full label run and that pressing cost joins the
/// ledger, with a news line (design §E-1 label half).
#[test]
fn sold_out_triggers_a_label_auto_repress() {
    let mut game = test_game();
    game.band.fame = 60;
    // M10: a touring act's regional presence so demand blows past a tiny run.
    give_regional_presence(&mut game, 80);
    game.band.record_deal = Some(test_deal(70, 0.12)); // ledger starts at 0

    let fresh_run = 70 * LABEL_PRESSING_PER_REACH + 60 * LABEL_PRESSING_PER_FAME;

    let mut release = test_release(1, ReleaseType::Single);
    release.release_quality = 90;
    release.week_released = 0;
    release.copies_pressed = 500; // tiny run — demand will blow past it
    game.just_released_music.push(release);

    game.week = INITIAL_SALES_WINDOW_WEEKS;
    game.process_music_releases_and_marketing();

    let release = &game.band.singles_released[0];
    assert_eq!(
        release.copies_pressed,
        500 + fresh_run,
        "the label restocked with a full fresh run on top of the sold-out one"
    );
    assert_eq!(
        game.band.current_deal().unwrap().unrecouped,
        (fresh_run as f32 * LABEL_RECOUP_PRESSING_PER_COPY) as i32,
        "the fresh run's pressing cost joined the ledger"
    );
    assert!(
        game.turn_log
            .iter()
            .any(|m| m.contains("fresh run") && m.contains("sold out")),
        "the auto-repress is announced as a sold-out restock"
    );
}

/// Crossing a certification level (without selling out) also triggers the
/// label auto-repress. A monster genre modifier forces a first-run hit large
/// enough to certify, which realistic scores never reach on their own.
#[test]
fn certification_triggers_a_label_auto_repress() {
    let mut game = test_game();
    game.band.fame = 60;
    // M10: a touring act's regional presence so first-run demand can reach
    // the certification threshold under the sum-over-territories model.
    give_regional_presence(&mut game, 80);
    game.band.record_deal = Some(test_deal(70, 0.12));

    let genre = genre::MusicGenre::Rock;
    // Force a runaway hit: certification lives on the catalog tail in normal
    // play, so we crank the era-independent genre modifier to drive first-run
    // demand past the Silver threshold in a single pass.
    game.world
        .dynamic_genre_modifiers
        .insert(genre.clone(), 100.0);

    let mut release = test_release(1, ReleaseType::Single);
    release.genre = Some(genre);
    release.release_quality = 90;
    release.week_released = 0;
    release.copies_pressed = 0; // uncapped, so it certifies without selling out
    game.just_released_music.push(release);

    game.week = INITIAL_SALES_WINDOW_WEEKS;
    game.process_music_releases_and_marketing();

    let release = &game.band.singles_released[0];
    assert!(
        release.certified >= 1,
        "the runaway hit certified (copies_sold = {})",
        release.copies_sold
    );
    // The certification fame bump lands before the repress sizes its run, so
    // the exact run count isn't hardcodable — but an uncapped release grew
    // from 0 by one full label run, and the ledger booked that run's cost.
    assert!(
        release.copies_pressed > 0,
        "an uncapped release was restocked by a fresh label run on certifying"
    );
    assert_eq!(
        game.band.current_deal().unwrap().unrecouped,
        (release.copies_pressed as f32 * LABEL_RECOUP_PRESSING_PER_COPY) as i32,
        "the fresh run's pressing cost joined the ledger"
    );
    assert!(
        game.turn_log
            .iter()
            .any(|m| m.contains("fresh run") && m.contains("certified")),
        "the auto-repress is announced as a certification restock"
    );
}

/// The structural point of §E-1: a signed release with strong sustained tail
/// demand re-presses over successive weeks — its stock never permanently runs
/// out at one ~12k label run — so its cumulative `copies_sold` can cross the
/// Silver certification threshold, each fresh run's cost joining the ledger.
#[test]
fn signed_release_re_presses_on_the_tail_and_can_certify() {
    let mut game = test_game();
    game.band.fame = 70;
    // M10: a signed, touring act — regional presence so the sustained tail
    // moves real volume across territories and can cross Silver.
    give_regional_presence(&mut game, 80);
    let mut deal = test_deal(90, 0.12);
    deal.unrecouped = 0; // already recouped — watch the ledger grow with represses
    game.band.record_deal = Some(deal);

    let first_run = 90 * LABEL_PRESSING_PER_REACH + 70 * LABEL_PRESSING_PER_FAME;
    let mut release = test_release(1, ReleaseType::Album);
    release.name = "Slow Burner".to_string();
    release.week_released = 0;
    release.initial_sales_score = 2000; // a genuine sustained hit
    release.copies_pressed = first_run;
    game.band.albums_released.push(release);

    let mut re_pressed = false;
    // Play out the tail week by week.
    for wk in (INITIAL_SALES_WINDOW_WEEKS + 1)..(INITIAL_SALES_WINDOW_WEEKS + 120) {
        game.week = wk;
        game.turn_log.clear();
        game.process_music_releases_and_marketing();
        if game.turn_log.iter().any(|m| m.contains("fresh run")) {
            re_pressed = true;
        }
    }

    let release = &game.band.albums_released[0];
    assert!(
        re_pressed,
        "the label re-pressed the release on the tail at least once"
    );
    assert!(
        release.copies_pressed > first_run,
        "copies_pressed grew past the first run via tail represses: {}",
        release.copies_pressed
    );
    assert!(
        release.copies_sold >= CERT_SILVER_THRESHOLD,
        "a signed act's sustained tail crosses Silver: {} copies sold",
        release.copies_sold
    );
    assert!(
        release.certified >= 1,
        "and it therefore certifies (level {})",
        release.certified
    );
    assert!(
        game.band.current_deal().unwrap().unrecouped > 0,
        "each fresh run's pressing cost joined the recoupment ledger"
    );
}

/// The other side of §E-1: an *indie* release must never auto-repress on the
/// tail — restocking a sold-out indie run is the player's M6 RePress, not the
/// game's. It simply stops selling when the pressing is gone.
#[test]
fn indie_release_never_auto_re_presses_on_the_tail() {
    let mut game = test_game();
    game.band.fame = 70;
    game.band.record_deal = None; // indie

    let mut release = test_release(1, ReleaseType::Album);
    release.name = "Indie Burner".to_string();
    release.week_released = 0;
    release.initial_sales_score = 2000;
    release.copies_pressed = 5_000; // a small self-pressed run
    game.band.albums_released.push(release);

    for wk in (INITIAL_SALES_WINDOW_WEEKS + 1)..(INITIAL_SALES_WINDOW_WEEKS + 30) {
        game.week = wk;
        game.process_music_releases_and_marketing();
    }

    let release = &game.band.albums_released[0];
    assert_eq!(
        release.copies_pressed, 5_000,
        "an indie pressing is never restocked automatically"
    );
    assert!(
        release.copies_sold <= 5_000,
        "an indie release can't sell past its one pressing: {}",
        release.copies_sold
    );
    assert!(
        !game.turn_log.iter().any(|m| m.contains("fresh run")),
        "no label auto-repress ever fires for an unsigned act"
    );
}

/// A pre-M5 save with no `unrecouped` field loads with the ledger at zero
/// (and `market_reach` still defaults too).
#[test]
fn old_save_defaults_unrecouped_to_zero() {
    let json = r#"{
        "label_name": "Old Records",
        "label_tier": "Independent",
        "advance": 5000,
        "royalty_rate": 0.1,
        "albums_required": 1,
        "albums_delivered": 0
    }"#;
    let deal: band::RecordDeal = serde_json::from_str(json).expect("a legacy deal still loads");
    assert_eq!(deal.unrecouped, 0, "pre-M5 deals owe nothing");
    assert_eq!(deal.market_reach, 50, "market_reach keeps its own default");
}
