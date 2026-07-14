# Design — The Life Cycle (v0.6)

> Bars, shows, fame gravity, and data-driven incidents. This document is the
> **decided design** for the v0.6 feature cycle, agreed in discussion on
> 2026-07-14 — now shipped as 0.6.0, including the L12 follow-up (derived
> live skill growth). The cycle's task board is archived at
> `docs/archive/HANDOFF-v0.6-cycle.md`. FUTURE.md (Musician, relationships,
> solo/band) stays the north star for the next cycle; where this doc
> overlapped FUTURE §9 (happiness), this doc won.
>
> **Deferred explicitly:** the drug/addiction system (FUTURE §9.1) is *not*
> in this cycle. The `drug_addiction` / `alcohol_addiction` fields stay
> serialized but dormant. Vacations picker (§9.3) and the Manager (§9.4)
> also wait.

Numbers marked **[tune]** are starting values to be validated in the
`sim.rs` balance lab, not contract. Everything else is decided design.

---

## §A — The four bars

The original 1989 game ran on happiness, creativity, alertness, and health.
We keep that engine with **stress in the alertness slot** — stress is the
better primitive: it is the *cause*, the other bars are the *effects*.

| Bar | Range | Role |
|-----|-------|------|
| **Health** | 0–100 | Survival. 0 = death (unchanged game over). |
| **Stress** | 0–100 | The pressure valve. Work raises it; it drains everything else. |
| **Happiness** | 0–100 | The career's mood. Fed by success, drained by stress. |
| **Creativity** | 0–100 | The tank songs are written from. Fed by great shows and rest, drained by stress and overwork. |

**Energy is removed as a mechanic.** The serialized `energy` field stays on
`Player` (never rename serialized fields) but nothing reads it. Action
guards that checked energy now check stress/health instead.

### The loop

Work hard → stress up → happiness and creativity sag → shows and songs get
worse… **unless the work is landing**, in which case great shows and
charting records refill the very bars the grind drains. Success sustains
the machine; failure while grinding spirals. That is the game.

### Weekly tick (runs in the turn pipeline)

- **Stress:** passive release **−3/week** [tune]. Raised by actions (below)
  and by bad outcomes: a flop **+15**, broke (money < 0) **+5/week** [tune].
- **Happiness:** drains **−(stress / 25) per week** (0 to −4) [tune].
  Gains: chart entry **+12 at #1 scaling to +3 at #40** [tune]; a tour that
  went very well (see §B) **+8**; first-time milestones (first single,
  first album, first tour) **+10** each, one-shot.
- **Creativity:** while stress > 40, drains **−(stress − 40) / 20 per
  week** (0 to −3) [tune]. Gains: each *great* show **+2**, each
  *transcendent* show **+3** (§B verdicts); a well-received tour **+5**;
  a lazing week **+3**; inspiration incidents (§F).
- **Health:** existing stress-driven decay stays (`stress / 20` per week —
  the dormant addiction terms contribute 0). **Touring wears harder**:
  **−4 per tour week** [tune], replacing today's 15 %-chance-of-−10.
  **Excessive lazing** also wears: lazing streak > 4 consecutive weeks →
  **−1/week** [tune] — turtling is safe for months, not years.

### Action costs (energy guards → stress economy)

| Action | Old energy rule | New rule |
|--------|-----------------|----------|
| Play gig | needs 30, costs 30 | blocked if stress ≥ 85 or health < 20; **+8 stress** [tune] |
| Tour | needs 40, costs 40, +30 stress | blocked if stress ≥ 70 or health < 30; **+12 stress per tour week** [tune] |
| Write song | costs 20 | blocked if stress ≥ 90; **+5 stress** [tune] |
| Record | costs 15 | blocked if stress ≥ 90; **+8 stress** [tune] |
| Laze | +20 energy, −10 stress | **−15 stress**, +3 creativity; health wear on long streaks (above) |
| Take a break | full reset | stress → 0, +10 happiness, +10 creativity, +30 health (as today) |
| Doctor | +20 health | unchanged |

### Writing consumes creativity — but only when forced

Writing does **not** drain creativity in the healthy case. It drains when:

- **Writing too much:** 3rd and every subsequent *consecutive* writing week
  → **−5 creativity/week** [tune] (track a `writing_streak`, reset by any
  non-writing action).
- **Writing under stress:** stress > 50 at the moment of writing →
  **−(stress − 50) / 5 creativity** [tune].

So the natural rhythm is the original's: write → tour it → the shows
refill the tank → write again. Grinding out an album in one stressed
sitting empties the well.

### Quality formulas

- **Songwriting** = base 30 + **creativity / 4** (0–25, replaces the old
  energy/stress bonus) + skill/genre terms as today + random variation,
  all × **happiness multiplier `0.8 + happiness/500`** (0.8–1.0, from
  FUTURE §9.2).
- **Recording** = base 30 + band-skill term as today + condition penalty
  if stress > 70 (−10) [tune], × happiness multiplier.

---

## §B — Per-show analysis (gigs and tours)

