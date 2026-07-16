# Future Plan — Musician Identity, Relationships & Solo/Band Architecture

> **Since this plan was written, the v0.6 Life Cycle shipped independently**
> (`CHANGELOG.md` 0.6.0, `docs/DESIGN-v0.6-life-cycle.md`) and already
> covers some ground this document anticipated, in a different final shape:
> the four-bar stat model (health/stress/happiness/creativity — §9.2's
> quality multiplier, `0.8 + happiness/500`, shipped exactly as drafted
> there), per-show reception/box-office/momentum, fame gravity, and
> `average_member_skill()` / `reputation.live_performance` now growing
> through Practice and playing shows (a stopgap precursor to this
> document's ability-picker Practice rework in §2/§6 — current Practice
> still does a flat, uniform bump per member, not per-ability training).
> Addiction (§9.1), vacations (§9.3), and the manager (§9.4) remain fully
> unbuilt and are still queued — the v0.7 Money Cycle (tour economics,
> lifestyle ladder, chart stability, certifications, pressing &
> distribution costs; `docs/DESIGN-v0.7-money-cycle.md`) slotted in ahead
> of them. The Musician struct itself (§1–§8) is untouched — this remains
> the plan for that cycle.

This document describes three interlocking systems that must be designed and
built together. They replace HANDOFF task #5 (the flat `genre_ratings`
HashMap approach) with a richer model where genre proficiency is **derived**
from abilities, band chemistry is **derived** from personality, and solo vs
band mode is simply "a Band with 0 members."

Build order: §1 first (it restructures the data model everything else
depends on), then §2 (quality/genre formulas), then §3 (solo/band
transitions), then §4 (relationships & events), then §5 (chart hardening),
then §6 (UI).

---

## §1 — The Musician Struct

### Problem with the current model

- `Player` has **no musical skills** — just money/health/stress.
- `BandMember` has a single `skill: u8` and an `Instrument` enum.
- `Band` has a flat `skill: u8` that doesn't derive from its members.
- These three types are disconnected — the player isn't a musician, a band
  member isn't a person with personality, and there's no shared type
  between them.

### The unified Musician

Every person who makes music — the player, band members, scene musicians —
is a `Musician`. The player IS a musician who also has a wallet and a liver.

```
src/game/musician.rs  [NEW]
```

```rust
pub struct Musician {
    pub name: String,
    pub abilities: Abilities,
    pub personality: Personality,
    pub ego: u8,            // 0-100. High = better stage presence, harder to work with.
    pub reliability: u8,    // 0-100. Low = misses rehearsals, late to gigs.
    pub drug_problem: bool,
}

pub struct Abilities {
    // Performance — what you play
    pub vocals: u8,         // 0-100. Singing, harmonies, screaming.
    pub guitar: u8,         // 0-100. Lead, rhythm, acoustic — one stat.
    pub bass: u8,           // 0-100. Bass guitar, upright bass.
    pub drums: u8,          // 0-100. Drums, percussion, rhythm section.
    pub keys: u8,           // 0-100. Piano, synth, organ, electronic pads.
    pub strings_wind: u8,   // 0-100. Flute, violin, sax — the "color" instruments.

    // Creative — what you contribute to the music
    pub songwriting: u8,    // 0-100. Lyrics + melody composition.
    pub production: u8,     // 0-100. Studio skills, mixing, sound design.
    pub experimentation: u8,// 0-100. Genre flexibility, innovation.

    // Meta — stage and presence
    pub stage_presence: u8, // 0-100. Crowd engagement, live charisma.
}

pub struct Personality {
    pub genre_preference: MusicGenre,  // What they WANT to play.
    pub ambition: u8,       // 0-100. High = chases fame, leaves stalling bands.
    pub temperament: u8,    // 0-100. High = easygoing. Low = volatile, dramatic.
    pub openness: u8,       // 0-100. High = embraces change. Low = purist.
}
```

**11 abilities, 4 personality traits, 3 meta stats.** Enough to differentiate,
few enough to show in a TUI column.

### Migration from current types

