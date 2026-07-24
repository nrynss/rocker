//! Tuning knobs and balance constants for the game simulation.
//!
//! Gameplay dials and RNG stream salts live here (formerly the top of
//! `game/mod.rs`). Data-level caps and costs (`MAX_HEALTH`, `WEEKS_PER_YEAR`,
//! recording prices, win thresholds, …) are re-exported from
//! [`crate::data::constants`] so callers can use one module:
//!
//! ```ignore
//! use crate::game::constants::{self, *};
//! // bare: LIVE_FAME_BASE_CAP, PRESSING_TIERS
//! // path: constants::MAX_FAME, constants::ACTION_STREAM_SALT
//! ```

pub use crate::data::constants::*;

// Quality calculation constants
pub(super) const QUALITY_BASE_SONGWRITING: u8 = 30;
pub(super) const QUALITY_SONGWRITING_RANDOM_VARIATION: u8 = 10;
pub(super) const QUALITY_BASE_RECORDING: u8 = 30;
pub(super) const QUALITY_RECORDING_RANDOM_VARIATION: u8 = 10;

// Sales model constants
pub(super) const INITIAL_SALES_WINDOW_WEEKS: u32 = 4;
pub(super) const SALES_QUALITY_WEIGHT: f32 = 2.5;
pub(super) const SALES_MARKETING_WEIGHT: f32 = 1.8;
pub(super) const SALES_FAME_WEIGHT: f32 = 1.2;

// Unit economics: a sales score converts into copies people want to buy,
// bounded by how many copies actually exist.
//
// M7 (§F): raised 10 → 30 so real careers move enough copies for the
// certification thresholds (§D: Silver 50k) to be reachable — at the old
// rate a hit sold only hundreds–low-thousands and nobody ever certified.
// Copies are a *count*, not a dollar, so L-SCALE (§B) left this at 30. It once
// coupled with `SALES_INCOME_DIVISOR` to hold per-release income flat while
// copies climbed to the 50k/150k/400k scale; L-SCALE has since retired that
// divisor (below), so copies now drive certification and — at the era-real
// per-copy royalty — income together.
pub(super) const UNITS_PER_SCORE_POINT: f32 = 30.0;
/// Divided record income after the `UNITS_PER_SCORE_POINT` bump so that raising
/// copies-sold for certification (§D) didn't inflate the compressed v0.6/v0.7
/// money economy (M7 §F).
///
/// L-SCALE (v0.8 §B) **retired** this — 3 → 1 — as the first step of the
/// one-honest-scale decompression: record income rises ×3 to meet the already
/// era-real copy counts, in lockstep with the uniform ×3 of every *other*
/// dollar in the game (recording/pressing/tour/gig/support/advance costs and
/// incomes), so ratios are preserved while magnitude triples. Retiring it
/// *alone* would triple record income while tour/gig income stayed flat and
/// skew the balance toward records — hence the whole-economy ×3.
///
/// Kept as a named constant (now `1`) rather than deleted so the
/// divisor-coupled `PRESSING_PER_COPY_*` literals below stay legible and
/// auto-track it (0.10/1, 0.50/1 — themselves ×3 vs the v0.7 values, the
/// pressing side of the same rescale).
pub(super) const SALES_INCOME_DIVISOR: u32 = 1;
pub(super) const INDIE_INCOME_PER_COPY: u32 = 2;
pub(super) const LABEL_INCOME_PER_COPY: u32 = 3;

// Living sales tail: post-launch decay and influence [tune].
//
// The decay step (3 → 4) and the tail unit rate (a magic `/ 5` in the tail
// loop → the named `/ 3` below) were retuned together when the weekly sales
// pass gained its once-per-week guard: instant actions had been re-running
// the tail at the same week's (early, high) rate without advancing the decay
// clock, and the §D/§F certification balance — median signed 15-year career
// reaches 1–3 silvers; a home-market act can reach Silver at all — was
// unknowingly calibrated on that bonus volume. These put the legitimate
// weekly tail where the exploit-inflated one effectively was: the sweep's
// label-loyalist lands med 3 certs (was 5 with the exploit, 0 without the
// retune). Re-check that sweep column before moving either. [tune]
pub(super) const TAIL_DIVISOR_WEEKS_PER_STEP: u32 = 4;
pub(super) const TAIL_MARKETING_WEIGHT: f32 = 1.8;
pub(super) const TAIL_FAME_WEIGHT: f32 = 0.3;
/// The tail moves this fraction (1/N) of the first-run per-week unit rate.
/// Retuned with `TAIL_DIVISOR_WEEKS_PER_STEP` above. [tune]
pub(super) const TAIL_UNITS_DIVISOR: u32 = 3;

// Pressing runs. Independents choose a run and pay setup plus per-copy
// costs; a label presses to the size of its network and your name. The
// National run (50k) is exactly the Silver certification mark (§D), so a
// self-pressed indie hit on the biggest run can just reach it.
//
// M7 (§F): the Garage run grew 500 → 1_500 (= 500 × SALES_INCOME_DIVISOR)
// so a garage-run act's capped income lands back where v0.6 had it after
// the income divisor — the larger tiers are unchanged (the determinism
// script presses a Club run and must stay affordable).
pub const PRESSING_TIERS: [(&str, u32); 4] = [
    ("Garage run", 1_500),
    ("Club run", 2_000),
    ("Regional run", 10_000),
    ("National run", 50_000),
];
// L-SCALE (§B): pressing setup ×3 with the rest of the economy (was 25 / 100).
pub(super) const PRESSING_SETUP_SINGLE: f32 = 75.0;
pub(super) const PRESSING_SETUP_ALBUM: f32 = 300.0;
// Per-copy pressing costs are COUPLED to the sales rescale, the same way as
// `LABEL_RECOUP_PRESSING_PER_COPY` below (issue #20's bug class): M7 tripled
// the copies a score moves (`UNITS_PER_SCORE_POINT`) while `SALES_INCOME_
// DIVISOR` kept income dollars flat, so an indie pressed 3× the copies for
// the same money and the per-copy bill had to divide by the same ratio.
// L-SCALE (§B) has now retired that divisor (set to 1), so these literals
// evaluate to $0.10/single and $0.50/album — themselves ×3 vs the v0.7
// values, exactly the per-copy dollar rescale L-SCALE applies everywhere
// else. The coupling is kept in the expression so the arithmetic stays
// legible and tracks the divisor if it ever moves again. [tune]
pub(super) const PRESSING_PER_COPY_SINGLE: f32 = 0.10 / SALES_INCOME_DIVISOR as f32;
pub(super) const PRESSING_PER_COPY_ALBUM: f32 = 0.50 / SALES_INCOME_DIVISOR as f32;
// Label pressing run sizes are COUPLED to the M7 sales rescale like the
// per-copy dollar constants above: `UNITS_PER_SCORE_POINT` tripled the
// copies a sales score demands, and a run is denominated in copies, so the
// run a label presses must triple with it (100/50 → 300/150). Left on the
// old scale, a signed act's first run nearly always sold out and the
// over-cap demand was silently discarded — a fame-100 act's hit leaked most
// of its first-week sales, which is why no signed career could accumulate
// the 50k copies Silver asks for (§D/§F) once the once-per-week sales-pass
// guard stopped instant actions from minting bonus tail weeks. If
// `UNITS_PER_SCORE_POINT` moves again, carry these along. [tune]
pub(super) const LABEL_PRESSING_PER_REACH: u32 = 300;
pub(super) const LABEL_PRESSING_PER_FAME: u32 = 150;

// Distribution model: how much of a release's potential audience you can
// actually reach. Labels bring their market_reach; independents are capped
// by their own fame.
pub(super) const INDIE_REACH_FLOOR: f32 = 0.15;

