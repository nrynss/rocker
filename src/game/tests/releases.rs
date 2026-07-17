//! Release economics: indie vs. label reach, pressing runs, charts, marketing,
//! recording safety, and era-genre fit.

use crate::game::music::{MarketingCampaignType, ReleaseType};
use rand::SeedableRng;
use rand::rngs::StdRng;

use super::*;

#[test]
fn unknown_indie_acts_reach_almost_nobody() {
    let mut game = test_game();
    game.band.record_deal = None;
    // M10: the act is known regionally (it has toured), so fame then drives
    // how much of that regional audience its indie distribution can reach.
    give_regional_presence(&mut game, 80);
    let release = test_release(1, ReleaseType::Single);

    game.band.fame = 5;
    let (unknown, _, _) = game.calculate_release_outcome(300, &release);
    game.band.fame = 95;
    let (famous, _, _) = game.calculate_release_outcome(300, &release);

    assert!(
        famous > unknown * 3,
        "a famous indie act should reach a far larger audience: {unknown} vs {famous}"
    );
}

#[test]
fn label_out_earns_indie_at_low_fame_but_not_at_high_fame() {
    let mut game = test_game();
    game.band.fame = 10;
    // M10: a touring act with regional presence, so the label's wider reach
    // (`market_reach`) actually multiplies through into territory sales — with
    // regional fame at 0 every act pins to the UK home floor and the reach
    // gap that this test is about never surfaces.
    give_regional_presence(&mut game, 80);
    let release = test_release(1, ReleaseType::Single);

    game.band.record_deal = None;
    let (indie_low, _, _) = game.calculate_release_outcome(300, &release);
    game.band.record_deal = Some(test_deal(90, 0.12));
    let (label_low, _, _) = game.calculate_release_outcome(300, &release);
    assert!(
        label_low > indie_low,
        "an unknown band should earn more through a label: label {label_low} vs indie {indie_low}"
    );

    game.band.fame = 95;
    game.band.record_deal = None;
    let (indie_high, _, _) = game.calculate_release_outcome(300, &release);
    assert!(
        indie_high > label_low * 2,
        "a superstar keeping everything should out-earn a royalty slice: indie {indie_high} vs label {label_low}"
    );
}

#[test]
fn pressing_costs_fall_on_indies_and_labels_press_for_you() {
    let mut game = test_game();
    game.band.record_deal = None;

    let garage = game.pressing_cost(&ReleaseType::Album, PRESSING_TIERS[0].1);
    let national = game.pressing_cost(&ReleaseType::Album, PRESSING_TIERS[3].1);
    assert!(garage > 0, "an indie band pays to press its own records");
    assert!(
        national > garage * 10,
        "a national run costs far more than a garage run: {garage} vs {national}"
    );
    let (copies, cost) = game
        .plan_pressing(&ReleaseType::Album, Some(0))
        .expect("tier 0 exists");
    assert_eq!(copies, PRESSING_TIERS[0].1);
    assert_eq!(cost, garage);

    game.band.record_deal = Some(test_deal(70, 0.10));
    game.band.fame = 40;
    let (label_copies, label_cost) = game
        .plan_pressing(&ReleaseType::Album, None)
        .expect("the label always presses");
    assert_eq!(label_cost, 0, "the label covers pressing when signed");
    assert_eq!(
        label_copies,
        70 * LABEL_PRESSING_PER_REACH + 40 * LABEL_PRESSING_PER_FAME,
        "the run scales with the label's network and the band's name"
    );
}

#[test]
fn a_pressing_can_sell_out() {
    let mut game = test_game();
    game.band.record_deal = None;
    game.band.fame = 60;
    // M10: a touring act's regional presence, so demand across the territories
    // can genuinely outrun a tiny garage run.
    give_regional_presence(&mut game, 80);

    let mut release = test_release(1, ReleaseType::Single);
    release.copies_pressed = 500;
    let (income, units, sold_out) = game.calculate_release_outcome(400, &release);
    assert!(sold_out, "demand should outstrip a garage run");
    assert_eq!(units, 500);
    // M7 (§F): income is copies × per-copy ÷ SALES_INCOME_DIVISOR — the copy
    // bump feeds certification, not the bank balance.
    assert_eq!(income, 500 * INDIE_INCOME_PER_COPY / SALES_INCOME_DIVISOR);

    release.copies_pressed = 50_000;
    let (_, units_uncapped, sold_out) = game.calculate_release_outcome(400, &release);
    assert!(!sold_out);
    assert!(units_uncapped > 500, "a bigger run keeps selling");
}

#[test]
fn signed_bands_do_not_run_their_own_marketing() {
    let mut game = test_game();
    game.just_released_music
        .push(test_release(7, ReleaseType::Single));
    game.band.record_deal = Some(test_deal(60, 0.12));

    let err = game
        .action_start_marketing_campaign(7, MarketingCampaignType::BasicPress)
        .unwrap_err();
    assert!(err.contains("job"), "unexpected error: {err}");
}