| Current | Becomes |
|---|---|
| `Player` | Keeps: `name`, `money`, `health`, `energy`, `stress`, `drug_addiction`, `alcohol_addiction`. Gains: `pub musician: Musician`. |
| `BandMember` | Replaced by `Musician`. The `instrument: Instrument` enum is deleted — a musician's role is determined by their highest performance ability. `loyalty` → `Musician.reliability` + relationship score. |
| `Band.skill: u8` | Deleted. Band skill is derived from the best abilities across its members (see §2). |
| `Band.members: Vec<BandMember>` | Becomes `Vec<Musician>`. |
| `Instrument` enum | Deleted. A musician's "instrument" is whichever performance ability is highest. For display: `best_instrument(&self) -> &'static str`. |

### Generating random Musicians

```rust
impl Musician {
    pub fn random(name: String, rng: &mut impl Rng) -> Self {
        // One or two abilities are strong (40-75), rest are weak (5-25).
        // Personality is fully random.
        // ego and reliability are normally distributed around 50.
    }

    /// A musician strong in a specific genre's key abilities.
    pub fn random_for_genre(name: String, genre: &MusicGenre, rng: &mut impl Rng) -> Self {
        // Boosts the abilities that genre_weights() emphasizes.
    }
}
```

### Scene band musicians: lazy materialization

Scene bands (`SceneBand`) do NOT store `Vec<Musician>`. They keep their
current lightweight shape: `name, fame, peak_fame, genre, label, momentum`.
Their aggregate "skill" is a single `u8` derived at creation time from a
seed.

Musicians are **materialized on demand** — only when the player interacts
(joining the band, poaching a member, inspecting them in a future UI). Once
materialized, the `Vec<Musician>` is stored persistently.

```rust
pub struct SceneBand {
    // ... existing fields ...
    /// None = lightweight mode. Some = materialized roster.
    #[serde(default)]
    pub roster: Option<Vec<Musician>>,
}

impl SceneBand {
    /// Deterministically generate members from band name + genre.
    /// Once called, stores the result in self.roster.
    pub fn materialize(&mut self, data_files: &GameDataFiles, rng: &mut impl Rng) {
        if self.roster.is_some() { return; }
        let count = rng.gen_range(3..=5);
        let members = (0..count)
            .map(|_| {
                let name = data_files.generate_member_name(rng);
                Musician::random_for_genre(name, &self.genre, rng)
            })
            .collect();
        self.roster = Some(members);
    }
}
```

---

## §2 — Derived Genre Proficiency & Quality Formulas

### Genre proficiency is computed, not stored

Delete `genre_ratings: HashMap<MusicGenre, u8>` from the old FUTURE plan.
Instead, genre proficiency for a musician (or band) is a weighted sum of
the abilities that matter for that genre:

```rust
impl MusicGenre {
    /// Weights for each ability that contributes to this genre.
    /// Returns: (vocals, guitar, bass, drums, keys, strings_wind,
    ///           songwriting, production, experimentation, stage_presence)
    pub fn ability_weights(&self) -> [f32; 10] {
        match self {
            Rock        => [0.20, 0.25, 0.10, 0.15, 0.00, 0.00, 0.20, 0.05, 0.00, 0.05],
            Pop         => [0.30, 0.05, 0.05, 0.05, 0.15, 0.00, 0.20, 0.15, 0.00, 0.05],
            Metal       => [0.15, 0.30, 0.10, 0.25, 0.00, 0.00, 0.05, 0.05, 0.05, 0.05],
            Punk        => [0.15, 0.20, 0.10, 0.15, 0.00, 0.00, 0.05, 0.00, 0.05, 0.30],
            Alternative => [0.15, 0.20, 0.05, 0.10, 0.05, 0.00, 0.20, 0.10, 0.15, 0.00],
            Electronic  => [0.10, 0.00, 0.00, 0.05, 0.30, 0.00, 0.10, 0.25, 0.20, 0.00],
            Folk        => [0.25, 0.15, 0.05, 0.05, 0.05, 0.20, 0.20, 0.00, 0.05, 0.00],
            Jazz        => [0.10, 0.05, 0.10, 0.10, 0.15, 0.25, 0.10, 0.00, 0.15, 0.00],
        }
    }
}
```

For a **solo musician**, genre score = dot product of their abilities and
the genre weights.

For a **band**, genre score uses the **best ability across all members**
for each slot (the guitarist plays guitar, not the drummer):

```
band_genre_score(genre, player_musician, members) =
    for each ability_i:
        best_i = max(player_musician.ability_i, max over members of m.ability_i)
    dot_product(best_values, genre.ability_weights())
