//! Record labels as living world entities (design §A, L-ENTITY).
//!
//! A label is no longer inert JSON: it becomes `GameWorld` state with a
//! balance sheet (`capital`) that rises on the wholesale margin its roster
//! sells and falls on the money it spends (advances, pressing, promo). This
//! module owns the `Label` struct, its supporting enums, and the deterministic
//! worldgen seeding from `data/record_labels.json`.
//!
//! **L-ENTITY scope.** This task lands the module and its public shape, seeds
//! the labels at worldgen, and books the weekly wholesale ledger for the
//! *player's* label (the ledger hook lives in `economy.rs`). Everything else
//! the §A struct carries is present but forward-compat — populated by later
//! tasks, with hook markers where they plug in:
//!
//! - `market_reach` / `tier` are seeded from the JSON and then **static** here.
//!   `// L-REACH will` drift them toward a capital/hit-window target (§C).
//! - `roster` / `hit_window` stay empty. `// L-ROSTER will` scout and sign
//!   scene bands and feed their releases' margin into `capital` (§D);
//!   `// L-REACH will` push charting release ids into `hit_window` (§C).
//! - `strategy` seeds to a neutral default. `// L-CHAR will` assign real
//!   strategies and wire their behaviour into offers/single-cuts (§G).
//! - `status` seeds to `Active`. `// L-MNA will` transition labels through
//!   `Distressed`/`Folding`/`Acquired` (§F).

use crate::data_loader::{GameDataFiles, RecordLabel};
use crate::game::constants::{
    HIT_WINDOW_LEN, LABEL_CAPITAL_PER_FP_CUBED, REACH_DRIFT_DEADBAND, REACH_DRIFT_PERIOD_WEEKS,
    REACH_DRIFT_STEP, REACH_HIT_BONUS_CAP, REACH_MAX, REACH_PER_HIT, ROSTER_APPETITE_FAME_HEADROOM,
    ROSTER_CAP_BOUTIQUE, ROSTER_CAP_INDEPENDENT, ROSTER_CAP_MAJOR, ROSTER_FIT_MATCH_BONUS,
    ROSTER_MARGIN_PER_CHART_SCORE, ROSTER_ROOM_WEIGHT, ROSTER_SCOUT_WEEKLY_CHANCE,
    ROSTER_SIGN_CAPITAL_SAFETY, ROSTER_SIGN_MIN_FAME, ROSTER_SIGN_NEWS_FAME,
    ROSTER_SIGNING_BASE_COST, ROSTER_SIGNING_COST_PER_FAME, TIER_INDEPENDENT_MIN_CAPITAL,
    TIER_INDEPENDENT_REACH_DEMOTE, TIER_INDEPENDENT_REACH_PROMOTE, TIER_MAJOR_MIN_CAPITAL,
    TIER_MAJOR_REACH_DEMOTE, TIER_MAJOR_REACH_PROMOTE,
};
use crate::game::genre::MusicGenre;
use rand::Rng;
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, HashMap};

use super::GameWorld;

/// A label's standing in the industry (design §A/§C). Seeded from which JSON
/// bucket the label came from; `// L-REACH will` make it drift with reach (§C).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum LabelTier {
    Boutique,
    #[default]
    Independent,
    Major,
}

/// A label's A&R personality (design §G). L-ENTITY seeds every label to
/// `Balanced`; the named variants are the §G vocabulary that L-CHAR assigns
/// and wires into offer pricing, single-cut aggression, and renewals.
///
/// `#[allow(dead_code)]`: the non-`Balanced` variants are constructed by
/// L-CHAR (§G), not yet here — kept so the enum's public shape is stable for
/// the downstream task rather than reshaped mid-cycle.
#[allow(dead_code)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum LabelStrategy {
    /// Neutral seed (L-ENTITY). L-CHAR replaces this with a real strategy.
    #[default]
    Balanced,
    /// Higher royalty, more creative freedom, patient with unrecouped acts.
    ArtistFriendly,
    /// Big advances, aggressive single-cuts, short patience.
    HitFactory,
    /// Only signs its specialty, but pays a premium for fit.
    GenrePurist,
}