// ============================================================================
// Certifications (design §D): records certify off cumulative copies_sold.
// Thresholds scaled for the regional sales model (§C).
// ============================================================================

pub(super) const CERT_SILVER_THRESHOLD: u32 = 50_000;
pub(super) const CERT_GOLD_THRESHOLD: u32 = 150_000;
pub(super) const CERT_PLATINUM_THRESHOLD: u32 = 400_000;
pub(super) const CERT_MULTIPLATINUM_STEP: u32 = 400_000;

// Certification year gates (L-CERT, design §I). The copy thresholds above are
// era-agnostic; only the *availability* of each tier gates on the game year,
// so the 1970→1993 timeline stops handing out awards that did not yet exist:
//   - No sales-certification scheme existed before 1973 (the BPI's UK Silver/
//     Gold scheme — which this game's UK-scaled thresholds track — launched in
//     1973). Before then, nothing certifies.
//   - RIAA-style Platinum did not exist before 1976.
//   - Multi-platinum did not exist before 1984.
// Applied at award-check time (`era_max_certification_level`), so a record that
// keeps selling into a later era can still reach a tier once it becomes
// available — the cap is on the awardable level, never on `copies_sold`.
pub(super) const CERT_YEAR_SCHEME_START: u32 = 1973;
pub(super) const CERT_YEAR_PLATINUM: u32 = 1976;
pub(super) const CERT_YEAR_MULTIPLATINUM: u32 = 1984;

// Award bumps for each certification level (silver/gold/platinum; multi-platinum repeats platinum).
pub(super) const CERT_FAME_BUMP: &[u8] = &[2, 4, 6]; // Silver, Gold, Platinum
pub(super) const CERT_HAPPINESS_BUMP: &[u8] = &[5, 8, 12]; // Silver, Gold, Platinum
pub(super) const CERT_COMMERCIAL_SUCCESS_BUMP: &[u8] = &[3, 5, 8]; // Silver, Gold, Platinum

// Support tours: bigger acts occasionally want you as their opener.
pub(super) const SUPPORT_OFFER_MIN_FAME: u8 = 5;
pub(super) const SUPPORT_OFFER_FAME_GAP: u8 = 10;
pub(super) const SUPPORT_OFFER_CHANCE: f64 = 0.06;
pub(super) const SUPPORT_OFFER_LIFETIME_WEEKS: u32 = 3;

// Record deals stay on the table about a month — one scouting cycle — so
// a slate the player sits on clears just as labels next come looking, and
// ignoring an offer can never silence the deal stream for good.
pub(super) const DEAL_OFFER_LIFETIME_WEEKS: u32 = 4;

pub(super) const PLAYER_MARKET_IMPACT_THRESHOLD_SALES_SCORE: u32 = 600;
pub(super) const PLAYER_MARKET_IMPACT_GENRE_MOD_BONUS: f32 = 0.05;
pub(super) const PLAYER_MARKET_IMPACT_DEMAND_BONUS: u8 = 1;

// Live fame ceilings: a gig only reaches the crowd in the room, and without
// records word of mouth stalls. Gigs and tours raise fame no further than
// the smaller of the venue's ceiling and the catalog cap.
pub(super) const VENUE_FAME_HEADROOM: u8 = 15;
pub(super) const LIVE_FAME_BASE_CAP: u8 = 35;
pub(super) const LIVE_FAME_PER_SINGLE: u8 = 6;
pub(super) const LIVE_FAME_PER_ALBUM: u8 = 12;

// A break clears the calendar for this many weeks (a full reset action).
pub const BREAK_WEEKS: u32 = 4;

// ============================================================================
// Fame gravity (v0.6 design §C — fully decided; only the comeback multiplier
// is [tune]). Fame is earned slowly and defended by staying in the picture;
// left alone it drifts back down toward a floor the band earned at its peak.
// ============================================================================

/// Comeback rule: while current fame is below the band's peak, every fame
/// *gain* is multiplied by this. Reclaiming ground is easier than the first
/// climb. Only this value is tune-able. (§C — Comeback)
pub(super) const FAME_COMEBACK_MULTIPLIER: u8 = 2;

/// The idle-decay ramp: −1 the first decay week, −2, −3, −4, then −5 every
/// week after — this is where it flattens out. (§C — The ramp)
pub(super) const FAME_RAMP_MAX_DECAY: u8 = 5;

/// Establishment rule: at or above this fame, an album/single released in the
/// recent window counts as staying in the picture. Below it, small acts must
/// keep physically showing up. (§C — Activity, rule 3)
pub(super) const ESTABLISHMENT_MIN_FAME: u8 = 60;
/// How recent a release must be to satisfy the establishment rule. (§C)
pub(super) const ESTABLISHMENT_RELEASE_WINDOW_WEEKS: u32 = 52;

/// The top floor (75) also requires this many *hits* — albums/singles that
/// charted at all (`peak_chart_position.is_some()`). (§C — Floors)
pub(super) const FAME_FLOOR_HITS_THRESHOLD: usize = 10;

/// Grace: consecutive quiet weeks before *any* decay begins, keyed by the
/// band's *current* fame. Each row is `(inclusive-upper-fame, grace weeks)`;
/// the first row whose bound the current fame falls under wins. (§C — Grace)
pub(super) const FAME_GRACE_TIERS: [(u8, u32); 7] = [
    (15, 2),       // 0–15   → 2 weeks
    (29, 4),       // 16–29  → 4 weeks
    (49, 8),       // 30–49  → 8 weeks
    (74, 13),      // 50–74  → 13 weeks (3 months)
    (89, 26),      // 75–89  → 26 weeks (6 months)
    (94, 39),      // 90–94  → 39 weeks (9 months)
    (u8::MAX, 52), // 95+    → 52 weeks (1 year)
];

/// Floors: fame never *decays* below these, keyed by the highest fame the band
/// ever reached (its peak). Each row is `(minimum-peak, floor)`, checked from
/// the top down so the highest matching peak wins. The 95+ row's 75 floor is
/// gated on `FAME_FLOOR_HITS_THRESHOLD` hits and applied in code. (§C — Floors)
pub(super) const FAME_FLOOR_TIERS: [(u8, u8); 7] = [
    (95, 70), // 95+ (→ 75 with ≥ 10 hits)
    (90, 60),
    (75, 45),
    (60, 30),
    (45, 15),
    (30, 10),
    (0, 0), // under 30
];
/// The elevated top floor once the 95+ peak is paired with enough hits. (§C)
pub(super) const FAME_FLOOR_LEGEND: u8 = 75;

// Era-genre fit: past these bounds the era clearly loves or has abandoned
// the band's sound, and the press says so — once per swing, not every week.
// (A genre missing from an era's table reads as out of fashion at 0.85.)
pub(super) const GENRE_TREND_HOT: f32 = 1.15;
pub(super) const GENRE_TREND_COLD: f32 = 0.85;

// ============================================================================
// Label single-cuts (v0.6 design §C — label releases a single on its own
// volition when signed, the band is quiet, and they have un-singled tracks).
// ============================================================================

/// Probability per week (0.0–1.0) that a label releases a single from an
/// eligible album. (§C — Label single-cuts) [tune]
pub(super) const LABEL_CUT_CHANCE: f64 = 0.10;

/// Weeks the band must be quiet (idle_streak) before the label cuts a single.
/// (§C — Label single-cuts) [tune]
pub(super) const LABEL_CUT_IDLE_WEEKS: u32 = 3;

/// Weeks since any release before the label is allowed to cut a single.
/// (§C — Label single-cuts) [tune]
pub(super) const LABEL_CUT_RELEASE_COOLDOWN_WEEKS: u32 = 6;

