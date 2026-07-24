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
    let _ = game.apply_certification_awards(0, 1, "Test Release", 50_000);

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

    // Check if certification would be awarded. `u8::MAX` = no era cap: this
    // test isolates the pure copy-threshold transition (L-CERT's year gate is
    // exercised separately below).
    let award = Game::compute_certification_awards(&release, u8::MAX);
    assert!(
        award.is_none(),
        "No award should trigger if already at this level"
    );

    // Now increase copies to Gold threshold.
    release.copies_sold = 150_000;
    let award = Game::compute_certification_awards(&release, u8::MAX);
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
    let _ = game.apply_certification_awards(0, 1, "Test Release", 50_000);
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

    let _ = game.apply_certification_awards(1, 2, "Test Release", 150_000);
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

    let _ = game.apply_certification_awards(2, 3, "Test Release", 400_000);
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
    let lines = game.apply_certification_awards(0, 4, "Test Release", 800_000);
    assert_eq!(
        lines.len(),
        4,
        "one 🏆 line per level crossed: silver, gold, platinum, ×2 tier"
    );

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
fn incremental_multiplatinum_awards_one_tier_per_step() {
    // The common path: a record already at ×2 (level 4) creeps up to ×3
    // (level 5) on tail sales. Crossing one new tier must award platinum's
    // bumps exactly once — not `new_level - 3` (= 2) times, which would
    // re-bank the tier it already had.
    let mut game = test_game();
    let happiness = game.player.happiness;
    let commercial = game.band.reputation.commercial_success;

    let lines = game.apply_certification_awards(4, 5, "Test Release", 1_200_000);
    assert_eq!(
        lines.len(),
        1,
        "×2 → ×3 crosses one tier: one 🏆 line, not two"
    );

    assert_eq!(
        game.player.happiness,
        happiness + 12,
        "×2 → ×3 crosses one tier: one platinum happiness bump (+12), not two"
    );
    assert_eq!(
        game.band.reputation.commercial_success,
        commercial + 8,
        "×2 → ×3 crosses one tier: one platinum commercial bump (+8), not two"
    );
}

#[test]
fn happiness_and_reputation_cap_at_100() {
    let mut game = test_game();

    // Max out happiness and reputation
    game.player.happiness = 100;
    game.band.reputation.commercial_success = 100;

    // Apply a certification bump (should stay at 100, not overflow)
    let _ = game.apply_certification_awards(0, 3, "Test Release", 400_000);

    assert_eq!(game.player.happiness, 100, "Happiness should cap at 100");
    assert_eq!(
        game.band.reputation.commercial_success, 100,
        "Commercial_success should cap at 100"
    );
}

// ── L-CERT (design §I): certifications gate on the game year ────────────────
// The copy thresholds (50k/150k/400k) never move; only the awardable *tier*
// gates on real industry history — no scheme before 1973, no Platinum before
// 1976, no multi-platinum before 1984. The cap is applied at award-check time,
// so a record still selling into a later era can reach a tier once it exists.

#[test]
fn era_max_certification_level_gates_each_tier_to_its_year() {
    // Pre-scheme: nothing certifies at all.
    assert_eq!(Game::era_max_certification_level(1970), 0);
    assert_eq!(Game::era_max_certification_level(1972), 0);
    // Scheme opens (1973): Gold (2) is the ceiling, Platinum unavailable.
    assert_eq!(Game::era_max_certification_level(1973), 2);
    assert_eq!(Game::era_max_certification_level(1975), 2);
    // Platinum arrives (1976): Platinum (3) available, multi-platinum not.
    assert_eq!(Game::era_max_certification_level(1976), 3);
    assert_eq!(Game::era_max_certification_level(1983), 3);
    // Multi-platinum arrives (1984): every tier available.
    assert_eq!(Game::era_max_certification_level(1984), u8::MAX);
    assert_eq!(Game::era_max_certification_level(1993), u8::MAX);
}

#[test]
fn no_certification_before_1973_even_at_platinum_volume() {
    let mut release = test_release(1, ReleaseType::Album);
    release.copies_sold = 400_000; // Platinum volume unrestricted
    for year in [1970u32, 1972] {
        let era_max = Game::era_max_certification_level(year);
        assert_eq!(
            Game::compute_certification_awards(&release, era_max),
            None,
            "no certification scheme existed in {year}, so nothing certifies"
        );
    }
}

#[test]
fn no_platinum_before_1976_caps_at_gold() {
    let mut release = test_release(1, ReleaseType::Album);
    release.copies_sold = 400_000; // would be Platinum (level 3) unrestricted
    let era_max = Game::era_max_certification_level(1974); // Gold era
    assert_eq!(
        Game::compute_certification_awards(&release, era_max),
        Some((0, 2)),
        "a Platinum-volume record in 1974 tops out at Gold (level 2)"
    );
}