#[test]
fn a_hit_release_enters_the_charts_and_a_flop_misses() {
    let mut game = test_game();
    game.initialize_player("Test", "The Tests", genre::MusicGenre::Rock);
    // A crowded Local board: scene records the player has to outsell.
    for i in 0..10 {
        game.world.submit_chart_entry(
            world::ChartRegion::Local,
            format!("Scene Filler {i}"),
            "Scene Band".into(),
            false,
            200,
        );
    }

    // A famous band drops a great record...
    game.band.fame = 80;
    let mut hit = test_release(1, ReleaseType::Single);
    hit.name = "Big Hit".to_string();
    hit.release_quality = 90;
    game.just_released_music.push(hit);
    game.week = INITIAL_SALES_WINDOW_WEEKS; // the sales window has closed
    game.process_music_releases_and_marketing();

    assert!(
        game.world
            .regional_charts
            .get(&world::ChartRegion::Local)
            .is_some_and(|entries| entries.iter().any(|e| e.is_player && e.title == "Big Hit")),
        "a high-scoring release should land on the Local chart"
    );
    assert!(
        game.turn_log
            .iter()
            .any(|m| m.contains("enters the Local chart at #1")),
        "charting should be reported to the player, named by region (M10)"
    );

    // ...while a nobody's dud sinks without a trace.
    game.band.fame = 0;
    let mut flop = test_release(2, ReleaseType::Single);
    flop.name = "Total Flop".to_string();
    flop.release_quality = 1;
    flop.week_released = game.week;
    game.just_released_music.push(flop);
    game.week += INITIAL_SALES_WINDOW_WEEKS;
    game.process_music_releases_and_marketing();

    assert!(
        !game
            .world
            .regional_charts
            .get(&world::ChartRegion::Local)
            .is_some_and(|entries| entries.iter().any(|e| e.title == "Total Flop")),
        "a flop should not crack the Local chart"
    );
}

#[test]
fn failed_recording_does_not_eat_songs() {
    let mut game = test_game();
    game.band.unreleased_songs.push(music::Song {
        id: 0,
        name: "Keeper".to_string(),
        songwriting_quality: 50,
    });
    game.player.money = 0;

    let mut rng = StdRng::seed_from_u64(0);
    assert!(game.action_record_single(Some(0), &mut rng).is_err());
    assert_eq!(
        game.band.unreleased_songs.len(),
        1,
        "songs must survive a recording attempt the player cannot afford"
    );
}

#[test]
fn a_release_riding_the_era_outsells_one_against_it() {
    let mut game = test_game();
    game.band.fame = 40;
    game.world.dynamic_genre_modifiers.clear(); // era taste is the only genre input

    let year = game.timeline.get_current_year();
    let era_fit =
        |genre: &genre::MusicGenre| game.data_files.era_genre_modifier(year, genre.aliases());
    let hot = genre::MusicGenre::ALL
        .iter()
        .max_by(|a, b| era_fit(a).total_cmp(&era_fit(b)))
        .expect("genres exist")
        .clone();
    let cold = genre::MusicGenre::ALL
        .iter()
        .min_by(|a, b| era_fit(a).total_cmp(&era_fit(b)))
        .expect("genres exist")
        .clone();
    assert!(
        era_fit(&hot) > era_fit(&cold),
        "the era should actually have tastes"
    );

    let mut on_trend = test_release(1, ReleaseType::Single);
    on_trend.genre = Some(hot);
    let mut against_the_grain = test_release(2, ReleaseType::Single);
    against_the_grain.genre = Some(cold);

    assert!(
        game.calculate_release_sales_score(&on_trend)
            > game.calculate_release_sales_score(&against_the_grain),
        "identical records should sell by the era's tastes"
    );
}

#[test]
fn post_launch_marketing_increases_catalog_tail_sales() {
    let mut game = test_game();
    game.band.fame = 30;
    game.band.record_deal = None; // indie, so we can see raw income

    // Create a release that has passed its launch window.
    let mut release = test_release(1, ReleaseType::Single);
    release.week_released = 0;
    release.release_quality = 80;
    release.copies_pressed = 10_000;
    game.band.singles_released.push(release);

    // Manually set the initial sales score (simulating post-launch window calculation).
    // This needs to be high enough that the tail calculation produces > 10 score.
    game.band.singles_released[0].initial_sales_score = 500;

    // Advance to week 10 (well past the 4-week initial window).
    game.week = 10;

    // First pass: catalog income without marketing campaign.
    game.process_music_releases_and_marketing();
    let income_without_marketing = game.band.singles_released[0].total_income_generated;

    // Reset income and copies sold for second test.
    game.band.singles_released[0].total_income_generated = 0;
    game.band.singles_released[0].copies_sold = 0;

    // Now add an active marketing campaign and run week 11.
    game.week = 11;
    let campaign = music::ActiveMarketingCampaign {
        campaign_type: music::MarketingCampaignType::BasicPress,
        start_week: 11,
        end_week: 15,
        effectiveness_bonus: 10,
    };
    game.band.singles_released[0]
        .active_marketing
        .push(campaign);

    game.process_music_releases_and_marketing();
    let income_with_marketing = game.band.singles_released[0].total_income_generated;

    // With the living tail model, post-launch marketing should increase catalog sales.
    assert!(
        income_with_marketing > income_without_marketing,
        "catalog income should be higher with active marketing: {} vs {}",
        income_with_marketing,
        income_without_marketing
    );
}