/// Maximum singles cut from any one album. (§C — Label single-cuts) [tune]
pub(super) const LABEL_CUT_MAX_PER_ALBUM: u32 = 2;

// --- L1: stat engine — the four bars (docs/DESIGN-v0.6-life-cycle.md §A) ---
// Weekly lifestyle tick (`lifestyle.rs`): stress bleeds off on its own,
// worse while broke; happiness and creativity sag once stress runs high;
// excessive lazing wears on health, but far slower than anything else.
pub(super) const STRESS_PASSIVE_RELEASE: u8 = 3;
pub(super) const BROKE_STRESS_PER_WEEK: u8 = 5;
pub(super) const HAPPINESS_STRESS_DIVISOR: u8 = 25;
pub(super) const CREATIVITY_STRESS_THRESHOLD: u8 = 40;
pub(super) const CREATIVITY_STRESS_DIVISOR: u8 = 20;
pub(super) const LAZE_WEAR_THRESHOLD_WEEKS: u32 = 4;

// Ceilings for the two new bars (health/stress already have MAX_HEALTH /
// MAX_STRESS from `data::constants`).
pub(super) const MAX_HAPPINESS: u8 = 100;
pub(super) const MAX_CREATIVITY: u8 = 100;

// Rest actions (`actions/rest.rs`): laze trades stress for a little
// creativity; a real break resets stress and gives happiness/creativity/
// health a proper boost.
pub(super) const LAZE_STRESS_RELIEF: u8 = 15;
pub(super) const LAZE_CREATIVITY_GAIN: u8 = 3;
pub(super) const BREAK_HAPPINESS_GAIN: u8 = 10;
pub(super) const BREAK_CREATIVITY_GAIN: u8 = 10;
pub(super) const BREAK_HEALTH_GAIN: u8 = 30;

// New-player defaults (`player.rs`) — the original 1989 game's starting
// mood/imagination, not maxed like health/energy.
pub(super) const DEFAULT_HAPPINESS: u8 = 60;
pub(super) const DEFAULT_CREATIVITY: u8 = 50;

// --- L2: writing & quality (docs/DESIGN-v0.6-life-cycle.md §A, Quality formulas) ---
// Writing and recording stress costs [tune], and the guard threshold.
pub(super) const WRITE_STRESS_COST: u8 = 5;
pub(super) const RECORD_STRESS_COST: u8 = 8;
// Rehearsal is light studio work: cheapest stress cost of the week jobs [tune].
pub(super) const PRACTICE_STRESS_COST: u8 = 4;
// `pub` (not `pub(super)`): the practice menu entry in `ui/app.rs` reads it.
pub const STUDIO_STRESS_BLOCK: u8 = 90;
// Recording quality penalty threshold [tune].
pub(super) const RECORDING_STRESS_PENALTY_THRESHOLD: u8 = 70;
// Writing streak: fatigue kicks in on the 3rd+ consecutive week [tune].
pub(super) const WRITING_STREAK_FATIGUE: u32 = 3;
pub(super) const WRITING_FATIGUE_CREATIVITY_COST: u8 = 5;
// Writing under stress drains creativity: threshold and divisor [tune].
pub(super) const WRITING_STRESS_CREATIVITY_THRESHOLD: u8 = 50;
pub(super) const WRITING_STRESS_CREATIVITY_DIVISOR: u8 = 5;
// Happiness multiplier for quality: 0.8 + happiness / 500.0 (clamped 0.8–1.0).
// Constants for the formula: min 0.8, divisor 500.
pub(super) const HAPPINESS_QUALITY_MULTIPLIER_MIN: f32 = 0.8;
pub(super) const HAPPINESS_QUALITY_MULTIPLIER_SCALE: f32 = 500.0;
// Creativity divisor for songwriting bonus: creativity / 4 yields 0–25 bonus at creativity 0–100.
pub(super) const SONGWRITING_CREATIVITY_DIVISOR: u8 = 4;
// Recording quality penalty when stress > RECORDING_STRESS_PENALTY_THRESHOLD [tune].
pub(super) const RECORDING_STRESS_PENALTY: i8 = 10;

// --- L8: data-driven incidents (docs/DESIGN-v0.6-life-cycle.md §F) ---
// Incidents are eligible every week (was every other week); this is the
// per-week chance one fires, rolled on the action stream in `events.rs`.
// [tune] — §F raises cadence from 30% every-other-week to 35% weekly.
pub(super) const INCIDENT_WEEKLY_CHANCE_PERCENT: u32 = 35;

// ============================================================================
// L3: per-show engine (docs/DESIGN-v0.6-life-cycle.md §B — `shows.rs`).
// Every gig/tour stop resolves individually: a reception roll, a verdict,
// and a momentum multiplier that carries word-of-mouth across a tour.
// ============================================================================

/// Five shows per tour week (decided design, not [tune]).
pub(super) const SHOWS_PER_TOUR_WEEK: u32 = 5;

// Reception = band_base + condition + era_fit + variance + creativity_upside,
// clamped 0-100 (§B). band_base is the dominant term by design.
pub(super) const RECEPTION_BAND_BASE_SKILL_WEIGHT: f32 = 0.7;
pub(super) const RECEPTION_BAND_BASE_REPUTATION_WEIGHT: f32 = 0.3;
pub(super) const RECEPTION_STRESS_THRESHOLD: u8 = 70;
pub(super) const RECEPTION_STRESS_PENALTY: f32 = 10.0;
pub(super) const RECEPTION_HEALTH_THRESHOLD: u8 = 40;
pub(super) const RECEPTION_HEALTH_PENALTY: f32 = 10.0;
/// Variance roll: inclusive -10..=10.
pub(super) const RECEPTION_VARIANCE_RANGE: i32 = 10;
/// Creativity upside roll: inclusive 0..=creativity/5 (0..=20 at max
/// creativity; exactly 0 at creativity 0 — never an empty range). Creativity
/// is upside only, never a multiplier on `band_base`.
pub(super) const RECEPTION_CREATIVITY_UPSIDE_DIVISOR: u8 = 5;

/// Era-genre fit scaled to a ±10 swing on reception. Reuses `GENRE_TREND_HOT`
/// as the anchor: a modifier at the "hot" boundary (1.15) or beyond scores
/// the full +10; a modifier at or past "cold" (0.85) scores -10; 1.0 → 0.
pub(super) const ERA_FIT_MAX_SWING: f32 = 10.0;

// Verdict boundaries (§B), inclusive lower bounds.
pub(super) const VERDICT_SOLID_MIN: u8 = 40;
pub(super) const VERDICT_GREAT_MIN: u8 = 70;
pub(super) const VERDICT_TRANSCENDENT_MIN: u8 = 85;

// Momentum: a rolling word-of-mouth multiplier across a tour, starts at 1.0,
// clamped after every show. [tune] deltas.
pub(super) const MOMENTUM_START: f32 = 1.0;
pub(super) const MOMENTUM_MIN: f32 = 0.85;
pub(super) const MOMENTUM_MAX: f32 = 1.15;
pub(super) const MOMENTUM_DELTA_TRANSCENDENT: f32 = 0.05;
pub(super) const MOMENTUM_DELTA_GREAT: f32 = 0.03;
pub(super) const MOMENTUM_DELTA_SOLID: f32 = 0.0;
pub(super) const MOMENTUM_DELTA_ROUGH: f32 = -0.05;

/// Reception's effect on attendance: a modest [tune] factor mapped linearly
/// from reception (0-100) onto this range, 1.0 at reception 50 — great
/// nights fill rooms, rough ones don't.
pub(super) const RECEPTION_ATTENDANCE_MIN_FACTOR: f32 = 0.85;
pub(super) const RECEPTION_ATTENDANCE_MAX_FACTOR: f32 = 1.15;