#[test]
fn no_multiplatinum_before_1984_caps_at_platinum() {
    let mut release = test_release(1, ReleaseType::Album);
    release.copies_sold = 800_000; // would be multi-platinum ×2 (level 4) unrestricted
    let era_max_1980 = Game::era_max_certification_level(1980); // Platinum era
    assert_eq!(
        Game::compute_certification_awards(&release, era_max_1980),
        Some((0, 3)),
        "a multi-platinum-volume record in 1980 tops out at Platinum (level 3)"
    );
    // From 1984 the identical volume reaches multi-platinum ×2 (level 4).
    let era_max_1984 = Game::era_max_certification_level(1984);
    assert_eq!(
        Game::compute_certification_awards(&release, era_max_1984),
        Some((0, 4)),
        "from 1984 the same record certifies multi-platinum ×2 (level 4)"
    );
}

#[test]
fn crossing_a_threshold_in_a_valid_year_still_certifies_normally() {
    // Silver (50k) in 1975: the scheme exists and Silver sits well under the
    // Gold ceiling, so the gate leaves the normal award untouched.
    let mut release = test_release(1, ReleaseType::Single);
    release.copies_sold = 50_000;
    let silver_era = Game::era_max_certification_level(1975);
    assert_eq!(
        Game::compute_certification_awards(&release, silver_era),
        Some((0, 1)),
        "Silver certifies normally in 1975"
    );
    // Platinum (400k) in 1980 certifies to the full level 3.
    release.copies_sold = 400_000;
    let platinum_era = Game::era_max_certification_level(1980);
    assert_eq!(
        Game::compute_certification_awards(&release, platinum_era),
        Some((0, 3)),
        "Platinum certifies normally in 1980"
    );
}

#[test]
fn a_record_catches_up_to_a_tier_once_its_era_arrives() {
    // The mid-era re-cross. A record banked Gold under the pre-1976 cap while
    // already at Platinum volume. It does NOT re-award while still capped at
    // Gold, but once the year reaches the Platinum era the same copies_sold
    // crosses into Platinum — the gate is applied at award-check time, not
    // frozen at first sale.
    let mut release = test_release(1, ReleaseType::Album);
    release.copies_sold = 400_000;
    release.certified = 2; // Gold, banked under the 1973–1975 cap

    let gold_era = Game::era_max_certification_level(1974);
    assert_eq!(
        Game::compute_certification_awards(&release, gold_era),
        None,
        "no re-award while still capped at the tier it already holds"
    );

    let platinum_era = Game::era_max_certification_level(1977);
    assert_eq!(
        Game::compute_certification_awards(&release, platinum_era),
        Some((2, 3)),
        "the record catches up to Platinum once the era allows it"
    );
}

#[test]
fn sales_pass_awards_no_certification_in_1970() {
    // End-to-end proof that the gate is wired into the sales pass, not only the
    // pure helper: a runaway first-run hit that clears the Silver copy threshold
    // certifies nothing in 1970. Mirrors the setup of
    // `recoupment::certification_triggers_a_label_auto_repress`, which runs the
    // same hit in a certification-valid year and DOES certify.
    let mut game = test_game();
    assert_eq!(
        game.timeline.get_current_year(),
        1970,
        "a fresh game starts before any certification scheme"
    );
    game.band.fame = 60;
    give_regional_presence(&mut game, 80);
    game.band.record_deal = Some(test_deal(70, 0.12));

    let genre = genre::MusicGenre::Rock;
    // Monster era-independent modifier so first-run demand blows past Silver in
    // a single pass (certification normally lives on the catalog tail).
    game.world
        .dynamic_genre_modifiers
        .insert(genre.clone(), 100.0);

    let mut release = test_release(1, ReleaseType::Single);
    release.genre = Some(genre);
    release.release_quality = 90;
    release.week_released = 0;
    release.copies_pressed = 0; // uncapped
    game.just_released_music.push(release);

    game.week = INITIAL_SALES_WINDOW_WEEKS;
    game.process_music_releases_and_marketing();

    let release = &game.band.singles_released[0];
    assert!(
        release.copies_sold >= CERT_SILVER_THRESHOLD,
        "the runaway hit did clear the Silver copy threshold ({} copies)",
        release.copies_sold
    );
    assert_eq!(
        release.certified, 0,
        "but nothing certifies in 1970 — the scheme did not yet exist"
    );
    assert!(
        !game.turn_log.iter().any(|m| m.contains("🏆")),
        "no certification news line fires before the scheme existed"
    );
}