```

This means:
- A solo Metal artist needs strong Guitar AND Drums AND Vocals personally.
- A Metal band just needs its guitarist, drummer, and vocalist to each be
  strong. The solo artist is at a natural disadvantage for heavy genres.

### active_genre field

`Band` still needs `active_genre: MusicGenre` (default Rock) to know what
genre the player is currently writing/performing in. This determines:
- Which `ability_weights()` table applies to quality calculations.
- The era trend modifier (`era_genre_modifier`).
- The genre tag on new songs and releases.

Switching `active_genre` is a free action (`GameAction::ChangeGenre`), BUT
it has personality consequences (see §4).

### Revised quality formulas

**Songwriting quality** (replaces `calculate_songwriting_quality`):

```
songwriter_skill = best songwriter in (player_musician + members)
genre_score = band_genre_score(active_genre, player_musician, members)
creativity_bonus = existing creativity/happiness logic from v0.6 §A (unchanged)
chemistry_mult = see §4

base = (songwriter_skill.songwriting * 0.6
      + genre_score * 0.3
      + creativity_bonus) * chemistry_mult
quality = base + random_variation
```

**Recording quality** (replaces `calculate_release_quality`):

```
producer_skill = best production ability in band
genre_score = same as above
base = (avg_song_quality * 0.5 + producer_skill * 0.3 + genre_score * 0.2)
       * chemistry_mult