// Synthesized tour-stop venue capacity, drawn from the region's population
// and economic strength [tune].
pub(super) const TOUR_VENUE_CAPACITY_POP_DIVISOR: f32 = 300.0;
pub(super) const TOUR_VENUE_CAPACITY_MIN: u32 = 300;
pub(super) const TOUR_VENUE_CAPACITY_MAX: u32 = 25_000;

// Stat costs and rewards for shows (§A/§B).
pub(super) const GIG_STRESS_COST: u8 = 8;
pub(super) const TOUR_STRESS_COST_PER_WEEK: u8 = 12;
pub(super) const TOUR_HEALTH_COST_PER_WEEK: u8 = 4;
pub(super) const GREAT_SHOW_CREATIVITY_GAIN: u8 = 2;
pub(super) const TRANSCENDENT_SHOW_CREATIVITY_GAIN: u8 = 3;
pub(super) const TRANSCENDENT_SHOW_HAPPINESS_GAIN: u8 = 2;
/// Tour verdict threshold: average reception at or above this means the tour
/// "went very well" (§B).
pub(super) const TOUR_WENT_WELL_RECEPTION_THRESHOLD: u8 = 70;
pub(super) const TOUR_WENT_WELL_HAPPINESS_GAIN: u8 = 8;
pub(super) const TOUR_WENT_WELL_CREATIVITY_GAIN: u8 = 5;

// --- L12: derived live-skill growth (fixes the open finding that
// `average_member_skill()` and `reputation.live_performance` were the
// reception formula's two dominant terms yet never moved after band
// creation). Rehearsal builds the former; stage time builds the latter. ---
/// Each band member's individual skill gained per week of Practice. [tune]
pub(super) const PRACTICE_MEMBER_SKILL_GAIN: u8 = 1;
/// `reputation.live_performance` gained per show, by verdict. Rough nights
/// teach nothing; better nights teach more. [tune]
pub(super) const SOLID_SHOW_LIVE_REPUTATION_GAIN: u8 = 1;
pub(super) const GREAT_SHOW_LIVE_REPUTATION_GAIN: u8 = 2;
pub(super) const TRANSCENDENT_SHOW_LIVE_REPUTATION_GAIN: u8 = 3;

// Action guards (energy guards → stress/health economy, §A). `pub` (not
// `pub(super)`): `ui/app.rs`'s menu entries read these directly to keep the
// enabled/disabled state in lockstep with the action guards in `live.rs`.
pub const GIG_STRESS_GUARD: u8 = 85;
pub const GIG_HEALTH_GUARD: u8 = 20;
pub const TOUR_STRESS_GUARD: u8 = 70;
pub const TOUR_HEALTH_GUARD: u8 = 30;

// ============================================================================
// M2: the lifestyle ladder (docs/DESIGN-v0.7-money-cycle.md §B). Weekly
// upkeep, stat effects, and one-shot move consequences, deducted/applied
// in `lifestyle.rs`. Per-tier arrays are indexed by `LifestyleTier`'s
// declaration order: Squat, Shared flat, City apartment, Townhouse,
// Mansion. [tune] except where noted.
// ============================================================================

/// Weekly rent, by tier. [tune]
/// L-SCALE (§B): ×3 to the era-real scale (was [0, 40, 180, 700, 2_800]).
/// `move_up_cost` derives from this, so lifestyle deposits scale in lockstep.
pub(super) const LIFESTYLE_UPKEEP_PER_WEEK: [u32; 5] = [0, 120, 540, 2_100, 8_400];
/// Added to `STRESS_PASSIVE_RELEASE`, by tier. [tune]
pub(super) const LIFESTYLE_STRESS_RELEASE_BONUS: [u8; 5] = [0, 1, 2, 3, 4];
/// The weekly stress drain cannot pull happiness below this floor
/// (event/incident losses still can), by tier. [tune]
pub(super) const LIFESTYLE_HAPPINESS_FLOOR: [u8; 5] = [0, 5, 10, 15, 20];
/// Added to the health/stress recovery of rest-type actions (`LazeAround`,
/// `TakeBreak`), by tier. [tune]
pub(super) const LIFESTYLE_REST_HEALING_BONUS: [u8; 5] = [0, 1, 2, 3, 4];

/// Fame at/above which living at Squat or Shared flat draws tabloid
/// attention. [tune]
pub(super) const LIFESTYLE_IMAGE_FAME_THRESHOLD: u8 = 60;
/// Happiness lost per week while the image penalty holds. [tune]
pub(super) const LIFESTYLE_IMAGE_HAPPINESS_LOSS: u8 = 2;

/// Moving up: deposit of this many weeks' upkeep, on top of the first
/// week (so 5 weeks' worth of the new tier's upkeep, total). [tune]
pub(super) const LIFESTYLE_MOVE_UP_DEPOSIT_WEEKS: u32 = 4;
/// One-shot happiness gain on a voluntary move up. [tune]
pub(super) const LIFESTYLE_MOVE_UP_HAPPINESS: u8 = 10;
/// One-shot happiness loss on a voluntary move down. [tune]
pub(super) const LIFESTYLE_MOVE_DOWN_HAPPINESS: u8 = 15;

/// Consecutive weeks with money < 0 before the landlord forces a
/// downgrade (decided design, not [tune]).
pub(super) const LIFESTYLE_EVICTION_WEEKS: u32 = 2;
/// One-shot happiness loss from a forced eviction. [tune]
pub(super) const LIFESTYLE_EVICTION_HAPPINESS: u8 = 20;

// ============================================================================
// M1: Tour economics — rig picker, length picker, itemized up-front quote
// (docs/DESIGN-v0.7-money-cycle.md §A). Fame never re-prices a tour: it only
// gates which rigs/lengths are selectable and how many seats a tour fills.
// Same region + rig + length = same cost, at any fame.
// ============================================================================

/// The four tour rigs (`TourRig`, `actions/live.rs`), in ascending scale —
/// index-aligned with every `TOUR_RIG_*` table below.
pub(super) const TOUR_RIG_FAME_GATE: [u8; 4] = [0, 25, 55, 75];

/// Cost per tour week, before country travel mult and the rig's
/// `markets.json` travel/equipment modifiers (design §A table).
///
/// M7 (§F): compressed from the original [150, 600, 2_500, 8_000], which —
/// against a tour box office of only a few hundred to a few thousand — made
/// every rig above the van a guaranteed deep loss at any fame. Paired with
/// the `TOUR_GROSS_COEFFICIENT` bump below so a bigger rig's capacity boost
/// can actually clear its cost: the van profits for a small act, full
/// production only once the act is big enough to fill the rooms it books.
///
/// L-SCALE (§B): ×3 to the era-real scale (was [120, 400, 1_200, 3_000]), in
/// lockstep with the ×3 on `TOUR_GROSS_COEFFICIENT` — box office and rig cost
/// both triple, so every rig's profit/loss sign and ordering are preserved.
pub(super) const TOUR_RIG_COST_PER_WEEK: [u32; 4] = [360, 1_200, 3_600, 9_000];

/// Venue-capacity multiplier: a bigger rig books bigger rooms (design §A).
pub(super) const TOUR_RIG_CAPACITY_MULT: [f32; 4] = [0.8, 1.0, 1.3, 1.7];