/// Where a label is in its lifecycle (design §A/§F). L-ENTITY seeds every
/// label `Active`; `// L-MNA will` drive the other states (§F).
///
/// `#[allow(dead_code)]`: the non-`Active` variants are constructed by L-MNA
/// (§F) — present now so the serialized shape is stable across the cycle.
#[allow(dead_code)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum LabelStatus {
    #[default]
    Active,
    Distressed,
    Folding,
    Acquired,
}

/// One act on a label's roster (design §A/§D). L-ENTITY only defines the shape
/// so `Label::roster` is forward-compatible; the `Vec` stays empty here.
/// `// L-ROSTER will` fill it (scene-band scouting + the player's slot) and may
/// extend this struct (§D).
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct RosterSlot {
    /// The signed act, matched by name (how the scene identifies acts today,
    /// `SceneBand::name`).
    pub band_name: String,
    /// Week the act signed to this label.
    #[serde(default)]
    pub signed_week: u32,
}

/// A record label as a living world entity (design §A). Instantiated at
/// worldgen from `data/record_labels.json` (the JSON is now seed data, not a
/// runtime lookup table).
///
/// Every serialized field is `#[serde(default)]` per the cycle's serde rule, so
/// a field a later task adds to a label loads cleanly on an early-v0.8 save,
/// and the whole `labels` map is itself `#[serde(default)]` on `GameWorld` so a
/// pre-0.8 save (which has no labels at all) loads with an empty map.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Label {
    /// Stable id, assigned at worldgen in JSON order (majors, then
    /// independents, then boutiques). The `labels` `BTreeMap` key.
    #[serde(default)]
    pub id: u32,
    #[serde(default)]
    pub name: String,
    #[serde(default)]
    pub tier: LabelTier,
    /// Specialty genres, parsed from the JSON `specialty_genres` strings onto
    /// the game's `MusicGenre` enum (unmappable strings are dropped).
    #[serde(default)]
    pub genres: Vec<MusicGenre>,
    #[serde(default)]
    pub founded_year: u16,
    /// The balance sheet — the "up and down" (design §A). A real, conserved
    /// cash balance: the wholesale margin its roster sells adds to it; advances,
    /// pressing, and promo spend leave it. Seeded from `financial_power`.
    #[serde(default)]
    pub capital: i64,
    /// Distribution muscle, 0–100. Seeded from the JSON; **static in L-ENTITY**.
    /// `// L-REACH will` drift this toward a health-derived target (§C).
    #[serde(default)]
    pub market_reach: u8,
    /// Capacity to front advances, 0–100. Drives the seed `capital`.
    #[serde(default)]
    pub financial_power: u8,
    /// A&R patience/quality, 0–100 (design §G). Carried for L-CHAR.
    #[serde(default)]
    pub artist_development: u8,
    /// Personality, 0–100 (design §G). Carried for L-CHAR.
    #[serde(default)]
    pub creative_freedom: u8,
    /// A&R strategy (design §G). Neutral seed; L-CHAR assigns the real one.
    #[serde(default)]
    pub strategy: LabelStrategy,
    /// Lifecycle state (design §A/§F). `Active` at seed; L-MNA drives the rest.
    #[serde(default)]
    pub status: LabelStatus,
    /// Player + scene bands (design §D). Empty in L-ENTITY; L-ROSTER fills it.
    #[serde(default)]
    pub roster: Vec<RosterSlot>,
    /// Release ids that charted, rolling (design §C). Empty in L-ENTITY;
    /// L-REACH pushes charting ids and drives tier/reach drift off it.
    #[serde(default)]
    pub hit_window: Vec<u32>,
    /// Profit/loss this game-year, for news + tier drift (design §A). L-ENTITY
    /// moves it in lockstep with `capital` on every ledger booking so the
    /// signal is live; `// L-REACH will` read it (and reset it yearly) for the
    /// tier-drift curve (§C).
    #[serde(default)]
    pub ledger_ytd: i64,
}