```

**Release sales score** (replaces `calculate_release_sales_score`):

```
existing formula (quality * weight + marketing * weight + fame * weight)
× era_sales_modifier         (unchanged)
× dynamic_genre_modifier     (unchanged)
× era_genre_modifier         (NEW — from data_files, same as scene bands use)
× proficiency_mult           (NEW — 0.7 + genre_score / 170, range ~0.7–1.3)
```

### Practice action

`GameAction::Practice` now opens a picker: **which ability to train?**

List the 10 abilities. Player picks one. That ability gets `+2` (capped
at 100). Energy cost and stress reduction unchanged.

This replaces the flat "+3 to genre rating" — the player is investing in
specific skills, which in turn improves their derived genre scores.

---

## §3 — Solo / Band Identity

### Core rule: `band.members.is_empty()` = solo

No new struct. Every system uses `Band` uniformly.

| Aspect | Solo | Band (≥1 member) |
|---|---|---|
| Name | Player's name or chosen stage name | Band name |
| Songwriting | `player.musician.songwriting` is the only songwriter | Best songwriter in group |
| Genre score | Player's abilities only | Best-of across all members |
| Live quality | Lower ceiling (one person) | Higher ceiling, but morale/chemistry issues |
| Record deals | Normal | Normal |
| Genre switch | Free, no social cost | Free action, but personality fallout (§4) |
| Weekly upkeep | $0 extra | $50 × member_count (salaries from `player.money`) |

### Setup flow

`[Your Name] → [Solo / Band?] → (if Band: [Band Name]) → [Starting Genre] → [Ability Allocation]`

- **Solo/Band toggle**: `←/→` switches. Default: Band (it's the classic experience).
- **Genre**: `←/→` cycles `MusicGenre::ALL`. Shows flavor text per genre.
- **Ability allocation**: Player gets **20 points** to distribute across 10
  abilities (minimum 0, maximum 40 at creation). Pre-filled with a
  genre-appropriate spread that the player can tweak. This IS the
  player's `Musician.abilities`.

If Band: generate 3 `Musician::random_for_genre(active_genre)` as members.
If Solo: `band.members = vec![]`.

### New game actions

```rust
GameAction::GoSolo                  // band → solo
GameAction::FormBand(String)        // solo → band
GameAction::JoinSceneBand(usize)    // solo → absorb a scene band
```

Each **consumes 1 week**.

#### GoSolo

1. Clear `band.members`.
2. `band.name = player.name`.
3. `band.fame = band.fame * 2 / 3` — you're known, but not the full act.
4. Void active record deal (`band.drop_deal()`). Log message.
5. Player's `musician.abilities` are unchanged — these are personal.
6. `player.stress += 15`.
7. Old chart entries get `is_player = false` (legacy entries decay
   normally).

The old band does NOT re-enter `world.bands`. They break up.

#### FormBand(name)

1. `band.name = name`.
2. Generate 3 `Musician::random_for_genre(active_genre)` as members.
3. `band.fame` unchanged — your audience follows you.
4. `player.money -= 200` (recruitment costs).
5. Initial relationships computed from personality compatibility (§4).

#### JoinSceneBand(index)

1. Validate: scene band exists, fame ≥ 15, player's fame within 30 of
   theirs.
2. Materialize scene band's roster (`scene_band.materialize()`).
3. Adopt identity: `band.name = scene_band.name`,
   `band.fame = scene_band.fame`.
4. `band.members = scene_band.roster`.
5. If scene band had a label: create `RecordDeal` with standard terms for
   that label's tier (2 albums, mid-range royalties).
6. Remove scene band from `world.bands`.
7. Their chart entries persist (same band, now player-controlled).
8. Initial relationships computed from personality compatibility.
9. `player.stress += 10`.

### Guards

- `FormBand` / `JoinSceneBand` only available when solo.
- `GoSolo` only available when in a band.
- Cannot switch if undelivered album obligation exists — must fulfill or
  drop deal first (reputation penalty: `reputation.commercial_success -= 10`,
  label won't offer for 26 weeks via `deal_cooldown: u16` on Band).

---

## §4 — Relationships

### Intra-band relationships (pairwise)

Stored on `Band` for the player's band only:

```rust
/// Relationship scores between pairs of musicians (including the player).
/// Key is a sorted (alphabetical) pair of names. Value: -100 to +100.
#[serde(default)]
pub relationships: BTreeMap<(String, String), i8>,
```

Using `BTreeMap` (not `HashMap`) so iteration order is deterministic for
the seeded RNG.

#### Initialization

When two musicians first meet (band formation, recruitment, joining),
compute starting relationship from personality compatibility:

```
base_compatibility(a, b) =
    + 15                                    // people generally start neutral-positive
    + if same genre_preference { 10 } else { -5 }
    - abs(a.ego - b.ego) / 5               // big ego gaps breed resentment
    - abs(a.ambition - b.ambition) / 4      // ambition mismatch
    + min(a.temperament, b.temperament) / 5 // one hothead drags it down
    + (a.openness + b.openness) / 10        // open people get along

result: clamped to -100..100, typically starts 10–60
```

#### Relationship drift & events

Each week during `advance_week_events`, the band's relationships shift:

- **Passive drift**: relationships drift slowly toward the base
  compatibility (people revert to their natural chemistry). ±1 per week.
- **Event triggers** (random, ~15% chance per week per pair):
  - Positive: "Mike and Sarah wrote a great riff together" → +5
  - Negative: "Dave stormed out of rehearsal after an argument with you" → -8
  - Event probability and direction weighted by `temperament` and `ego`.
- **Genre switch fallout**: When `ChangeGenre(g)` is called, each member
  whose `personality.genre_preference != g` AND `openness < 50` loses
  relationship with the player by `(50 - openness) / 5` (3–10 points).
- **Success boost**: Chart entry → all relationships +3 (shared triumph).
- **Failure stress**: Rejected deal / bad gig → random pair loses 2–5.

#### Gameplay effects

**Chemistry multiplier** (affects songwriting and recording quality):

```
avg_relationship = mean of all pairwise relationships in the band
chemistry_mult = 0.5 + (avg_relationship + 100) / 400
    // relationship -100 → mult 0.5  (hostile, barely functional)
    // relationship    0 → mult 0.75 (indifferent)
    // relationship  +50 → mult 0.875
    // relationship +100 → mult 1.0  (perfect harmony)