/// The tour box-office coefficient (M7 §F): scales the whole-tour gross pot
/// (`base_gross = sqrt(pop) × econ × audience × this`). Raised 0.06 → 0.15
/// so that, with the capacity multiplier above and the compressed rig costs,
/// touring is a real revenue path — a full-production rig turns a profit
/// once the act is famous enough to fill its rooms, not a bottomless loss.
///
/// L-SCALE (§B): ×3 to the era-real scale (0.15 → 0.45) — this is the tour
/// income lever, tripling box office in lockstep with `TOUR_RIG_COST_PER_WEEK`
/// so touring's profitability ratios (§F: van profitable, heavy rigs a
/// capacity bet) hold while magnitude triples. Missing it would leave touring
/// bots (road-dog) flat against the ×3 record economy.
pub(super) const TOUR_GROSS_COEFFICIENT: f32 = 0.45;

/// Wear per tour week — health lost, replacing the flat
/// `TOUR_HEALTH_COST_PER_WEEK` for the headline tour (support tours keep the
/// flat cost). The van grinds you down; the production rig has roadies.
pub(super) const TOUR_RIG_HEALTH_COST_PER_WEEK: [u8; 4] = [5, 4, 3, 2];
/// Wear per tour week — stress gained, replacing the flat
/// `TOUR_STRESS_COST_PER_WEEK` for the headline tour (design §A).
pub(super) const TOUR_RIG_STRESS_COST_PER_WEEK: [u8; 4] = [9, 8, 6, 5];

/// Tour length picker bounds: 1-4 weeks (today's fame-derived length is
/// deleted along with the fame-derived cost tier).
pub(super) const TOUR_LENGTH_MIN_WEEKS: u8 = 1;
pub(super) const TOUR_LENGTH_MAX_WEEKS: u8 = 4;
/// Fame required to select each length, indexed by `weeks - 1`: 3 weeks
/// gated at fame 40, 4 at fame 60 (design §A); 1-2 weeks are never gated.
pub(super) const TOUR_LENGTH_FAME_GATE: [u8; 4] = [0, 0, 40, 60];

/// Fame and regional-fame gains scale sublinearly with tour length — a long
/// tour is a bigger investment, not a strictly better one (design §A,
/// explicitly left to be picked here) — via `base * weeks.powf(exponent)`,
/// rounded. At exponent 0.7: 1wk -> x1.0, 2wk -> x1.62, 3wk -> x2.16,
/// 4wk -> x2.64 (roughly two-thirds of linear scaling). [tune]
pub(super) const TOUR_FAME_GAIN_BASE: f32 = 4.0;
pub(super) const TOUR_FAME_WEEKS_EXPONENT: f32 = 0.7;
/// Regional fame reuses the same sublinear weeks curve; the old flat
/// `10 + rng(0..=5)` becomes this base plus the same rng spread. [tune]
pub(super) const TOUR_REGIONAL_FAME_GAIN_BASE: f32 = 7.0;
pub(super) const TOUR_REGIONAL_FAME_GAIN_RNG_SPREAD: u8 = 5;

// ============================================================================
// M5: Label recoupment + label auto-repress
// (docs/DESIGN-v0.7-money-cycle.md §E-2 and the §E-1 label half). The label's
// money is a loan, not a gift: the advance at signing plus pressing and promo
// at every release accrue to the deal's `unrecouped` ledger, and royalty
// income pays that balance down before any of it reaches the player.
// ============================================================================

/// Per-copy pressing cost the label books against recoupment (design §E-2:
/// $0.30/copy at the pre-M7 10-units-per-point scale). Applied to the label's
/// pressing run at each release and to every auto-repress run (§E-1 label
/// half).
///
/// COUPLED to the M7 sales rescale: `UNITS_PER_SCORE_POINT` tripled the
/// copies a score moves while `SALES_INCOME_DIVISOR` kept royalty dollars
/// flat, so this per-copy charge must divide by at least the same ratio
/// ($0.30 ÷ 3 = $0.10). Left at $0.30 it outran every royalty below 30% and
/// recoupment was structurally impossible (issue #20).
///
/// Then halved once more to $0.05 (break-even at a 5% royalty — majors net
/// $0.05–0.07/copy after pressing, indies far more): issue #20's $0.10
/// suggestion was calibrated while instant actions still re-ran the weekly
/// sales pass, inflating catalog royalty velocity; with that double-count
/// fixed, $0.10 left the median first deal recouping at ~58% of its term,
/// missing the §F target ("median signed act recoups the advance before the
/// term's halfway mark" — asserted by
/// `median_signed_act_recoups_the_advance_before_the_terms_halfway_mark`,
/// which is the number to re-check first if this moves). If
/// `UNITS_PER_SCORE_POINT` or `SALES_INCOME_DIVISOR` move again, carry this
/// constant along. [tune]
///
/// L-SCALE (§B): `SALES_INCOME_DIVISOR` did move (retired, 3 → 1), so this
/// per-copy charge ×3 (0.05 → 0.15) to keep the recoup coupling — the ledger's
/// pressing accrual triples alongside the ×3 royalty income, so the recoupment
/// *trajectory in weeks* is unchanged and the §F "recoup before halfway" target
/// holds exactly as it did at the v0.7 scale.
pub(super) const LABEL_RECOUP_PRESSING_PER_COPY: f32 = 0.15;
/// Recoupment cost per point of promo push the label applies in
/// `apply_label_promo` (design §E-2: $15/point).
///
/// L-SCALE (§B): ×3 (15 → 45) — a dollar in the recoupment ledger, tripled
/// alongside the advance, the pressing accrual, and royalty income so the
/// recoup ratio is preserved. Beyond the task's explicit cost list, but the
/// "every dollar ×3, ratios preserved" directive covers it: leaving it flat
/// would make the promo slice of the ledger recoup ~3× faster relative to the
/// tripled income. [tune]
pub(super) const LABEL_RECOUP_PROMO_PER_PUSH: i32 = 45;

// ============================================================================
// M9: Deal lifecycle — term, breach, renewal window, active label
// (docs/DESIGN-v0.7-money-cycle.md §E-4, §E-5). A contract binds both
// directions: albums owed AND a term served — free agency at the later of
// the two. The term expiring with albums still owed is a breach; a
// recoupment-dependent renewal window opens before a healthy term's
// expiry; the label leans harder on the single-cut machinery and sends
// memos while it's owed money.
// ============================================================================

/// Contract term at signing, by tier — inclusive week ranges (design §E-4
/// table: Boutique 52-78, Independent 78-104, Major 104-156). [tune]
pub(super) const DEAL_TERM_BOUTIQUE_WEEKS: (u16, u16) = (52, 78);
pub(super) const DEAL_TERM_INDEPENDENT_WEEKS: (u16, u16) = (78, 104);
pub(super) const DEAL_TERM_MAJOR_WEEKS: (u16, u16) = (104, 156);

/// Breach (design §E-4): reputation hit and the cooldown before any label
/// makes a new offer. [tune]
pub(super) const DEAL_BREACH_REPUTATION_HIT: u8 = 10;
pub(super) const DEAL_BREACH_COOLDOWN_WEEKS: u16 = 26;

/// The renewal window opens this many weeks before a healthy term's expiry
/// (albums already delivered). [tune]
pub(super) const DEAL_RENEWAL_WINDOW_WEEKS: u32 = 26;