impl Label {
    /// Book a ledger movement: `delta` dollars to `capital` (positive = margin
    /// earned, negative = money spent), mirrored into the year-to-date figure.
    /// The single conserved entry point for every capital change (design §A —
    /// "a label's money is real and conserved").
    pub fn book(&mut self, delta: i64) {
        self.capital += delta;
        self.ledger_ytd += delta;
    }

    /// L-REACH (§C): record a charting release in the rolling `hit_window` — the
    /// recent-form half of the reach-drift target. Bounded to `HIT_WINDOW_LEN`
    /// most-recent ids (older ones roll off the front) and de-duplicated, so a
    /// record that re-enters a chart never stacks. Deterministic (pure `Vec`
    /// ops). The player's charting releases feed the player's label from the
    /// sales pass (`Game::record_label_chart_hit`); scene-band chart hits are
    /// L-ROSTER's feed (§D) — see `// L-ROSTER feeds scene hits` in
    /// `GameWorld::drift_labels`.
    pub(in crate::game) fn record_chart_hit(&mut self, release_id: u32) {
        if self.hit_window.contains(&release_id) {
            return;
        }
        self.hit_window.push(release_id);
        let overflow = self.hit_window.len().saturating_sub(HIT_WINDOW_LEN);
        if overflow > 0 {
            self.hit_window.drain(0..overflow);
        }
    }

    /// L-REACH (§C): the reach this label's health justifies — the drift target.
    /// A capital-derived base (the inverse of `seed_capital`, so a label whose
    /// balance sheet matches financial-power `fp` sits near reach ≈ `fp`, i.e.
    /// roughly its seed reach) plus a bounded bonus for recent charting hits.
    /// Flush + hot → a high target it drifts up toward; broke + cold → ~0, which
    /// it drifts down toward. Bounded 0..=`REACH_MAX`. Pure and deterministic.
    pub(in crate::game) fn reach_target(&self) -> u8 {
        // capital = fp³ × k  ⇒  fp = ∛(capital / k). Negative/zero capital
        // (a label bleeding red) floors the base at 0 → the label cools toward 0.
        let base = ((self.capital.max(0) as f64) / (LABEL_CAPITAL_PER_FP_CUBED as f64))
            .cbrt()
            .round()
            .clamp(0.0, f64::from(REACH_MAX)) as i32;
        let hits = self.hit_window.len() as i32;
        let bonus = (hits * i32::from(REACH_PER_HIT)).min(i32::from(REACH_HIT_BONUS_CAP));
        (base + bonus).clamp(0, i32::from(REACH_MAX)) as u8
    }

    /// L-REACH (§C): nudge `market_reach` one period toward `reach_target`,
    /// moving at most `REACH_DRIFT_STEP` points and only when the gap exceeds
    /// the `REACH_DRIFT_DEADBAND` hysteresis band (so a label at rest doesn't
    /// jitter), then let `tier` follow the new reach via `tier_for`. Bounded
    /// 0..=`REACH_MAX`. Pure and deterministic — no RNG (design §C).
    pub(in crate::game) fn drift_reach_and_tier(&mut self) {
        let target = i32::from(self.reach_target());
        let reach = i32::from(self.market_reach);
        let gap = target - reach;
        if gap.unsigned_abs() > u32::from(REACH_DRIFT_DEADBAND) {
            let step = i32::from(REACH_DRIFT_STEP).min(gap.abs());
            let moved = reach + step * gap.signum();
            self.market_reach = moved.clamp(0, i32::from(REACH_MAX)) as u8;
        }
        self.tier = Self::tier_for(self.tier, self.market_reach, self.capital);
    }

