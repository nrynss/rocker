//! Indie re-press + indie distribution tiers (design §E-1 indie half, §E-3,
//! M6): a sold-out (or low-stock) release can be topped up out of pocket
//! while unsigned, and an unsigned release buys reach through a purchasable
//! distribution channel.

use crate::game::music::{DistributionChannel, ReleaseType};

use super::*;

// --- §E-1 indie half: RePress ------------------------------------------

/// A near-sold-out release can be topped up: `copies_pressed` grows by the
/// chosen tier's run, and the player pays exactly that tier's cost.
#[test]
fn re_press_bumps_copies_pressed_and_charges_the_tier_cost() {
    let mut game = test_game();
    game.band.record_deal = None;
    game.player.money = 100_000;

    let mut release = test_release(1, ReleaseType::Single);
    release.copies_pressed = 1_000;
    release.copies_sold = 950; // 95% sold — past the low-stock threshold
    game.band.singles_released.push(release);

    let money_before = game.player.money;
    let tier = 1usize;
    let expected_run = PRESSING_TIERS[tier].1;
    let expected_cost = game.pressing_cost(&ReleaseType::Single, expected_run);

    game.action_re_press(1, Some(tier))
        .expect("a near-sold-out release is eligible for a re-press");

    let release = &game.band.singles_released[0];
    assert_eq!(
        release.copies_pressed,
        1_000 + expected_run,
        "the fresh run adds to the existing pressing"
    );
    assert_eq!(
        game.player.money,
        money_before - expected_cost,
        "the player pays exactly the chosen tier's cost"
    );
}

/// A signed act's label restocks on its own (M5, `label_auto_repress`) — the
/// player never hand-presses what the label already covers.
#[test]
fn re_press_is_unavailable_for_a_signed_act() {
    let mut game = test_game();
    game.band.record_deal = Some(test_deal(70, 0.12));

    let mut release = test_release(1, ReleaseType::Single);
    release.copies_pressed = 1_000;
    release.copies_sold = 1_000;
    game.band.singles_released.push(release);

    let err = game
        .action_re_press(1, Some(0))
        .expect_err("a signed act cannot hand re-press");
    assert!(
        err.contains("restocks"),
        "the rejection should point at the label, got: {err}"
    );
    assert_eq!(
        game.band.singles_released[0].copies_pressed, 1_000,
        "a rejected re-press changes nothing"
    );
}

/// A release that's still well-stocked isn't a candidate — and neither is a
/// legacy release with `copies_pressed == 0` (uncapped, pre-0.6).
#[test]
fn repressable_releases_only_lists_sold_out_or_low_stock() {
    let mut game = test_game();
    game.band.record_deal = None;

    let mut well_stocked = test_release(1, ReleaseType::Single);
    well_stocked.copies_pressed = 1_000;
    well_stocked.copies_sold = 100;
    game.band.singles_released.push(well_stocked);

    let mut low_stock = test_release(2, ReleaseType::Single);
    low_stock.copies_pressed = 1_000;
    low_stock.copies_sold = 950;
    game.band.singles_released.push(low_stock);

    let mut legacy_uncapped = test_release(3, ReleaseType::Single);
    legacy_uncapped.copies_pressed = 0;
    legacy_uncapped.copies_sold = 50_000;
    game.band.singles_released.push(legacy_uncapped);

    let ids: Vec<u32> = game
        .repressable_releases()
        .iter()
        .map(|release| release.id)
        .collect();
    assert_eq!(
        ids,
        vec![2],
        "only the near-sold-out release qualifies for a re-press"
    );
}

/// Signed acts never see a re-press list at all — the label's job, not
/// theirs (M5, design §E-1).
#[test]
fn repressable_releases_is_empty_while_signed() {
    let mut game = test_game();
    game.band.record_deal = Some(test_deal(70, 0.12));

    let mut release = test_release(1, ReleaseType::Single);
    release.copies_pressed = 1_000;
    release.copies_sold = 1_000;
    game.band.singles_released.push(release);

    assert!(game.repressable_releases().is_empty());
}

// --- §E-3: indie distribution tiers -------------------------------------

/// Each channel's reach floor wins over the fame formula at low fame — a
/// purchased channel buys reach a nobody's fame alone could never reach.
#[test]
fn each_distribution_channel_applies_its_reach_floor() {
    let mut game = test_game();
    game.band.record_deal = None;
    game.band.fame = 5; // fame formula alone: 0.15 + 0.05 * 0.85 ≈ 0.19

    let mut release = test_release(1, ReleaseType::Single);
    release.copies_pressed = 0; // uncapped, so outcome isn't stock-limited

    release.distribution_channel = Some(DistributionChannel::MailOrder);
    let (_, mail_units, _) = game.calculate_release_outcome(1_000, &release);

    release.distribution_channel = Some(DistributionChannel::Regional);
    let (_, regional_units, _) = game.calculate_release_outcome(1_000, &release);

    release.distribution_channel = Some(DistributionChannel::National);
    let (_, national_units, _) = game.calculate_release_outcome(1_000, &release);

    assert!(
        regional_units > mail_units,
        "Regional's 0.30 floor should beat Mail order's 0.15 at low fame: {regional_units} vs {mail_units}"
    );
    assert!(
        national_units > regional_units,
        "National's 0.50 floor should beat Regional's 0.30: {national_units} vs {regional_units}"
    );
}

