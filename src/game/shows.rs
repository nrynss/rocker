//! The per-show engine (docs/DESIGN-v0.6-life-cycle.md §B): every concert —
//! one-off gig or tour stop — resolves individually with a reception roll,
//! a verdict, and a momentum multiplier that carries word-of-mouth across a
//! tour. `actions/live.rs` wires this into box office and stat effects;
//! this module is the math and the report types.

use rand::Rng;
use serde::{Deserialize, Serialize};

use super::band::Band;
use super::constants::{self, *};

/// How the crowd took it, from a reception score (§B — Verdicts).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum ShowVerdict {
    Rough,
    Solid,
    Great,
    Transcendent,
}

impl ShowVerdict {
    /// Verdict boundaries: < 40 rough · 40-69 solid · 70-84 great · ≥ 85
    /// transcendent (§B).
    pub(super) fn from_reception(reception: u8) -> Self {
        if reception >= VERDICT_TRANSCENDENT_MIN {
            ShowVerdict::Transcendent
        } else if reception >= VERDICT_GREAT_MIN {
            ShowVerdict::Great
        } else if reception >= VERDICT_SOLID_MIN {
            ShowVerdict::Solid
        } else {
            ShowVerdict::Rough
        }
    }

    /// The human word for the log and the report row.
    pub(super) fn label(self) -> &'static str {
        match self {
            ShowVerdict::Rough => "rough night",
            ShowVerdict::Solid => "solid",
            ShowVerdict::Great => "great",
            ShowVerdict::Transcendent => "transcendent",
        }
    }

    /// The momentum delta this verdict applies, before the 0.85-1.15 clamp
    /// (§B — Momentum). [tune]
    pub(super) fn momentum_delta(self) -> f32 {
        match self {
            ShowVerdict::Transcendent => MOMENTUM_DELTA_TRANSCENDENT,
            ShowVerdict::Great => MOMENTUM_DELTA_GREAT,
            ShowVerdict::Solid => MOMENTUM_DELTA_SOLID,
            ShowVerdict::Rough => MOMENTUM_DELTA_ROUGH,
        }
    }
}

/// The era-genre modifier (typically ~0.3-2.0, see `era_genre_modifier`)
/// scaled to a ±10 swing on reception: 1.0 (no opinion) maps to 0, the
/// "hot"/"cold" news-swing boundaries (`GENRE_TREND_HOT`/`COLD`) map to the
/// full ±10, and anything further out just clamps there (§B — Reception).
pub(super) fn era_fit_scaled(modifier: f32) -> f32 {
    let scale = ERA_FIT_MAX_SWING / (GENRE_TREND_HOT - 1.0);
    ((modifier - 1.0) * scale).clamp(-ERA_FIT_MAX_SWING, ERA_FIT_MAX_SWING)
}

/// Reception's effect on attendance: a modest factor centered on 1.0 at a
/// "solid" reception of 50 (§B — Box office). [tune]
pub(super) fn reception_attendance_factor(reception: u8) -> f32 {
    let t = reception as f32 / 100.0;
    RECEPTION_ATTENDANCE_MIN_FACTOR
        + t * (RECEPTION_ATTENDANCE_MAX_FACTOR - RECEPTION_ATTENDANCE_MIN_FACTOR)
}

/// Roll one show's reception (§B):
/// `band_base + condition + era_fit + variance + creativity_upside`,
/// clamped to 0-100. `band_base` is the dominant term by design — a tight,
/// uninspired band (0 creativity) can still be exceptional; creativity only
/// ever widens the upside tail, never multiplies the base.
pub(super) fn compute_reception(
    band: &Band,
    stress: u8,
    health: u8,
    era_genre_modifier: f32,
    creativity: u8,
    rng: &mut impl Rng,
) -> u8 {
    let band_base = constants::RECEPTION_BAND_BASE_SKILL_WEIGHT
        * band.average_member_skill() as f32
        + constants::RECEPTION_BAND_BASE_REPUTATION_WEIGHT
            * band.reputation.live_performance as f32;

    let mut condition = 0.0f32;
    if stress > RECEPTION_STRESS_THRESHOLD {
        condition -= RECEPTION_STRESS_PENALTY;
    }
    if health < RECEPTION_HEALTH_THRESHOLD {
        condition -= RECEPTION_HEALTH_PENALTY;
    }

    let era_fit = era_fit_scaled(era_genre_modifier);

    let variance = rng.gen_range(-RECEPTION_VARIANCE_RANGE..=RECEPTION_VARIANCE_RANGE) as f32;

    let upside_max = (creativity / RECEPTION_CREATIVITY_UPSIDE_DIVISOR) as i32;
    let creativity_upside = rng.gen_range(0..=upside_max) as f32;

    let raw = band_base + condition + era_fit + variance + creativity_upside;
    raw.round().clamp(0.0, 100.0) as u8
}