    /// L-REACH (§C): the `tier` a label's reach + capital support, relative to
    /// its current tier. Promotion requires reach past the promote threshold AND
    /// a capital floor (a hit can spike reach, but the label isn't "big" until
    /// its balance sheet backs it — "rich before big"); demotion is pure reach
    /// falling through a lower threshold (the promote/demote gap is the
    /// hysteresis that stops boundary flip-flop). No capital gate on the way
    /// down: a label that loses its distribution falls however rich it is.
    pub(in crate::game) fn tier_for(current: LabelTier, reach: u8, capital: i64) -> LabelTier {
        let cap_major = capital >= TIER_MAJOR_MIN_CAPITAL;
        let cap_indie = capital >= TIER_INDEPENDENT_MIN_CAPITAL;
        match current {
            LabelTier::Major => {
                if reach < TIER_INDEPENDENT_REACH_DEMOTE {
                    LabelTier::Boutique
                } else if reach < TIER_MAJOR_REACH_DEMOTE {
                    LabelTier::Independent
                } else {
                    LabelTier::Major
                }
            }
            LabelTier::Independent => {
                if reach >= TIER_MAJOR_REACH_PROMOTE && cap_major {
                    LabelTier::Major
                } else if reach < TIER_INDEPENDENT_REACH_DEMOTE {
                    LabelTier::Boutique
                } else {
                    LabelTier::Independent
                }
            }
            LabelTier::Boutique => {
                if reach >= TIER_MAJOR_REACH_PROMOTE && cap_major {
                    LabelTier::Major
                } else if reach >= TIER_INDEPENDENT_REACH_PROMOTE && cap_indie {
                    LabelTier::Independent
                } else {
                    LabelTier::Boutique
                }
            }
        }
    }
}

impl GameWorld {
    /// Seed the label roster at worldgen from `data/record_labels.json`
    /// (design §A). Deterministic and RNG-free: capital is a pure function of
    /// `financial_power`, and ids are assigned in JSON order (majors →
    /// independents → boutiques), so the world stream is untouched and the
    /// determinism trio is unaffected. Iterating a `BTreeMap` (by id) keeps
    /// every later label pass deterministic.
    pub(super) fn seed_labels(data_files: &GameDataFiles) -> BTreeMap<u32, Label> {
        let data = data_files.get_record_labels_data();
        let mut labels = BTreeMap::new();
        let mut next_id = 0u32;

        let buckets = [
            (LabelTier::Major, &data.major_labels),
            (LabelTier::Independent, &data.independent_labels),
            (LabelTier::Boutique, &data.boutique_labels),
        ];
        for (tier, bucket) in buckets {
            for seed in bucket {
                let id = next_id;
                next_id += 1;
                labels.insert(id, Label::from_seed(id, tier, seed));
            }
        }
        labels
    }

    /// L-REACH (§C): the per-period reach/tier drift pass, wired into the weekly
    /// world tick (`update_week`). It is *called* every week but only *acts*
    /// every `REACH_DRIFT_PERIOD_WEEKS` (monthly), so the yearly movement stays
    /// inside `REACH_DRIFT_PER_YEAR`; between period boundaries it is a cheap
    /// no-op. Deterministic and RNG-free, and it iterates the `labels`
    /// `BTreeMap` in id order, so the determinism trio is unaffected (design §C:
    /// "World RNG injected, not ambient" — this pass needs none).
    ///
    /// `// L-ROSTER feeds scene hits here`: a scene band's charting release
    /// pushes into its label's `hit_window` (via `Label::record_chart_hit`) at
    /// the scene sim's charting site (§D). This pass then drives reach/tier off
    /// the combined capital + hit-window signal for *every* label — the player's
    /// and the scene's alike — so a rival act's breakout lifts its label too.
    pub(in crate::game) fn drift_labels(&mut self) {
        self.label_reach_tick = self.label_reach_tick.wrapping_add(1);
        if !self
            .label_reach_tick
            .is_multiple_of(REACH_DRIFT_PERIOD_WEEKS)
        {
            return;
        }
        for label in self.labels.values_mut() {
            label.drift_reach_and_tier();
        }
    }
}