/// The National tier is fame-gated (design §E-3 table: 35 fame); Mail order
/// and Regional are not.
#[test]
fn national_distribution_tier_is_fame_gated() {
    let mut game = test_game();
    game.band.record_deal = None;
    game.band.fame = 34;

    assert!(
        game.plan_distribution(DistributionChannel::MailOrder)
            .is_ok(),
        "Mail order is ungated"
    );
    assert!(
        game.plan_distribution(DistributionChannel::Regional)
            .is_ok(),
        "Regional is ungated"
    );
    assert!(
        game.plan_distribution(DistributionChannel::National)
            .is_err(),
        "National needs 35 fame — 34 should be refused"
    );

    game.band.fame = 35;
    assert!(
        game.plan_distribution(DistributionChannel::National)
            .is_ok(),
        "National opens up right at the gate"
    );
}

/// A label deal ignores channels entirely: no fee, no gate, regardless of
/// what's "currently selected".
#[test]
fn signed_acts_never_pay_a_distribution_fee() {
    let mut game = test_game();
    game.band.fame = 0; // would fail every gate if it mattered
    game.band.record_deal = Some(test_deal(70, 0.12));

    assert_eq!(
        game.plan_distribution(DistributionChannel::National),
        Ok(0),
        "signed acts pay nothing — market_reach does the work"
    );
}

/// Recording under a paid channel charges its fee alongside studio/pressing
/// costs, and stamps the channel onto the new release (frozen per-release,
/// not a retroactive global setting).
#[test]
fn distribution_fee_is_charged_at_release_and_stamped_on_the_release() {
    use rand::SeedableRng;
    use rand::rngs::StdRng;

    let mut game = test_game();
    game.band.record_deal = None;
    game.band.fame = 50; // clears every gate
    game.band.unreleased_songs.push(music::Song {
        id: 0,
        name: "Track".to_string(),
        songwriting_quality: 50,
    });
    game.current_distribution_channel = DistributionChannel::Regional;
    game.player.money = 100_000;

    let recording_cost = game.recording_cost(&ReleaseType::Single);
    let (_, pressing_cost) = game
        .plan_pressing(&ReleaseType::Single, Some(0))
        .expect("tier 0 exists");
    let fee = DistributionChannel::Regional.fee();
    assert!(fee > 0, "Regional carries a real fee");
    let money_before = game.player.money;

    let mut rng = StdRng::seed_from_u64(0);
    game.action_record_single(Some(0), &mut rng)
        .expect("affordable recording");

    assert_eq!(
        game.player.money,
        money_before - (recording_cost + pressing_cost + fee),
        "the distribution fee is charged alongside studio and pressing costs"
    );
    let release = game
        .just_released_music
        .last()
        .expect("the single was recorded");
    assert_eq!(
        release.distribution_channel,
        Some(DistributionChannel::Regional),
        "the release remembers the channel it went out under"
    );
}

/// A signed release never carries a channel — reach is `market_reach`,
/// channel-blind (design §E-3).
#[test]
fn signed_releases_never_stamp_a_distribution_channel() {
    use rand::SeedableRng;
    use rand::rngs::StdRng;

    let mut game = test_game();
    game.band.fame = 50;
    game.band.record_deal = Some(test_deal(70, 0.12));
    game.band.unreleased_songs.push(music::Song {
        id: 0,
        name: "Track".to_string(),
        songwriting_quality: 50,
    });
    game.player.money = 100_000;

    let mut rng = StdRng::seed_from_u64(0);
    game.action_record_single(None, &mut rng)
        .expect("the label covers a signed release");

    let release = game
        .just_released_music
        .last()
        .expect("the single was recorded");
    assert_eq!(release.distribution_channel, None);
}

/// An old save predating M6 has no channel on its releases and no chosen
/// channel on the game — both default sanely (Mail order & gigs), so a
/// legacy release's reach reads exactly as it always did.
#[test]
fn old_save_defaults_distribution_fields_sanely() {
    let release = test_release(1, ReleaseType::Single);
    assert_eq!(release.distribution_channel, None);

    let mut saved = serde_json::to_value(test_game()).expect("games serialize");
    saved
        .as_object_mut()
        .expect("a game serializes to an object")
        .remove("current_distribution_channel");
    let loaded: Game = serde_json::from_value(saved).expect("old saves must keep loading");
    assert_eq!(
        loaded.current_distribution_channel,
        DistributionChannel::MailOrder
    );
}