/// Apply a verdict's momentum delta, then clamp to 0.85-1.15 (§B —
/// Momentum). A hot streak sells the back half of a tour; a mid-tour
/// disaster deflates it.
pub(super) fn apply_momentum_delta(momentum: f32, verdict: ShowVerdict) -> f32 {
    (momentum + verdict.momentum_delta()).clamp(MOMENTUM_MIN, MOMENTUM_MAX)
}

/// One resolved show — a row in the tour report (§B — The tour report).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ShowReport {
    pub week: u32,
    pub venue_name: String,
    pub verdict: String,
    pub reception: u8,
    pub attendance: u32,
    pub capacity: u32,
    pub take: u32,
}

/// A resolved tour (or a one-off gig, stored as a single-row report) —
/// `Game::last_tour_report` (§B — The tour report).
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct TourReport {
    pub rows: Vec<ShowReport>,
    pub avg_reception: u8,
    pub total_gross: u32,
    pub fame_gained: u8,
}

impl TourReport {
    /// Build a report from resolved rows, computing the summary fields.
    pub(super) fn from_rows(rows: Vec<ShowReport>, fame_gained: u8) -> Self {
        let total_gross: u32 = rows.iter().map(|row| row.take).sum();
        let avg_reception = if rows.is_empty() {
            0
        } else {
            let sum: u32 = rows.iter().map(|row| row.reception as u32).sum();
            (sum / rows.len() as u32) as u8
        };
        Self {
            rows,
            avg_reception,
            total_gross,
            fame_gained,
        }
    }