impl Label {
    /// Instantiate a `Label` from one `record_labels.json` row.
    fn from_seed(id: u32, tier: LabelTier, seed: &RecordLabel) -> Self {
        Label {
            id,
            name: seed.name.clone(),
            tier,
            genres: parse_specialty_genres(&seed.specialty_genres),
            founded_year: seed.founded as u16,
            capital: seed_capital(seed.financial_power),
            market_reach: seed.market_reach,
            financial_power: seed.financial_power,
            artist_development: seed.artist_development,
            creative_freedom: seed.creative_freedom,
            // L-CHAR (§G) assigns the real strategy; a neutral seed here.
            strategy: LabelStrategy::Balanced,
            status: LabelStatus::Active,
            roster: Vec::new(),
            hit_window: Vec::new(),
            ledger_ytd: 0,
        }
    }
}

/// Seed a label's opening `capital` from its `financial_power` (0–100) at the
/// era-real L-SCALE money scale (§B). Cubed so the spread between tiers is
/// steep: a major (fp ~90) starts flush — millions, enough to front its top
/// advance several times over and still book margin — while a boutique
/// (fp ~15) starts lean — tens of thousands, enough for its small advances and
/// pressing but nowhere near a major's war chest. Sized against §B so advances
/// and pressing are affordable but not infinite. [tune] — L-SIM referees the
/// coefficient against label solvency over 15 years (§I).
///
/// Reference points at `LABEL_CAPITAL_PER_FP_CUBED = 15`:
///   Columbia (fp 95) → ~$12.9M · Atlantic (fp 90) → ~$10.9M ·
///   Sub Pop (fp 35) → ~$643k · Dischord (fp 15) → ~$51k · SST (fp 12) → ~$26k.
fn seed_capital(financial_power: u8) -> i64 {
    let fp = i64::from(financial_power);
    fp * fp * fp * LABEL_CAPITAL_PER_FP_CUBED
}

/// Map the JSON `specialty_genres` strings onto the game's `MusicGenre` enum.
/// The JSON carries finer real-world genres (e.g. "Post-Punk", "Shoegaze",
/// "Noise Rock", "Indie Pop") than the eight-variant enum, so each string is
/// matched to its closest enum variant by keyword; strings with no match are
/// dropped. Order-preserving and de-duplicated so the result is deterministic.
fn parse_specialty_genres(specialty: &[String]) -> Vec<MusicGenre> {
    let mut out: Vec<MusicGenre> = Vec::new();
    for s in specialty {
        if let Some(genre) = genre_from_specialty(s)
            && !out.contains(&genre)
        {
            out.push(genre);
        }
    }
    out
}

/// Closest `MusicGenre` for one specialty string. Keyword order matters: the
/// more specific tag wins (e.g. "post-punk" → Punk before the generic checks,
/// "noise rock" → Rock, "indie pop" → Pop). `None` for tags with no home in
/// the enum's vocabulary.
fn genre_from_specialty(s: &str) -> Option<MusicGenre> {
    let l = s.to_lowercase();
    if l.contains("metal") {
        Some(MusicGenre::Metal)
    } else if l.contains("punk") || l.contains("hardcore") {
        Some(MusicGenre::Punk)
    } else if l.contains("grunge")
        || l.contains("shoegaze")
        || l.contains("new wave")
        || l.contains("alternative")
        || l == "indie"
    {
        Some(MusicGenre::Alternative)
    } else if l.contains("jazz") || l.contains("blues") {
        Some(MusicGenre::Jazz)
    } else if l.contains("folk") {
        Some(MusicGenre::Folk)
    } else if l.contains("dance")
        || l.contains("disco")
        || l.contains("house")
        || l.contains("electronic")
        || l.contains("synth")
    {
        Some(MusicGenre::Electronic)
    } else if l.contains("rock") {
        // Art Rock / Indie Rock / Noise Rock / plain Rock.
        Some(MusicGenre::Rock)
    } else if l.contains("pop") || l.contains("r&b") {
        // Indie Pop / Brit Pop / plain Pop; R&B has no enum home → nearest Pop.
        Some(MusicGenre::Pop)
    } else {
        None
    }
}