/// Issue #21: when a signed first run sells out, the log must read in story
/// order — 💿 sales → 📦 sold out → 🏭 restock — and the 📦 line must report
/// the run that actually sold out, not the ledger-inflated post-re-press
/// count (the auto-repress mutates `copies_pressed` before the lines are
/// emitted).
#[test]
fn sold_out_log_reports_the_first_run_and_precedes_the_restock() {
    let mut game = test_game();
    game.band.fame = 60;
    // M10: a touring act's regional presence so demand blows past a tiny run.
    give_regional_presence(&mut game, 80);
    game.band.record_deal = Some(test_deal(70, 0.12));

    let mut release = test_release(1, ReleaseType::Single);
    release.release_quality = 90;
    release.week_released = 0;
    release.copies_pressed = 500; // tiny run — demand will blow past it
    game.just_released_music.push(release);

    game.week = INITIAL_SALES_WINDOW_WEEKS;
    game.process_music_releases_and_marketing();

    let position = |needle: &str| game.turn_log.iter().position(|m| m.contains(needle));
    let sales = position("💿").expect("the sales line fires");
    let sold_out = position("📦").expect("the sold-out line fires");
    let restock = position("fresh run").expect("the restock line fires");
    assert!(
        game.turn_log[sold_out].contains("all 500 copies gone"),
        "📦 reports the run that sold out, not the post-re-press total: {}",
        game.turn_log[sold_out]
    );
    assert!(
        sales < sold_out && sold_out < restock,
        "story order is sales → sold out → restock; got 💿 at {sales}, 📦 at {sold_out}, \
         🏭 at {restock}: {:?}",
        game.turn_log
    );
}

/// Issue #20's bug class, player side: the per-copy pressing bill must stay
/// coupled to the M7 sales rescale. A fully-sold pressing run — any tier,
/// single or album — must out-earn its own bill in EVERY era, no matter the
/// era's `recording_cost_modifier`. Before the fix, `PRESSING_PER_COPY_ALBUM`
/// was still on the pre-M7 scale ($0.50 against $0.667/copy income), so a
/// sold-out album run lost money unconditionally in every era with a cost
/// modifier above 4/3 — five of the ten.
#[test]
fn a_fully_sold_pressing_run_out_earns_its_bill_in_every_era() {
    let game = test_game();
    for era in game.timeline.eras.values() {
        let modifier = era.recording_cost_modifier;
        for (tier, copies) in PRESSING_TIERS {
            for (kind, setup, per_copy) in [
                ("single", PRESSING_SETUP_SINGLE, PRESSING_PER_COPY_SINGLE),
                ("album", PRESSING_SETUP_ALBUM, PRESSING_PER_COPY_ALBUM),
            ] {
                let bill = (setup + per_copy * copies as f32) * modifier;
                // The most a run can ever earn: every pressed copy sold, at
                // indie per-copy income after the M7 divisor.
                let max_income = (copies * INDIE_INCOME_PER_COPY / SALES_INCOME_DIVISOR) as f32;
                assert!(
                    max_income > bill,
                    "a fully-sold {kind} {tier} must out-earn its pressing bill in '{}' \
                     (cost modifier {modifier}): income ${max_income} vs bill ${bill}",
                    era.era_name
                );
            }
        }
    }
}

/// The weekly sales pass runs after EVERY action, but instant actions
/// (marketing, lifestyle, re-press, deal responses) don't advance the
/// calendar — a second pass in the same week must be a no-op, or every
/// instant action mints a bonus catalog-tail week: free copies, income,
/// recoupment paydown, and certification progress.
#[test]
fn the_sales_pass_resolves_at_most_once_per_week() {
    let mut game = test_game();
    game.band.fame = 50;
    give_regional_presence(&mut game, 80);

    let mut release = test_release(1, ReleaseType::Single);
    release.week_released = 1;
    release.initial_sales_score = 500; // a healthy tail
    release.copies_pressed = 0; // uncapped
    game.band.singles_released.push(release);

    game.week = 10;
    game.process_music_releases_and_marketing();
    let sold_after_first = game.band.singles_released[0].copies_sold;
    let income_after_first = game.band.singles_released[0].total_income_generated;
    assert!(sold_after_first > 0, "the tail moved copies on the first pass");

    // Same week again — the instant-action path. Nothing may move.
    game.process_music_releases_and_marketing();
    assert_eq!(
        game.band.singles_released[0].copies_sold, sold_after_first,
        "a second pass in the same week must not sell more copies"
    );
    assert_eq!(
        game.band.singles_released[0].total_income_generated, income_after_first,
        "a second pass in the same week must not mint more income"
    );

    // Next week the tail sells again.
    game.week = 11;
    game.process_music_releases_and_marketing();
    assert!(
        game.band.singles_released[0].copies_sold > sold_after_first,
        "the following week's pass still sells"
    );
}