```

For solo musicians: `chemistry_mult = 0.85` (no friction, but no creative
sparks from collaboration either).

**Member departure**:

```
leave_chance_per_week(member, player_relationship) =
    base 0.5%
    + if player_relationship < 0:  abs(relationship) * 0.05%
    + if member.ambition > 70 and band.fame < 20:  1%
    + if member.reliability < 30:  1%
    + if member.drug_problem:  0.5%
```

When a member leaves: news log, member removed, relationships with them
deleted. If they were the best at a key ability, quality takes a real hit.

**Recruitment screening**: When recruiting (forming a band, replacing a
member), show personality compatibility preview. "⚠ Sarah has a volatile
temperament — could clash with Dave." Player can still recruit, but they're
making an informed choice.

### External relationships (future expansion, not v1)

For v1, external relationships (label execs, producers, rival musicians)
are NOT explicitly tracked. They're implied by existing systems: label
reputation via `BandReputation.commercial_success`, rival via the future
feud mechanic. This avoids scope creep.

---

## §5 — Chart Hardening

### No special eviction — ✅ already shipped

`src/game/world/charts.rs` already matches this exactly: `CHART_DECAY =
0.85`, `CHART_FLOOR_SCORE = 25`, `CHART_SIZE = 10`, pure score-based
lifecycle, no `is_player_bankrupt` or active-list eviction. Nothing to do
here — noted for the next two subsections, which are NOT yet done.

### Player identity changes and charts

When the player switches identity (GoSolo, FormBand, JoinSceneBand),
existing chart entries under the old name get `is_player = false`. They
decay normally. New releases chart under the new name. The player can
have entries under multiple names simultaneously (like Phil Collins and
Genesis co-existing on the charts).

### Uncapped scene population

Remove `SCENE_MAX_BANDS`. Always allow `rng.gen_range(0..=2)` newcomers
when above `SCENE_MIN_BANDS`. Natural attrition (fame < 8, momentum ≤ 0,
5% weekly breakup chance) is the only population control. Delete the
`SCENE_MAX_BANDS` constant.

### Genre tag on chart entries

Add `genre: MusicGenre` to `ChartEntry` for display and future filtering:

```rust
pub struct ChartEntry {
    pub title: String,
    pub band_name: String,
    pub is_player: bool,
    pub score: u32,
    pub weeks_on_chart: u32,
    pub genre: MusicGenre,      // NEW
}
```

---

## §6 — UI Changes

### Band panel

- Solo: show "Solo Artist" instead of member list. Show `[j] Manage`.
- Band: show member names with their top instrument and relationship
  indicator (💚 good / 😐 neutral / 🔴 tense).
- Show active genre: `"Genre: Rock"` + derived genre score as a bar or
  star rating.
- Below: top 3 abilities of the player's musician as a compact list.

### Practice picker

New modal on `p` (Practice): list of 10 abilities with current values.
`↑/↓` to select, `Enter` to train. Shows "+2" preview.

### Band Management modal (`j`)

- If solo: "Form a Band" (text input) / "Join a Band" (scrollable list
  of eligible scene bands with fame + genre shown).
- If in band: "Go Solo" (confirmation prompt) / "Fire Member" (select
  from list) / "Recruit" (if < 5 members).

### Setup flow

Four steps: Name → Solo/Band → (Band Name) → Genre → Abilities.
Ability allocation screen: 10 abilities, 20 points, genre-appropriate
defaults pre-filled.

### Charts modal (`c`)

Top 10 list: `#1 "Title" — Band Name (Genre) [3 wks]`. Player's entries
highlighted. Shows genre tag.

---

## §7 — Interaction Matrix (Edge Cases)