    /// Tour verdict (§B): average reception ≥ 70 → "the tour went very well".
    pub fn went_very_well(&self) -> bool {
        self.avg_reception >= TOUR_WENT_WELL_RECEPTION_THRESHOLD
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn band_with(skill: u8, live_performance: u8) -> Band {
        let mut band = Band {
            skill,
            ..Band::default()
        };
        for member in &mut band.members {
            member.skill = skill;
        }
        band.reputation.live_performance = live_performance;
        band
    }

    #[test]
    fn verdict_boundaries_match_the_design_table() {
        assert_eq!(ShowVerdict::from_reception(0), ShowVerdict::Rough);
        assert_eq!(ShowVerdict::from_reception(39), ShowVerdict::Rough);
        assert_eq!(ShowVerdict::from_reception(40), ShowVerdict::Solid);
        assert_eq!(ShowVerdict::from_reception(69), ShowVerdict::Solid);
        assert_eq!(ShowVerdict::from_reception(70), ShowVerdict::Great);
        assert_eq!(ShowVerdict::from_reception(84), ShowVerdict::Great);
        assert_eq!(ShowVerdict::from_reception(85), ShowVerdict::Transcendent);
        assert_eq!(ShowVerdict::from_reception(100), ShowVerdict::Transcendent);
    }

    #[test]
    fn momentum_clamps_at_the_ceiling_and_floor() {
        let mut momentum = MOMENTUM_START;
        for _ in 0..20 {
            momentum = apply_momentum_delta(momentum, ShowVerdict::Transcendent);
        }
        assert!((momentum - MOMENTUM_MAX).abs() < f32::EPSILON);

        let mut momentum = MOMENTUM_START;
        for _ in 0..20 {
            momentum = apply_momentum_delta(momentum, ShowVerdict::Rough);
        }
        assert!((momentum - MOMENTUM_MIN).abs() < f32::EPSILON);
    }

    #[test]
    fn momentum_delta_applies_then_clamps() {
        let after_great = apply_momentum_delta(1.0, ShowVerdict::Great);
        assert!((after_great - 1.03).abs() < 1e-6);
        let after_rough = apply_momentum_delta(1.0, ShowVerdict::Rough);
        assert!((after_rough - 0.95).abs() < 1e-6);
        let after_solid = apply_momentum_delta(1.0, ShowVerdict::Solid);
        assert!((after_solid - 1.0).abs() < 1e-6);
    }

    #[test]
    fn era_fit_scales_to_plus_minus_ten_and_zeroes_at_neutral() {
        assert_eq!(era_fit_scaled(1.0), 0.0);
        assert!((era_fit_scaled(GENRE_TREND_HOT) - ERA_FIT_MAX_SWING).abs() < 1e-4);
        assert!((era_fit_scaled(GENRE_TREND_COLD) - (-ERA_FIT_MAX_SWING)).abs() < 1e-4);
        // Far outside the hot/cold envelope still clamps to ±10.
        assert_eq!(era_fit_scaled(2.0), ERA_FIT_MAX_SWING);
        assert_eq!(era_fit_scaled(0.3), -ERA_FIT_MAX_SWING);
    }

    #[test]
    fn reception_attendance_factor_centers_on_one_at_fifty() {
        assert!((reception_attendance_factor(50) - 1.0).abs() < 0.01);
        assert!((reception_attendance_factor(0) - RECEPTION_ATTENDANCE_MIN_FACTOR).abs() < 1e-6);
        assert!((reception_attendance_factor(100) - RECEPTION_ATTENDANCE_MAX_FACTOR).abs() < 1e-6);
    }

    #[test]
    fn a_perfect_band_with_zero_creativity_can_still_be_exceptional() {
        // 100% skill, full reputation, 0 creativity: band_base alone is
        // 0.7*100 + 0.3*100 = 100. Even the worst variance roll (-10) still
        // clears the transcendent bar (85), proving creativity is never a
        // multiplier gating the top verdict.
        use rand::rngs::mock::StepRng;
        let band = band_with(100, 100);
        // StepRng always returns the same word, driving gen_range to its
        // minimum on both the variance and creativity-upside rolls.
        let mut rng = StepRng::new(0, 0);
        let reception = compute_reception(&band, 0, 100, 1.0, 0, &mut rng);
        assert_eq!(
            reception, 90,
            "band_base 100 + worst-case variance -10 should still land at 90"
        );
        assert_eq!(
            ShowVerdict::from_reception(reception),
            ShowVerdict::Transcendent
        );
    }

    #[test]
    fn creativity_upside_is_zero_safe_at_zero_creativity() {
        // A real, varying RNG across many seeds: should never panic (an
        // empty range would) on creativity 0 -> gen_range(0..=0). A fixed
        // mock isn't safe here — `gen_range` rejection-samples, and a
        // non-varying source can reject forever; a real PRNG's output
        // changes every draw, so this always resolves promptly.
        use rand::SeedableRng;
        use rand::rngs::StdRng;
        let band = band_with(50, 50);
        for seed in 0..50u64 {
            let mut rng = StdRng::seed_from_u64(seed);
            let _ = compute_reception(&band, 0, 100, 1.0, 0, &mut rng);
        }
    }

    #[test]
    fn condition_penalties_stack_when_both_apply() {
        use rand::rngs::mock::StepRng;
        let band = band_with(50, 50);
        // Midpoint rng: near-zero variance and creativity upside contribution.
        let mut rng_ok = StepRng::new(0, 0);
        let healthy = compute_reception(&band, 0, 100, 1.0, 0, &mut rng_ok);

        let mut rng_bad = StepRng::new(0, 0);
        let stressed_and_sick = compute_reception(&band, 80, 10, 1.0, 0, &mut rng_bad);

        assert_eq!(
            healthy.saturating_sub(stressed_and_sick),
            20,
            "both condition penalties should stack to -20"
        );
    }
}