Every concert — one-off gig or tour stop — resolves **individually** with
two numbers. **5 shows per tour week** (decided). A 3-week tour = 15 shows.

### Reception (0–100): how the crowd took it

```
reception = band_base                    # dominant term
          + condition                    # stress/health penalties
          + era_fit                      # genre-era modifier, scaled ±10
          + variance                     # rng, −10..+10
          + creativity_upside            # rng, 0..=creativity/5 (inclusive; 0 at creativity 0)
```

- `band_base` = **0.7 × average member skill + 0.3 ×
  reputation.live_performance** [tune]. This is the dominant term by
  design: **a band with 100 % rating and 0 creativity can still be
  exceptional.** Creativity is *never* a multiplier — it widens the upside
  tail (an inclusive roll of 0 to +20 at creativity 100; exactly 0 at
  creativity 0, never an empty/panicking range), making transcendent
  nights more likely. A tight, uninspired band is reliably excellent; an inspired one
  catches fire.
- `condition`: stress > 70 → −10; health < 40 → −10 [tune].

**Verdicts:** < 40 *rough night* · 40–69 *solid* · 70–84 *great* ·
≥ 85 *transcendent*. Great and transcendent shows feed creativity (§A);
transcendent also +2 happiness on the spot.

### Box office: tickets and money

- One-off gig: existing venue attendance model
  (`(fame+10)/(prestige+10)` ratio × capacity) becomes the per-show base,
  **× momentum** (below). Earnings per the existing payment formula.
- Tour stop: venue is synthesized per show from the region (capacity drawn
  from population/economic strength [tune]); attendance from fame +
  regional fame × momentum; gross per the existing touring economics,
  paid per show. Total tour money should land in today's ballpark —
  validate in the sim lab, it's a redistribution, not a buff.

### Momentum: word of mouth inside a tour

A rolling multiplier, starts at 1.0. After each show, apply the delta
for its verdict, *then* clamp to **0.85–1.15**. Deltas [tune]:
transcendent **+0.05**, great **+0.03**, solid **0**, rough **−0.05**.
A hot streak sells the back half of the tour; a mid-tour disaster
deflates it. This is
what makes the per-day report worth *reading* — the nights are a story,
not independent rolls.

### The tour report

A new modal (existing `render/modals/` pattern) showing one row per show:
**week/day, city, venue, reception verdict, attendance/capacity, take** —
plus a summary line (average reception, total gross, fame gained). Stored
on `Game` as `last_tour_report` (serde default empty; old saves fine).
One-off gigs produce the same row inline in the log and the report holds
the last gig too.

**Tour verdicts** (drives §A happiness/creativity): average reception
≥ 70 → *"the tour went very well"* → +8 happiness, +5 creativity. Fame
gains stay per-show, small, capped by the existing `live_fame_cap`.

---

## §C — Fame gravity (fully decided)

### New state

- `Band::peak_fame: u8` — highest fame ever reached. Serde default 0; on
  load, lift to current fame.
- `Release::peak_chart_position: Option<u8>` — best chart position ever.
  Serde default None. **A "hit" = a release that charted at all** (i.e.
  `peak_chart_position.is_some()`).

### Floors — earned at peak, permanent, by 15s

| Peak fame reached | Floor (never fall below) |
|---|---|
| under 30 | 0 |
| 30 | 10 |
| 45 | 15 |
| 60 | 30 |
| 75 | 45 |
| 90 | 60 |
| 95+ | 70 |
| 95+ **and** ≥ 10 hit albums/singles | 75 |

### Grace — quiet weeks before decay, by current fame

| Current fame | Grace |
|---|---|
| 0–15 | 2 weeks |
| 16–29 | 4 weeks |
| 30–49 | 8 weeks |
| 50–74 | 13 weeks (3 months) |
| 75–89 | 26 weeks (6 months) |
| 90–94 | 39 weeks (9 months) |
| 95+ | 52 weeks (1 year) |

### The ramp

After grace expires: **−1 the first week, −2, −3, −4, then −5/week**
flat from there. Evaluated weekly against *current* fame's tier. Stops
dead at the floor. Any activity resets the idle streak (and thus the
ramp) to zero. Worked check (decided example — hence the 0–15 bottom
tier, "till 15" inclusive): fame 15, fully idle — nothing weeks 1–2,
then −1, −2, −3, −4, −5 on weeks 3–7 → fame 0 at week 7.

### Comeback

While `fame < peak_fame`, **all fame gains ×2** [only the multiplier is
tune-able], normal beyond. Reclaiming ground is easier than conquering it.

### Activity — three ways to be "in the picture" (idle streak stays 0)