// ============================================================================
// L-ROSTER (design §D): scene-band signing + roster P&L.
//
// Labels scout and sign unsigned scene acts by fit (genre specialty overlap)
// and appetite (weight class + capital), populate their `roster`, and book a
// plausible weekly wholesale margin off each signed act's chart presence. This
// is what makes the *whole industry* move: L-ENTITY only ever booked the
// player's own label, so majors sat static at seed capital. All rolls derive
// from the injected world seed; labels and bands iterate in a deterministic
// order (BTreeMap by id, `bands` by index).
//
// The logic lives here in `labels.rs` (the task's ownership); it is wired into
// the world with a single call, `tick_scene_rosters`, from the scene pass
// (`scene::update_scene_bands`).
// ============================================================================

impl GameWorld {
    /// The weekly scene-roster tick (design §D): reconcile each label's roster
    /// with the scene, scout and sign unsigned acts by fit + appetite, then book
    /// the roster's wholesale margin to label capital. Called once a week from
    /// the scene pass. The only randomness is the per-band scouting roll, drawn
    /// from the injected world `rng` in band-index order — so a seed replays
    /// signing and booking identically.
    pub(super) fn tick_scene_rosters(&mut self, rng: &mut impl Rng, news: &mut Vec<String>) {
        self.reconcile_rosters();
        self.scout_and_sign_scene_acts(rng, news);
        self.book_scene_roster_margin();
    }

    /// Keep every label's `roster` in step with the scene's `SceneBand::label`
    /// (the signed-to name — the scene's source of truth): drop slots for bands
    /// that left the scene or changed label, and register bands signed to a
    /// seeded label but not yet on its roster. That second pass is where the
    /// acts the worldgen scene starts *already signed* first appear on a roster:
    /// L-ENTITY seeds empty rosters (rosters are empty right after `new`), so
    /// they are picked up on the first weekly tick. Idempotent and RNG-free.
    ///
    /// The player is never on a scene roster (a signed player links to its label
    /// by `RecordDeal::label_id`, not a `RosterSlot`), so pruning by scene
    /// membership can never drop the player. A later task that puts the player
    /// on a roster must guard that here.
    fn reconcile_rosters(&mut self) {
        // Prune. A slot survives only if its band is still a scene act signed to
        // *this* label. `signed_to` borrows `self.bands`; the retain borrows
        // `self.labels` — disjoint fields, so the two borrows never overlap.
        {
            let mut signed_to: HashMap<&str, &str> = HashMap::new();
            for band in &self.bands {
                if let Some(label_name) = band.label.as_deref() {
                    signed_to.insert(band.name.as_str(), label_name);
                }
            }
            for label in self.labels.values_mut() {
                let this = label.name.clone();
                label.roster.retain(|slot| {
                    signed_to.get(slot.band_name.as_str()).copied() == Some(this.as_str())
                });
            }
        }

        // Register. Signed bands whose label is a seeded entity but which are
        // not yet on that label's roster (the worldgen-signed acts, week one).
        // Collected read-only first (immutable borrows of `bands` + `labels`),
        // then applied, so the label borrows never overlap.
        let mut additions: Vec<(u32, String)> = Vec::new();
        {
            let name_to_id: HashMap<&str, u32> = self
                .labels
                .iter()
                .map(|(id, label)| (label.name.as_str(), *id))
                .collect();
            for band in &self.bands {
                let Some(label_name) = band.label.as_deref() else {
                    continue;
                };
                // A name that isn't a seeded world entity (e.g. a band poached
                // to an ad-hoc "unknown label") simply has no balance sheet to
                // join — skip it.
                let Some(&id) = name_to_id.get(label_name) else {
                    continue;
                };
                if !self.labels[&id]
                    .roster
                    .iter()
                    .any(|s| s.band_name == band.name)
                {
                    additions.push((id, band.name.clone()));
                }
            }
        }
        for (id, band_name) in additions {
            if let Some(label) = self.labels.get_mut(&id) {
                label.roster.push(RosterSlot {
                    band_name,
                    // The world tick is week-agnostic (it does not carry the
                    // absolute game week), so scene-roster slots record 0; the
                    // player's own slot, when a deal is signed, stamps the real
                    // week from `Game` (`actions/business.rs`).
                    signed_week: 0,
                });
            }
        }
    }