| Scenario | Resolution |
|---|---|
| Solo player switches genre | Free. No social cost (no one to annoy). |
| Band genre switch, closed-minded member | Relationship drops by `(50 - openness) / 5`. May trigger departure if already low. |
| Player practices Guitar but band plays Electronic | Guitar goes up. Their Electronic genre score doesn't improve (wrong ability). The player is wasting time — but it's their choice. |
| Band member is a better songwriter than the player | Quality formula uses the BEST songwriter. Having great members is genuinely valuable. |
| Scene band is joined but has a terrible roster | Player can fire members and recruit replacements. The band's fame is the prize, not the members. |
| All members leave one by one | `band.members` empties → player is effectively solo. No forced game over. They can recruit or stay solo. |
| Player goes solo mid-record-deal | Deal voided. `reputation.commercial_success -= 10`. `deal_cooldown = 26` weeks. |
| Two high-ego members in the band | Base compatibility is low. Frequent negative events. Great music if they click, but fragile. |
| Member with drug_problem | Lower reliability → higher leave chance. Health spiral. But might have great abilities (Keith Richards archetype). |

---

## §8 — Implementation Order

1. **`musician.rs` [NEW]** — `Musician`, `Abilities`, `Personality` structs
   and generation functions.
2. **`band.rs` [MODIFY]** — replace `Vec<BandMember>` with
   `Vec<Musician>`, add `active_genre`, `relationships`, `deal_cooldown`.
   Delete `skill: u8`, `Instrument` enum. Add derived-skill helpers.
3. **`player.rs` [MODIFY]** — add `pub musician: Musician`. Keep all
   existing fields.
4. **`world.rs` [MODIFY]** — add `genre` to `ChartEntry`, add
   `roster: Option<Vec<Musician>>` to `SceneBand`, remove
   `SCENE_MAX_BANDS`, update `submit_chart_entry` signature.
5. **`music.rs` [MODIFY]** — add genre to `Song` struct.
6. **`mod.rs` [MODIFY]** — rewrite quality formulas, practice action,
   new `GameAction` variants (GoSolo, FormBand, JoinSceneBand,
   ChangeGenre), relationship weekly tick, member departure check,
   weekly upkeep deduction, genre switch personality fallout.
7. **`app.rs` / `render.rs` [MODIFY]** — setup flow, practice picker,
   band management modal, charts modal, band panel redesign.
8. **Tests** — musician generation, genre score derivation, relationship
   drift, identity transitions, chart behavior across transitions,
   uncapped population.

---

## §9 — The Rockstar Life: Addiction, Happiness, Vacations, the Manager

These four are the heart of the original 1989 game — where the band was
flavor text, *these* were the mechanics — restored here on top of systems
the original never had (the timeline, the scene, real band members).
Independent of §1–§6 except where marked: safe to build before, after, or
alongside the Musician work. Everything follows existing patterns: picker
modals, offer streams, seeded action-stream RNG, `#[serde(default)]` on
every new field so old saves keep loading.

### §9.1 — Addiction with teeth

`drug_addiction` / `alcohol_addiction` already exist on `Player` and
drain health weekly — and nothing else. In the original, drugs were a
*choice*, and they could kill you. Restore both:

- **Offers become choices.** `DrugOffer` currently rolls
  `rng.gen_bool(0.3)` and decides *for* the player (`events_apply.rs`).
  Make it a modal: Accept (+20 energy now, +addiction, −health) /
  Refuse. This is the first of the README's planned "player choices in
  events" — build the event-choice modal once, reuse it for later events.
- **Cravings.** While `drug_addiction > 40`, refusing costs stress +10
  and energy −10. Saying no gets mechanically harder.
- **Overdose.** Weekly roll on the action stream while
  `drug_addiction > 70`: ~3% collapse — health halved, hospital bill,
  2 weeks lost, news line. Above 90 the collapse can kill: game over,
  like 1989. Telegraph it: the doctor warns at 50+, the log darkens at 70+.
- **No-shows.** Above 60, each gig/tour week risks a missed show — fee
  lost, fame −2, `reputation.live_performance` −5.
- **Rehab** (new action, lives in `actions/rest.rs`): $2,000, 6 weeks,
  addictions → 10, stress → 0, happiness +20. Idle fame decay applies
  throughout. The press reacts through the timeline — a shrug in 1972,
  a tabloid feeding-frenzy in 1988. An era hook the original couldn't have.

### §9.2 — Happiness