/// Renewal-window ledger thresholds (design §E-4) deciding new contract /
/// extension / silence. "Decent" / "weak" sales read off
/// `reputation.commercial_success`; "deep in the red" off `unrecouped`.
/// Recouped + decent sales → new contract; deep in the red + weak sales →
/// silence; everything else (the common case: not yet recouped) →
/// extension. [tune]
pub(super) const DEAL_RENEWAL_DECENT_SALES_MIN_COMMERCIAL_SUCCESS: u8 = 20;
pub(super) const DEAL_RENEWAL_WEAK_SALES_MAX_COMMERCIAL_SUCCESS: u8 = 8;
/// L-SCALE (§B): ×3 (10_000 → 30_000) — this is a dollar threshold on the
/// `unrecouped` ledger, whose balances all tripled (advance + pressing/promo
/// accrual). Tripling it keeps "deep in the red" the same *proportion* of a
/// deal, so renewal behaviour (new contract / extension / silence) is
/// preserved in shape. Beyond the task's explicit cost list, but required by
/// the "ratios preserved" directive (the two sales-threshold constants above
/// are unitless reputation points, so they stay). [tune]
pub(super) const DEAL_RENEWAL_DEEP_RED_UNRECOUPED: i32 = 30_000;

/// New-contract renewal terms: royalty bump range (raw rate, i.e. 0.02 =
/// 2pp) on top of the current deal's rate, clamped to 1.0. [tune]
pub(super) const DEAL_NEW_CONTRACT_ROYALTY_BUMP_MIN: f32 = 0.02;
pub(super) const DEAL_NEW_CONTRACT_ROYALTY_BUMP_MAX: f32 = 0.04;

/// Extension renewal terms: one more album, a year longer, royalty
/// unchanged, a small advance as a fraction of the old deal's advance — the
/// label protecting its investment, not rewarding the band. [tune]
pub(super) const DEAL_EXTENSION_ALBUMS: u8 = 1;
pub(super) const DEAL_EXTENSION_TERM_WEEKS: u16 = 52;
pub(super) const DEAL_EXTENSION_ADVANCE_FRACTION: f32 = 0.15;

/// Recoup pressure (design §E-5): while `unrecouped > 0` the label's
/// single-cut chance doubles and its idle-weeks gate drops (3 → 2). [tune]
pub(super) const LABEL_CUT_CHANCE_PRESSURE_MULTIPLIER: f64 = 2.0;
pub(super) const LABEL_CUT_IDLE_WEEKS_PRESSURED: u32 = 2;

/// Label memos (design §E-5): weekly roll while a condition holds, one
/// memo max per week. The deadline memo's stress bite applies every week
/// the condition holds, independent of whether the memo message itself
/// rolled — "the deadline is real pressure, not flavor". [tune]
pub(super) const DEAL_MEMO_CHANCE: f64 = 0.25;
pub(super) const DEAL_MEMO_IDLE_WEEKS: u32 = 4;
pub(super) const DEAL_MEMO_DEADLINE_WINDOW_WEEKS: u32 = 12;
pub(super) const DEAL_MEMO_DEADLINE_STRESS_PER_WEEK: u8 = 3;

// ============================================================================
// M6: indie re-press + indie distribution tiers
// (docs/DESIGN-v0.7-money-cycle.md §E-1 indie half, §E-3). Distribution
// channel tables are index-aligned with `DistributionChannel::ALL`
// (`music.rs`): Mail order & gigs, Regional distributor, National
// distributor. Re-pressing itself reuses `PRESSING_TIERS` and
// `pressing_cost` (M1/pre-existing) — nothing new to tune there.
// ============================================================================

/// Fame required to select each channel (design §E-3 table). Only the
/// National gate is meaningfully tunable; Mail order/Regional are ungated
/// by design (`0`). [tune]
pub(super) const DISTRIBUTION_CHANNEL_FAME_GATE: [u8; 3] = [0, 0, 35];
/// Fee charged at each release under this channel (design §E-3 table).
/// L-SCALE (§B): ×3 to the era-real scale (was [0, 400, 1_500]) so an indie's
/// distribution outlay tracks its ×3 record income. [tune]
pub(super) const DISTRIBUTION_CHANNEL_FEE: [i32; 3] = [0, 1_200, 4_500];
/// Reach floor: effective indie reach is `max(floor, current fame formula)`
/// (design §E-3 table). [tune]
pub(super) const DISTRIBUTION_CHANNEL_REACH_FLOOR: [f32; 3] = [0.15, 0.30, 0.50];
/// International presence floor by channel: how far a self-releasing act
/// reaches into the three *foreign* sales territories (Europe/America/Japan)
/// on distribution alone, before any touring. Mail order stays home; a
/// regional distributor leaks a little across borders; a national
/// distributor gets picked up for export. This is the indie half of the fix
/// for "no international independent distribution" — the reach the player can
/// *buy* into foreign markets, on top of which touring's regional fame still
/// stacks (`economy::territory_presence`). Kept well below the label network
/// coefficient: signing is still the real passport abroad. [tune]
pub(super) const DISTRIBUTION_CHANNEL_INTERNATIONAL_FLOOR: [f32; 3] = [0.0, 0.08, 0.20];

/// A release counts as eligible for an indie re-press once cumulative sales
/// reach this fraction of the pressed run — "sold out (or low on stock)"
/// (design §E-1). [tune]
pub(super) const REPRESS_LOW_STOCK_SOLD_RATIO: f32 = 0.9;

// ============================================================================
// M10: player regional presence + sum-over-territories sales
// (docs/DESIGN-v0.7-money-cycle.md §C — "Presence gates entry" and
// "Regional sales — copies scale with presence"). The player's release
// submits to the Local scene board (full score, home turf) and to each of
// the four `ChartRegion::TERRITORIES` at `score × presence(territory)`, and
// demand is the same presence sum over those four territories. Presence is
// `reach × regional-fame factor`, where reach is the release's distribution
// reach (label `market_reach` when signed, else the bought channel / indie
// fame formula — M6's `reach_for`, per-release floor preserved) and the
// regional-fame factor is how known the act is in that territory's country.
// Local is a UK subset: full score on its board, but it never adds demand
// of its own and never feeds Worldwide (that stays M3's rule). All numbers
// here are [tune] — M7's sim lab calibrates them against §F targets.
// ============================================================================

/// Presence on the Local scene board — always full score. Local is home
/// turf (design §C): the player always enters, but its sales are already UK
/// sales, so this board never contributes demand or Worldwide.
pub(super) const LOCAL_PRESENCE: f32 = 1.0;

/// Regional fame (`0..=100`) normalizes by this into the `0..1` "how known
/// the act is in this country" factor that ADDS to the distribution floor
/// (below) for the three foreign territories. Touring is what raises regional
/// fame (`actions/live.rs`); on top of whatever floor the act's distribution
/// already grants abroad, it deepens a specific territory toward full
/// presence. A studio-only act reads 0 here and rides its distribution floor
/// alone. [tune]
pub(super) const REGIONAL_FAME_PRESENCE_DIVISOR: f32 = 100.0;

/// A label's international network, applied to the *square* of its
/// `market_reach` (`economy::international_floor_for`): the foreign-territory
/// presence floor a signed act gets on release day, before touring.
/// Distribution IS what a label sells — a major (reach ~95) puts the record in
/// Europe/America/Japan shops at once (floor ~0.81 → ~3.4× the home-only
/// demand), while a small indie label (reach ~20) is a domestic operation that
/// barely reaches abroad (floor ~0.04). The squared curve is what separates a
/// global major from a regional indie. This is the core fix for the sales
/// mechanic: `market_reach` used to be multiplied by a regional fame that only
/// touring could raise, so a label's reach did *nothing* abroad and every
/// signed act sold UK-only. The §C/§D thresholds were sized for the "present on
/// all four territories → 3–4× the copies" act this floor finally delivers.
/// Touring's regional fame still stacks on top. [tune]
pub(super) const LABEL_INTERNATIONAL_PRESENCE_COEFF: f32 = 0.9;