    /// Scout unsigned scene acts and sign the ones a label wants (design §D).
    /// Each eligible unsigned band draws a weekly scouting roll; a band that
    /// draws interest is offered to the best-fitting label that can afford it
    /// (`pick_signing_label`), which debits the advance from its `capital`, adds
    /// the act to its `roster`, and stamps the band's `label`. Bands iterate in
    /// index order and the roll is the injected world `rng`, so signing replays
    /// identically for a seed.
    pub(crate) fn scout_and_sign_scene_acts(&mut self, rng: &mut impl Rng, news: &mut Vec<String>) {
        for idx in 0..self.bands.len() {
            let (fame, genre, unsigned, name) = {
                let band = &self.bands[idx];
                (
                    band.fame,
                    band.genre.clone(),
                    band.label.is_none(),
                    band.name.clone(),
                )
            };
            if !unsigned || fame < ROSTER_SIGN_MIN_FAME {
                continue;
            }
            if !rng.gen_bool(ROSTER_SCOUT_WEEKLY_CHANCE) {
                continue;
            }
            let Some(label_id) = self.pick_signing_label(fame, &genre) else {
                continue;
            };
            let cost = scene_signing_cost(fame);
            let label_name = {
                let label = self
                    .labels
                    .get_mut(&label_id)
                    .expect("pick_signing_label only returns live label ids");
                label.book(-cost);
                label.roster.push(RosterSlot {
                    band_name: name.clone(),
                    signed_week: 0,
                });
                label.name.clone()
            };
            let band = &mut self.bands[idx];
            band.label = Some(label_name.clone());
            band.momentum = (band.momentum + 1).min(3);
            if fame >= ROSTER_SIGN_NEWS_FAME {
                news.push(format!("🖋️ {} sign with {}.", name, label_name));
            }
        }
    }

    /// Choose the label that signs a scene act of the given fame/genre, or
    /// `None` if none will (design §D — fit + appetite). A candidate must be
    /// `Active`, have roster room, be within reach of the act's fame (the
    /// appetite ceiling), and hold enough `capital` to front the advance with a
    /// safety margin — so a label in the red never signs and a lean one signs
    /// sparingly ("a broke label signs less"). Among candidates the highest
    /// score wins: genre fit, weight-class match (a label most wants bands near
    /// its own financial stature, which stratifies signings so majors take the
    /// big names and boutiques the small), and a mild roster-room nudge for
    /// spread. Deterministic — BTreeMap id order breaks ties (lowest id wins),
    /// no RNG.
    pub(crate) fn pick_signing_label(&self, band_fame: u8, band_genre: &MusicGenre) -> Option<u32> {
        let cost = scene_signing_cost(band_fame);
        let mut best: Option<(i32, u32)> = None;
        for (id, label) in &self.labels {
            if label.status != LabelStatus::Active {
                continue;
            }
            let cap = roster_cap(label.tier);
            if label.roster.len() >= cap {
                continue;
            }
            // Appetite ceiling: a label cannot reach far above its means.
            if band_fame
                > label
                    .financial_power
                    .saturating_add(ROSTER_APPETITE_FAME_HEADROOM)
            {
                continue;
            }
            // A label cannot spend money it does not have, and only signs when
            // it can comfortably cover the advance (§D key constraint).
            if label.capital < cost.saturating_mul(ROSTER_SIGN_CAPITAL_SAFETY) {
                continue;
            }
            let fit = if label.genres.contains(band_genre) {
                ROSTER_FIT_MATCH_BONUS
            } else {
                0
            };
            // Higher = a closer match between the act's fame and the label's
            // financial stature; keeps the big names with the big labels.
            let weight_class =
                100 - (i32::from(band_fame) - i32::from(label.financial_power)).abs();
            let room_pct = (cap - label.roster.len()) as i32 * 100 / cap as i32;
            let score = weight_class + fit + room_pct * ROSTER_ROOM_WEIGHT / 100;
            if best.is_none_or(|(bs, _)| score > bs) {
                best = Some((score, *id));
            }
        }
        best.map(|(_, id)| id)
    }