**Partially shipped in v0.6** (`docs/DESIGN-v0.6-life-cycle.md` §A):
`happiness: u8` on `Player` (serde default 60) exists, drains with
stress, and the quality multiplier — `0.8 + happiness/500`, range
0.8–1.0 — is already wired into songwriting and recording (`chemistry_mult`
from §4 below is still a Musician-cycle addition on top of it). What v0.6
did NOT build, still open:

- **Charting happiness bonus** (+12 at #1 down to +3 at #10) and one-shot
  firsts (first single / album / tour, +10 each) — v0.6's happiness gains
  are only from great/transcendent shows and vacations-equivalent
  (Take a Break); charting itself doesn't move happiness yet.
- **Low happiness feeding addiction** (below 30, craving costs double,
  DrugOffer reads as self-medication) — blocked on §9.1, which is
  entirely unbuilt.
- **Floor:** 4 consecutive weeks at 0 → a forced, unpaid month off
  (auto-vacation) — not implemented; nothing currently reacts to
  happiness bottoming out.

### §9.3 — Vacations

Replace the flat Take a Break with a picker (venue-picker pattern) —
the original's escalating menu, cheap weekend to world cruise:

| Tier | Cost | Weeks | Effect |
|------|------|-------|--------|
| Seaside weekend | $50 | 1 | energy full, stress −20, happiness +5 |
| Country retreat | $400 | 2 | health +30, energy full, stress −50, happiness +12 |
| Mediterranean holiday | $2,000 | 4 | health/energy full, stress → 0, happiness +25 |
| World cruise | $12,000 | 10 | everything reset, happiness → 100, addictions −20 |

Idle fame decay already applies to every quiet week, so the trade-off
costs no new code: the cruise is the superstar's move (fame to burn,
$12k is nothing), while a struggling act can only afford the weekend it
also can't afford fame-wise. `GameAction::TakeBreak` becomes
`Vacation(usize)` — actions aren't persisted in saves, so the enum is
free to change. Destination table in code first; graduate to
`data/vacations.txt` if customization is wanted.

### §9.4 — The Manager

The deal-offer stream pattern, applied to a person:

```rust
pub struct Manager {
    pub name: String,
    pub cut: f32,   // 0.10–0.25 of ALL income
    pub skill: u8,  // 0-100: how much they actually do for you
    honesty: u8,    // hidden. Low = the Allen Klein experience.
}
```

- Managers scout you the way labels do (fame + catalog buzz); offers
  expire on the same beat.
- Benefits scale with `skill`: venues open one prestige tier early, deal
  offers arrive with +2–5pp royalty, marketing at 25% off, tour travel
  costs reduced.
- `honesty` is **hidden**, rolled at generation. Low-honesty managers
  skim: a slice of income vanishes silently; each week a small chance
  the skim is discovered — back-pay gone, firing scene, tabloid news,
  big happiness hit. Period-perfect, and mechanically a gamble on an
  offer you can't fully vet.
- Firing: settlement ≈ 8 weeks of their average take; 12-week cooldown
  before new offers arrive.