// ============================================================================
// L-ENTITY (v0.8 §A): the label balance sheet — seed capital + the weekly
// wholesale ledger. A label is now a world entity with `capital` that rises on
// the wholesale margin its roster sells and falls on what it spends. All
// numbers here are at the era-real L-SCALE scale (§B) and [tune] — L-SIM (§I)
// validates them against the "Gold nets low-six-figures / Platinum catalogue →
// millions" target and 15-year label solvency.
// ============================================================================

/// Seed `capital` = `financial_power³ × this`. Cubed so majors start flush and
/// boutiques lean (see `world::labels::seed_capital` for the reference points).
/// [tune] — sized so a major can front its top advance several times over
/// without being effectively infinite; L-SIM referees it.
pub(super) const LABEL_CAPITAL_PER_FP_CUBED: i64 = 15;

/// The label's per-copy wholesale ledger (design §A):
/// `margin_per_copy = wholesale − manufacturing − mechanical − artist_royalty`.
/// The artist royalty is NOT a constant here — it reuses the royalty the player
/// is actually paid (`LABEL_INCOME_PER_COPY × royalty_rate`, netted through
/// recoupment), so a still-unrecouped act's retained royalty stays with the
/// label as margin (the Albini asymmetry, §A). Era-real 1970s LP economics:
/// wholesale ~$4, manufacturing ~$1, mechanical (songwriter/publisher) ~$0.30.
///
/// NOTE (accounting seam, flagged for L-SIM): `manufacturing` here is the COGS
/// of a *sold* copy, while the pressing outlay debited to `capital` at release
/// (`LABEL_RECOUP_PRESSING_PER_COPY`, $0.15/copy *pressed* — the same figure
/// that accrues to the deal's `unrecouped` ledger) is the recoup-coupled cash
/// the label fronts for the run. Both are per the §A/task spec; their
/// magnitudes differ ($1.00 vs $0.15), so they are not one accounting line.
/// L-SIM owns reconciling/tuning the pair. [tune]
pub(super) const LABEL_WHOLESALE_PER_COPY: f32 = 4.0;
pub(super) const LABEL_MANUFACTURING_PER_COPY: f32 = 1.0;
pub(super) const LABEL_MECHANICAL_PER_COPY: f32 = 0.30;

// ============================================================================
// L-REACH (v0.8 §C): reach + tier drift. `market_reach` and `tier` stop being
// constants and drift toward a target derived from the label's health — its
// `capital` and its recent charting `hit_window` — bounded per period with a
// hysteresis dead-band so a label's rise or fall reads as a multi-year
// trajectory, never a jitter or a teleport (design §C). Because the sales fix
// made reach drive international distribution, a label's climb literally widens
// where its acts sell, and its fall contracts it. All numbers [tune] — L-SIM
// (§I) referees them against "boutiques can rise a tier, majors can fall,
// nobody teleports, the population stays mobile but stable" over 15 years.
// ============================================================================

/// How many charting releases a label's rolling `hit_window` holds (design §C).
/// The most-recent N charting record ids; older ones roll off the front, so the
/// window means "recent form", not a lifetime tally. Kept short. [tune]
pub(super) const HIT_WINDOW_LEN: usize = 5;

/// Reach points one recent hit adds to the drift target, and the cap on the
/// total hit bonus. The cap is below the Boutique→Major reach gap on purpose: a
/// hot streak lifts a label meaningfully, but hits alone can't peg reach to the
/// top — capital still gates the ceiling ("rich before big", §C). [tune]
pub(super) const REACH_PER_HIT: u8 = 4;
pub(super) const REACH_HIT_BONUS_CAP: u8 = 20;

/// Reach-drift cadence + rate (design §C: "drift, not teleport, bounded per
/// year"). The drift pass is wired into the weekly world tick but only *acts*
/// every `REACH_DRIFT_PERIOD_WEEKS` (monthly — the same rhythm as deal offers),
/// moving reach at most `REACH_DRIFT_STEP` points toward target each period.
/// `REACH_DRIFT_PER_YEAR` is the annual cap this yields (the documented [tune]
/// knob); `REACH_DRIFT_STEP` is derived from it so the per-period bound and the
/// yearly bound can never drift apart: a 2-point monthly step over 13 periods
/// ≈ 26 points/year, so a boutique climbing from ~20 toward a major's ~70 is a
/// ~2-year arc, not an overnight jump. [tune]
pub(super) const REACH_DRIFT_PER_YEAR: u8 = 26;
pub(super) const REACH_DRIFT_PERIOD_WEEKS: u32 = 4;
pub(super) const REACH_DRIFT_STEP: u8 =
    ((REACH_DRIFT_PER_YEAR as u32 * REACH_DRIFT_PERIOD_WEEKS) / WEEKS_PER_YEAR) as u8;

/// Hysteresis dead-band (design §C): reach only moves when it is more than this
/// many points from target, so a label already sitting at its natural reach
/// stays put instead of oscillating ±1 every period. [tune]
pub(super) const REACH_DRIFT_DEADBAND: u8 = 3;

/// The reach ceiling (reach is a 0..=`REACH_MAX` scale, mirroring the JSON
/// `market_reach`). Drift and the target are clamped to it.
pub(super) const REACH_MAX: u8 = 100;

/// Tier thresholds (design §C): `tier` follows `market_reach` across these
/// bands. Promotion needs BOTH reach past the promote threshold AND a capital
/// floor — a hit can spike a boutique's reach, but it isn't called big until
/// its balance sheet backs it ("a one-hit boutique gets rich before it gets
/// big", §C). Demotion is pure reach through a lower threshold: a label that
/// loses its distribution falls regardless of cash. The promote/demote gap is
/// the hysteresis that stops a label hovering at a boundary from flip-flopping
/// tier every period. [tune]
pub(super) const TIER_INDEPENDENT_REACH_PROMOTE: u8 = 38;
pub(super) const TIER_INDEPENDENT_REACH_DEMOTE: u8 = 30;
pub(super) const TIER_MAJOR_REACH_PROMOTE: u8 = 70;
pub(super) const TIER_MAJOR_REACH_DEMOTE: u8 = 62;
pub(super) const TIER_INDEPENDENT_MIN_CAPITAL: i64 = 200_000;
pub(super) const TIER_MAJOR_MIN_CAPITAL: i64 = 2_000_000;

// ============================================================================
// L-ROSTER (v0.8 §D): scene-band signing + roster P&L. Labels scout and sign
// unsigned scene acts by fit (genre specialty overlap) and appetite (weight
// class + capital), populate their `roster`, and book a plausible weekly
// wholesale margin off each signed act's chart presence — what finally makes
// majors *active* instead of sitting static at seed capital (L-ENTITY only ever
// moved the player's own label). All rolls derive from the injected world seed;
// labels/bands iterate in a deterministic order. Every number here is [tune],
// refereed by L-SIM's 15-yr label-solvency sweep (§I): sized to keep a
// deep-roster major accumulating in the low-to-mid millions over a career —
// the player-side ledger's ballpark (millions, not billions) — while boutiques
// stay solvent.
// ============================================================================

/// A scene act must reach this scene fame before any label will scout it —
/// below it an act is too small to be worth a contract. [tune]
pub(super) const ROSTER_SIGN_MIN_FAME: u8 = 20;

/// Scouting cadence: each unsigned, eligible band rolls this once a week to draw
/// label interest. Kept low so the ~180-band scene turns over gradually rather
/// than in a signing frenzy. [tune]
pub(super) const ROSTER_SCOUT_WEEKLY_CHANCE: f64 = 0.04;

/// A signing at/above this scene fame makes the news feed (matches the pre-0.8
/// scene-signing threshold); smaller signings happen silently. [tune]
pub(super) const ROSTER_SIGN_NEWS_FAME: u8 = 45;