    /// Book each label's weekly wholesale margin off its roster's chart presence
    /// (design §D). A signed scene act has no `copies_sold` pipeline, so its
    /// effective chart score — summed over every board it currently charts on,
    /// so a major's wider distribution earns more — stands in for units moved,
    /// scaled by `ROSTER_MARGIN_PER_CHART_SCORE`. Added to `capital` through
    /// `Label::book`, the same conserved entry point the player-side ledger
    /// uses; this only ever ADDS the scene roster's contribution and never
    /// touches the player's income or recoupment. RNG-free.
    pub(crate) fn book_scene_roster_margin(&mut self) {
        // Sum this week's effective chart score per scene act (player entries
        // excluded — the player's margin is the economy.rs ledger's job). Reads
        // `self.regional_charts`; the booking loop borrows `self.labels` —
        // disjoint fields.
        let mut chart_presence: HashMap<String, u64> = HashMap::new();
        for entries in self.regional_charts.values() {
            for entry in entries {
                if !entry.is_player {
                    *chart_presence.entry(entry.band_name.clone()).or_insert(0) +=
                        u64::from(entry.score);
                }
            }
        }
        for label in self.labels.values_mut() {
            let mut margin: i64 = 0;
            for slot in &label.roster {
                if let Some(&score_sum) = chart_presence.get(slot.band_name.as_str()) {
                    margin += (score_sum as f32 * ROSTER_MARGIN_PER_CHART_SCORE) as i64;
                }
            }
            if margin > 0 {
                label.book(margin);
            }
        }
    }

    /// L-ROSTER (§D): which label a charting scene act records for — the "rival
    /// act → label" readout L-UI (§H) surfaces on the charts. Reads the scene
    /// band's `label` (the signed-to name — the field L-ROSTER populates);
    /// `None` for an unsigned act, an ambient/filler chart row with no scene
    /// band behind it, or the player.
    #[allow(dead_code)] // consumed by L-UI (§H); the accessor lands with the field it reads.
    pub fn rival_act_label(&self, band_name: &str) -> Option<&str> {
        self.bands
            .iter()
            .find(|band| band.name == band_name)
            .and_then(|band| band.label.as_deref())
    }
}

/// Roster capacity for a tier (design §D): majors deep, boutiques a handful.
fn roster_cap(tier: LabelTier) -> usize {
    match tier {
        LabelTier::Boutique => ROSTER_CAP_BOUTIQUE,
        LabelTier::Independent => ROSTER_CAP_INDEPENDENT,
        LabelTier::Major => ROSTER_CAP_MAJOR,
    }
}

/// The advance a label fronts to sign a scene act of the given fame (design §D):
/// `base + fame × per_fame`. Debited from the label's `capital` at signing.
fn scene_signing_cost(band_fame: u8) -> i64 {
    ROSTER_SIGNING_BASE_COST + i64::from(band_fame) * ROSTER_SIGNING_COST_PER_FAME
}