- v1 is player-only (§4's "external relationships" deferral stands);
  scene bands don't need visible managers.

### §9.5 — Build order & testing

Addiction, vacations, and the manager touch no quality formulas — safe
to build **now**, before §1–§6. Happiness *effects* land with the §2
quality rewrite (the stat itself can ship earlier, inert). Add two bots
to the balance lab (`sim.rs`): a `junkie-spiral` bot that accepts every
offer and never rests — must reliably die; and a `wellness` bot that
vacations optimally — must survive but chart worse than the grinders.
All new rolls draw from the existing action stream; all new fields take
`#[serde(default)]`, with the save-compat test extended to cover them.

---

## §10 — Later cycles: Music Rights & Era Upgrades

Two enhancement themes captured here so they aren't lost. **Neither is
part of the Musician cycle (§1–§9) nor the v0.7 Money Cycle** — they are
their own future work, listed with the systems they depend on. Do not
start either until its prerequisites have shipped.

### §10.1 — Music Rights & Ownership

**Prerequisite: v0.7 M5 (label recoupment) and M9 (deal lifecycle) must
be shipped first.** A buyout price is fundamentally *the unrecouped
balance plus a premium*, and "a bigger label acquires the release" only
makes sense once labels are active agents with a valuation model — so
this can't be designed coherently until the recoupment ledger
(`RecordDeal.unrecouped`) and the deal-as-relationship model are real
and proven. Building it earlier means building on unfinished
foundations.

The theme is master ownership as a tradeable asset:

- **Who owns a release.** Today a `Release` is implicitly owned by
  whoever put it out. Make ownership explicit (an owner tag: the player,
  or a named label) so it can change hands.
- **Player buys back their master.** From a small/struggling label:
  price ≈ remaining `unrecouped` + a premium scaled by the release's
  sales tail and the label's leverage. Frees future royalties to flow
  100% to the player, at a large up-front cost — the Taylor-Swift
  re-record move, in miniature. Reputation effects lean on M9's
  breach/free-agency rules.
- **A bigger label buys the master out from under a small one.** The
  release keeps selling but now under the acquirer's terms/reach; the
  player may get a better royalty and distribution, or may resent losing
  the relationship — a personality/relationship hook for the Musician
  cycle if it lands first.
- **Label M&A.** A struggling scene label gets acquired; its signed acts
  (including the player, if signed to it) transfer to the acquirer.
  Natural extension of the scene's existing attrition model.
- **Catalog sales.** A label (or the player) sells a back-catalogue
  release for a lump sum, forfeiting its long-tail royalties — cash now
  vs. income later.

Everything derives from the recoupment ledger and the deal model; no new
quality formulas. This is the coherent "labels as a whole" cycle that
one-off buyout features would otherwise orphan.

### §10.2 — Era Upgrades: Music Videos & MTV

**Prerequisite: none hard, but best after the v0.7 marketing/fame work
settles.** The game already has a timeline with eras
(`timeline.rs`, `market_conditions`, `era_genre_modifier`) and a
marketing-campaign system (`MarketingCampaignType`, the buzz model) —
this theme makes *the era itself* unlock new promotional surfaces, so a
1974 career and a 1985 career play differently.

- **Music videos as an era-gated marketing channel.** Before MTV
  (launched 1981 in reality — drive the exact year from the timeline
  data, not a hardcode), a video is a niche promo spend. From the MTV
  era on, a video becomes a powerful, expensive marketing campaign type:
  a large buzz/fame multiplier, gated by the era, with cost scaling to
  production values. A cheap clip vs. a lavish one is the same
  quote-first decision the tour rig picker uses.
- **The MTV rotation as a chart-adjacent surface.** Heavy rotation is a
  fame/reach amplifier the way regional presence (v0.7 §C) is for sales
  — a video in rotation lifts a release's chart presence in the
  territories that carry the channel. Ties into the regional-charts
  model rather than duplicating it.
- **Era-reactive framing.** The press/timeline already reacts to the era
  (the v0.6 rehab example — a shrug in 1972, a frenzy in 1988); a video
  flop or a banned/controversial clip is period-perfect fodder for the
  same era-aware event system.
- **Genre interaction.** Some genres photographed better on early MTV
  (glam, new wave, pop) than others — a natural tie to
  `era_genre_modifier` and the Musician cycle's genre model.

All new fields `#[serde(default)]`; era gating reads the timeline, never
a hardcoded year; any new rolls draw from the existing action stream.

---

## Key design constraints — do not violate

- **`Musician` is the universal type for anyone who makes music.** Player,
  band members, and (when materialized) scene band members all share it.
- **Genre proficiency is derived from abilities, never stored.** No
  `genre_ratings` HashMap. The `ability_weights()` table on `MusicGenre`
  is the single source of truth.
- **Band with empty members = solo.** No `SoloMusician` struct.
- **Relationships are stored only for the player's band.** Scene band
  chemistry is derived from personality on demand.
- **Charts are pure score decay.** No special eviction for any reason.
- **Scene bands stay lightweight until materialized.** Most of the 180+
  bands are just `(name, fame, genre, momentum)`. Rosters appear only
  on interaction.
- **Identity changes cost a week.** No free restructuring.
- **World RNG is injected, not ambient.** (Inherited from HANDOFF.)