/// Roster capacity by tier — a major carries a deep roster, a boutique a
/// handful. A label at capacity stops signing (design §D). [tune]
pub(super) const ROSTER_CAP_BOUTIQUE: usize = 6;
pub(super) const ROSTER_CAP_INDEPENDENT: usize = 15;
pub(super) const ROSTER_CAP_MAJOR: usize = 30;

/// Appetite ceiling: a label reaches for a band whose scene fame is no more than
/// this many points above its own `financial_power` — a lean boutique cannot
/// land a marquee act (it can't front the advance and the star wouldn't sign).
/// [tune]
pub(super) const ROSTER_APPETITE_FAME_HEADROOM: u8 = 15;

/// Caution: a label only signs when its `capital` covers at least this many
/// times the advance — so a label deep in the red never signs, and a lean one
/// signs sparingly (a broke label signs less, design §D). [tune]
pub(super) const ROSTER_SIGN_CAPITAL_SAFETY: i64 = 2;

/// Genre-fit bonus in label selection: a band whose genre sits in the label's
/// specialty scores this much higher, so labels lean toward their wheelhouse
/// without being locked to it (`GenrePurist` strictness is L-CHAR's, §G). [tune]
pub(super) const ROSTER_FIT_MATCH_BONUS: i32 = 40;

/// Roster-room weight in label selection: an emptier roster scores up to this
/// many extra points (scaled by how full it is, tier-independent), so signings
/// spread across peers instead of one label hogging its whole weight class
/// before any other bites. [tune]
pub(super) const ROSTER_ROOM_WEIGHT: i32 = 20;

/// Signing advance debited from a label's `capital` when it signs a scene act:
/// `base + fame × per_fame`. Modest against seed capital so it never bankrupts a
/// label on its own — the act's roster margin replenishes it (design §D). [tune]
pub(super) const ROSTER_SIGNING_BASE_COST: i64 = 1_000;
pub(super) const ROSTER_SIGNING_COST_PER_FAME: i64 = 150;

/// Scene-roster margin: a signed act's effective weekly chart score (summed over
/// every board it charts on, so a major's wider distribution earns more) × this
/// books to its label's `capital`. The scene analogue of the player-side
/// per-copy wholesale spread (a scene act has no `copies_sold` pipeline, so its
/// chart presence stands in for units moved). Sized so a scene act's per-release
/// margin lands in the same ballpark as a comparable player release and a
/// deep-roster major accumulates low-to-mid millions over 15 years — L-SIM
/// referees against the solvency sweep. [tune]
pub(super) const ROSTER_MARGIN_PER_CHART_SCORE: f32 = 1.0;

// ============================================================================
// L-BID (v0.8 §E): the deal-offer stream priced from live label state. An
// offer's advance and royalty come from the offering `Label`'s CURRENT
// `capital` / `market_reach` / `financial_power` / `tier` and the act's fame —
// not the static `record_labels.json` row. A label cannot advance money it does
// not have (advance bounded by `capital`), more than one label can table an
// offer in the same window, and when two-plus want the act their terms escalate
// toward the richer/hungrier label's ceiling, each bounded by its own means.
// Every number here is [tune], refereed by L-SIM (§I): the bidding must draw a
// hot act better offers than an unknown without making advances unrecoupable or
// bankrupting labels (the recoup + 15-yr solvency sweeps).
// ============================================================================

/// Affordability bound: a label will front at most this fraction of its
/// `capital` as a single advance — the hard "cannot advance money it does not
/// have" ceiling (design §E / "a label cannot spend money it does not have").
/// Well below 1.0 so signing never approaches insolvency; at seed
/// (`capital = fp³ × 15`) this ceiling is `fp³ × 2.25`, which sits above the
/// stature figure below, so a *healthy* label pays on stature/heat and only a
/// cash-strapped one is squeezed by affordability. [tune]
pub(super) const ADVANCE_CAPITAL_FRACTION: f32 = 0.15;

/// The marquee advance a label's *stature* justifies, `financial_power³ × this`
/// — what it would pay a fame-100 act before the affordability bound. Cubed so
/// the tier spread is steep (a major reaches for far more than a boutique),
/// matching the seed-capital curve. Sized so an indie's top advance lands in the
/// low tens of thousands and a major's in the high six/low seven figures — the
/// era-real ballpark the recoup sweep already validated. [tune]
pub(super) const ADVANCE_STATURE_PER_FP_CUBED: f32 = 1.1;

/// Fame's pull on the advance: the fraction of the stature figure an act commands
/// scales linearly from this floor (a total unknown) at fame 0 to 1.0 (the full
/// marquee figure) at fame 100 — "a hot act draws better offers than an unknown"
/// (design §E). [tune]
pub(super) const ADVANCE_FAME_FLOOR: f32 = 0.15;

/// A label only tables an offer when the advance it could afford
/// (`capital × ADVANCE_CAPITAL_FRACTION`) clears this floor — below it the label
/// is too cash-strapped to compete and stays quiet ("a label deep in the red
/// signs cautiously or not at all", design §E). At the seed scale even the
/// leanest label clears this comfortably; it bites only once `capital` has bled
/// far below seed. [tune]
pub(super) const ADVANCE_MIN_VIABLE_AFFORDABLE: i64 = 2_000;

/// Royalty priced from reach (design §E): a low-reach label competes on royalty
/// (it cannot compete on advance), a high-reach major pays the least. The rate
/// runs linearly from `MAX` at reach 0 down to `MIN` at reach 100, reproducing
/// the JSON shape (majors ~10-11%, indies ~22-25%) from *live* reach so it tracks
/// L-REACH drift. [tune]
pub(super) const ROYALTY_REACH_MAX: f32 = 0.32;
pub(super) const ROYALTY_REACH_MIN: f32 = 0.10;

/// A hot act also commands a better rate: fame adds up to this many points to the
/// reach-set royalty, linearly by fame. [tune]
pub(super) const ROYALTY_FAME_BONUS_MAX: f32 = 0.03;

/// Bidding room on royalty (design §E): under competition a label will sweeten
/// its reach-set rate by up to this premium, never past the hard cap. Royalty is
/// the cheap lever a poorer label can still pull when it cannot match a rival's
/// advance. [tune]
pub(super) const ROYALTY_BID_PREMIUM: f32 = 0.05;
pub(super) const ROYALTY_HARD_CAP: f32 = 0.35;

/// Bidding escalation (design §E): with two-plus offers on the table, each
/// closes this fraction of the gap to the best competing terms — advance toward
/// the top advance (bounded by the label's own affordability, so a broke label
/// cannot outbid a flush one) and royalty toward the top royalty (bounded by the
/// label's royalty ceiling). Half-way is a firm but not runaway war. [tune]
pub(super) const BID_ESCALATION_FRACTION: f32 = 0.5;

/// Genre fit sharpens appetite (design §E): a label whose specialty includes the
/// act's genre is this much likelier to table an offer, so a genre-fitting act
/// draws more concurrent suitors (and thus more bidding). Multiplies the tiered
/// base chance, clamped to a sane ceiling. [tune]
pub(super) const GENRE_FIT_CHANCE_MULTIPLIER: f64 = 1.5;
pub(super) const OFFER_CHANCE_MAX: f64 = 0.95;

// Determinism salts — stream construction lives in `rng.rs`.
// ACTION_STREAM_SALT keeps the action stream uncorrelated with the world
// stream (π's fractional bits: arbitrary, fixed forever).
pub(super) const ACTION_STREAM_SALT: u64 = 0x243F_6A88_85A3_08D3;
// Setup rolls (bandmate names) use this reserved pre-game week so they
// never replay week 1's action stream.
pub(super) const SETUP_STREAM_WEEK: u64 = 0;
