//! Tests for record certifications (design §D).

use super::*;

#[test]
fn old_save_defaults_to_uncertified() {
    // When a release doesn't have a certified field (old save), it should default to 0.
    let release = test_release(1, ReleaseType::Album);
    assert_eq!(release.certified, 0, "Default certified should be 0");
}

#[test]
fn crossing_silver_threshold_awards_certification() {
    let mut game = test_game();
    game.band.fame = 50; // Set some fame so we get interesting sales

    // Create and push a test single with low quality/marketing to keep sales predictable.
    let mut release = test_release(1, ReleaseType::Single);
    release.release_quality = 80;
    release.week_released = 1;
    release.initial_sales_score = 5000; // High enough to generate sales

    // To sell 50k copies, we need the calculation:
    // demand = sales_score * distribution * UNITS_PER_SCORE_POINT
    // We'll set copies_sold directly to simulate this.
    release.copies_sold = 50_000;
    game.band.singles_released.push(release);

    let initial_happiness = game.player.happiness;
    let initial_commercial = game.band.reputation.commercial_success;

    // Verify the certification level computation.
    let cert_level = Game::compute_certification_level(50_000);
    assert_eq!(cert_level, 1, "50,000 copies should be SILVER (level 1)");

    // Manually apply the awards to verify the bumps.
    game.apply_certification_awards(0, 1, "Test Release", 50_000);

    // Check the bumps were applied.
    assert_eq!(
        game.player.happiness,
        initial_happiness + 5,
        "Silver should add 5 happiness"
    );
    assert_eq!(
        game.band.reputation.commercial_success,
        initial_commercial + 3,
        "Silver should add 3 commercial_success"
    );
}

#[test]
fn crossing_gold_threshold_awards_certification() {
    let cert_level = Game::compute_certification_level(150_000);
    assert_eq!(cert_level, 2, "150,000 copies should be GOLD (level 2)");
}

#[test]
fn crossing_platinum_threshold_awards_certification() {
    let cert_level = Game::compute_certification_level(400_000);
    assert_eq!(cert_level, 3, "400,000 copies should be PLATINUM (level 3)");
}

#[test]
fn multiplatinum_scales_correctly() {
    // Multi-platinum: 3 + additional 400k tiers
    // 800k = 400k (platinum) + 400k (×2) = certified level 4
    let level_800k = Game::compute_certification_level(800_000);
    assert_eq!(
        level_800k, 4,
        "800,000 should be multi-platinum ×2 (level 4)"
    );

    // 1.2M = 400k + 400k + 400k = ×3
    let level_1200k = Game::compute_certification_level(1_200_000);
    assert_eq!(
        level_1200k, 5,
        "1,200,000 should be multi-platinum ×3 (level 5)"
    );
}

#[test]
fn no_double_award_when_copies_stay_flat() {
    let _game = test_game();

    // Create a release already certified at Silver.
    let mut release = test_release(1, ReleaseType::Single);
    release.copies_sold = 50_000;
    release.certified = 1; // Already Silver

    // Check if certification would be awarded.
    let award = Game::compute_certification_awards(&release);
    assert!(
        award.is_none(),
        "No award should trigger if already at this level"
    );

    // Now increase copies to Gold threshold.
    release.copies_sold = 150_000;
    let award = Game::compute_certification_awards(&release);
    assert_eq!(
        award,
        Some((1, 2)),
        "Should award Gold transition from Silver to Gold"
    );
}

#[test]
fn certification_bumps_are_correct_magnitude() {
    let mut game = test_game();
    let initial_happiness = game.player.happiness;
    let initial_commercial = game.band.reputation.commercial_success;

    // Apply silver (level 0→1) awards
    game.apply_certification_awards(0, 1, "Test Release", 50_000);
    assert_eq!(
        game.player.happiness,
        initial_happiness + 5,
        "Silver happiness bump should be +5"
    );
    assert_eq!(
        game.band.reputation.commercial_success,
        initial_commercial + 3,
        "Silver commercial_success bump should be +3"
    );

    // Test gold transition (level 1→2, so only gold bump applies)
    let happiness_after_silver = game.player.happiness;
    let commercial_after_silver = game.band.reputation.commercial_success;

    game.apply_certification_awards(1, 2, "Test Release", 150_000);
    assert_eq!(
        game.player.happiness,
        happiness_after_silver + 8,
        "Gold happiness bump should be +8"
    );
    assert_eq!(
        game.band.reputation.commercial_success,
        commercial_after_silver + 5,
        "Gold commercial_success bump should be +5"
    );

    // Test platinum transition (level 2→3, so only platinum bump applies)
    let happiness_after_gold = game.player.happiness;
    let commercial_after_gold = game.band.reputation.commercial_success;

    game.apply_certification_awards(2, 3, "Test Release", 400_000);
    assert_eq!(
        game.player.happiness,
        happiness_after_gold + 12,
        "Platinum happiness bump should be +12"
    );
    assert_eq!(
        game.band.reputation.commercial_success,
        commercial_after_gold + 8,
        "Platinum commercial_success bump should be +8"
    );
}

#[test]
fn multiplatinum_repeats_platinum_bumps() {
    let mut game = test_game();
    let initial_happiness = game.player.happiness;
    let initial_commercial = game.band.reputation.commercial_success;

    // Apply platinum and then multi-platinum (×2 at 800k).
    // Levels 0 → 4 should include: silver (1), gold (2), platinum (3), then multi-platinum tier.
    game.apply_certification_awards(0, 4, "Test Release", 800_000);

    // Silver: +5, Gold: +8, Platinum: +12, Multi-platinum ×1 tier: +12
    // Total: 5 + 8 + 12 + 12 = 37
    assert_eq!(
        game.player.happiness,
        initial_happiness + 37,
        "Uncertified → multi-platinum ×2 (level 4) should give 5 + 8 + 12 + 12 = 37 happiness"
    );
    assert_eq!(
        game.band.reputation.commercial_success,
        initial_commercial + 24,
        "Uncertified → multi-platinum ×2 (level 4) should give 3 + 5 + 8 + 8 = 24 commercial_success"
    );
}

#[test]
fn happiness_and_reputation_cap_at_100() {
    let mut game = test_game();

    // Max out happiness and reputation
    game.player.happiness = 100;
    game.band.reputation.commercial_success = 100;

    // Apply a certification bump (should stay at 100, not overflow)
    game.apply_certification_awards(0, 3, "Test Release", 400_000);

    assert_eq!(game.player.happiness, 100, "Happiness should cap at 100");
    assert_eq!(
        game.band.reputation.commercial_success, 100,
        "Commercial_success should cap at 100"
    );
}