1. **Public action** — gig, tour, support slot, or a release in its
   4-week launch window (today's rule).
2. **On the charts** — while any player release is on the charts, the
   idle clock does not run. Immunity duration is emergent: a #1 album
   protects you for its whole run, a #38 single buys two weeks.
3. **The establishment rule** — at fame ≥ 60, an album or single released
   in the past 52 weeks counts as activity. Below 60, it doesn't — small
   acts must keep showing up.

### Label single-cuts

If signed, with an album that has un-singled tracks, and the band has
gone quiet [tune: idle ≥ 3 weeks, ~10 % weekly chance on the action
stream, max 2 cuts per album, ≥ 6 weeks since any release]: **the label
releases a single from the album on its own volition.** It is a real
release — label pressing, label promo, chartable, royalties at deal
rate — so it feeds activity rules 1–3 automatically. Log makes clear it
wasn't your call: *"📀 Without asking, {label} pulls '{song}' off the
album as a single."* (Relationship friction hook for the Musician cycle.)

---

## §D — Sales tail

Today's long tail is `initial_score / (1 + weeks_since_window)` with an
extra ÷5 — the first tail week sells ~7 % of launch and a modest record
is dead in 3 months. Also, post-launch marketing is recomputed but
**never read** — campaigns started after the launch window burn money.

Changes:

- **Gentler curve:** divisor grows at a third of the rate —
  `1 + weeks_since / 3` [tune].
- **Living tail:** ongoing score =
  `(initial_score + current_marketing × 1.8 + fame × 0.3) / divisor`
  [tune] — post-launch marketing now works, and a famous act's catalog
  keeps selling (pairs with §C: charting protects fame, fame sells
  catalog).
- Keep the pressed-copies bound and the trickle cutoff; retune the unit
  divisor in the sim lab so total lifetime income lands sane.

---

## §E — No ending at the top

Reaching `ROCKSTAR_FAME_THRESHOLD` + album count **no longer ends the
game.** It becomes a one-time milestone: celebratory log + a
`rockstar_achieved: bool` on `Game` (serde default false; set once, never
unset). Death (health 0) and broke-and-unknown remain the only endings.
Post-peak decline, comebacks, and the long second act are now playable —
that's what §C's floors and comeback multiplier are for.

---

## §F — Data-driven incidents

**More random incidents, defined in JSON, outside the Rust source.**
New file `data/incidents.json`, loaded through `data_loader.rs` like
`markets.json` / `record_labels.json`.

**Loader contract** (L8 implements; nothing exists in `data_loader.rs`
yet): `GameDataFiles` gains an `incidents_data: IncidentsData` field,
deserialized from `data/incidents.json` at startup alongside the other
JSON files, with load-time validation — non-empty incident list, every
`weight` ≥ 1, every effect range `[lo, hi]` with `lo ≤ hi`, unique `id`s
— failing fast with a clear error like the existing loaders. Selection
goes through an accessor (e.g. `eligible_incidents(&GameState) ->
Vec<&Incident>`) that filters on `conditions`; the weighted pick itself
rolls on the action stream in `events.rs`.

### Schema

```json
{
  "incidents": [
    {
      "id": "backstage_party",
      "category": "social",
      "weight": 3,
      "conditions": { "min_fame": 10, "max_fame": null, "on_tour": null, "signed": null },
      "effects": {
        "stress":     [-15, -5],
        "happiness":  [5, 15],
        "creativity": [0, 5],
        "health":     [-5, 0],
        "money":      [0, 0],
        "fame":       [0, 0]
      },
      "message": "🎉 A legendary after-party — the whole scene was there."
    }
  ]
}
```

- `weight` = relative selection weight among incidents whose `conditions`
  match the current game state. `effects` are inclusive ranges rolled on
  the action stream; zero-range fields may be omitted.
- The hardcoded `RandomEvent` enum arms (equipment, media, health, money,
  industry, band member) migrate into the JSON. `DrugOffer` is **dropped**
  from the pool this cycle (system deferred). `EventManager`'s serialized
  shape (`last_event_week`) is unchanged.
- **Cadence up** ("more random incidents"): eligible every week (was
  every 2), **35 % chance** [tune] (was 30 % every other week).
- New content to write alongside the migration: parties, press moments,
  gear failures, venue mishaps, fan encounters, industry gossip —
  incidents that move the four bars, since the bars are now the game.
- **Scope note:** this cycle externalizes *incidents*. The general log
  message catalog (gig lines, deal lines, …) staying in code is accepted
  for now; a follow-up cycle can externalize messages wholesale.

---

## §G — Engineering constraints (unchanged law)

- **Determinism:** all new rolls (per-show, incidents, label cuts) draw
  from the existing seeded streams — per-show rolls on the **action
  stream** inside the action that caused them, world evolution stays on
  the world stream. The determinism tests must keep passing (same-version
  replay is the contract; this cycle may change what a seed produces
  vs 0.5, that's fine).
- **Saves:** every new field `#[serde(default)]` (or default-fn). Never
  rename serialized fields. `tests/fixtures/pre-0.5.sav` must keep
  loading. `energy` and the addiction fields stay serialized, dormant.
- **Quality gates:** `cargo test`, `cargo clippy --all-targets -- -D
  warnings`, `cargo fmt --check` — green on every task completion.
- **Balance validation:** the `sim.rs` bot harness is the referee for
  every [tune] value — add sweeps comparing income/fame trajectories
  before declaring a number final.
