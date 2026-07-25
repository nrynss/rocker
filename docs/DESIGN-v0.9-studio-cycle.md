# Design — The Studio Cycle (v0.9)

> Making a record is the one thing this game is about, and it is the one
> thing that costs nothing to decide. Today "Record Album" is a single
> keypress that atomically writes a master, presses a run, buys
> distribution, and hands the calendar back — and a single is the same
> keypress with a smaller number attached. There is no studio to choose,
> no time in it, no difference between a two-song 45 and a fourteen-track
> double LP beyond a constant, and no way for anything to go wrong while
> you are in there.
>
> This cycle makes **the record a project instead of a purchase**: you
> pick a format, pick the songs, pick a studio, and then you are *in* that
> studio for weeks, one week at a time, with the world still turning
> around you.
>
> This is the **decided design** for the v0.9 feature cycle, opened
> 2026-07-25 from the user's brief. The cycle's task board is `HANDOFF.md`
> at the repo root. FUTURE.md (Musician, relationships, solo/band) stays
> the north star for a later cycle.
>
> **Builds on:** `tour/economics-pass` (the tour-economics fix), which this
> cycle is stacked on. §F below generalizes the calendar handling that
> pass left alone.
>
> **Deferred explicitly (unchanged):** addiction (FUTURE §9.1), vacations
> picker (§9.3), the Manager (§9.4), and the whole Musician plan
> (FUTURE §1–§8) stay dormant. Do not implement them here; do not remove
> the dormant fields.

Numbers marked **[tune]** are starting values to be validated in the
`sim.rs` balance lab (task S-SIM), not contract. Everything else is
decided design.

Baseline at cycle start: **0.8.0** + the tour-economics pass, `cargo test`
→ **248 passed, 8 ignored**, clippy `-D warnings` clean, fmt clean,
determinism trio unmodified. Cycle closes as **0.9.0**.

---

## §A — Singles and albums become different records

### The problem in the current code

`ReleaseType::{Single, Album}` exists, but almost nothing downstream
distinguishes them:

- A single takes **exactly one song** (`can_record_single` is
  `!unreleased_songs.is_empty()`); an album takes exactly
  `MIN_ALBUM_SONGS` (8). The player never chooses.
- Cost is two constants (`SINGLE_RECORDING_COST` 300,
  `ALBUM_RECORDING_BASE_COST` 3000) times an era modifier. Song count
  does not enter.
- Both land on the same charts, and the discography shows one list.

### The fix: a format is a decision with a shape

**Song counts are a bounded player choice.**

| Format | Songs | Default | Notes |
|--------|-------|---------|-------|
| Single | 2–3 | 2 | A-side + B-side. Below 2 is not a single. |
| Album | 8–14 | 10 | |

- `MIN_SINGLE_SONGS = 2`, `MAX_SINGLE_SONGS = 3`,
  `MIN_ALBUM_SONGS = 8` (unchanged), `MAX_ALBUM_SONGS = 14`. [tune on the
  maxima only — the minima are the user's decided design]
- The picker offers the range the band's `unreleased_songs` can actually
  fill, and refuses below the minimum with the shortfall named.
- **Which** songs: the best-quality N, not the last N written. Today
  `get_selected_songs_for_release` drains the *tail* of the vector, which
  silently spends the newest songs regardless of quality. Selecting the
  best N makes the writing→recording loop legible and makes "write more,
  then pick" a real strategy.

**Cost scales with what you are making.** Recording cost becomes
`per-song studio rate × songs × era modifier`, so a 14-track album costs
what fourteen tracks cost and a 2-song single is genuinely cheap. The
flat `SINGLE_RECORDING_COST` / `ALBUM_RECORDING_BASE_COST` pair retires.

**A single earns less, and differently.** A single's initial sales score
and catalogue tail are scaled by `SINGLE_SALES_FACTOR` [tune: 0.35]
against an album's. It is the cheap, fast, low-ceiling release — the way
you buy chart presence and fame between albums, not the way you get rich.

---

## §B — Recording takes time

**A record is booked, not bought.** Recording occupies real weeks:

`weeks = ceil(songs × studio.weeks_per_song)`, clamped to
`[1, RECORDING_MAX_WEEKS]` [tune: 12].

At the §C rates that is roughly: a 2-song single in a demo room, 1 week;
a 10-track album at a good studio, 4–5 weeks; a 14-track album at the top
room, 8+ weeks. Every one of those weeks is a week the calendar actually
plays (§F) — the band is unavailable, the world keeps turning, and the
sales tail of everything already out keeps ticking.

**Money is due at booking**, in full, up front — you cannot start a
record you can't finish. (A later cycle may add walking out of a session;
not here.)

---

## §C — Five recording studios

The studio is the quality lever, and it is priced in both currencies —
money **and** time.

| # | Studio | Quality bonus [tune] | Rate / song [tune] | Weeks / song [tune] | Fame gate |
|---|--------|---------------------|--------------------|---------------------|-----------|
| 1 | Home 4-track | −8 | $60 | 0.20 | — |
| 2 | Local demo room | 0 | $220 | 0.30 | — |
| 3 | Regional studio | +6 | $650 | 0.45 | 20 |
| 4 | Major city studio | +12 | $1,600 | 0.60 | 45 |
| 5 | World-class room | +20 | $4,000 | 0.80 | 70 |

- The quality bonus is added inside `calculate_release_quality`, before
  the existing clamp — it shifts the whole distribution, it does not
  multiply it, so a great band in a demo room still beats a poor band at
  the world-class room.
- Studio 2 is the neutral baseline: it reproduces today's numbers, so a
  player who always picks the default sees the old game.
- Fame gates keep the top rooms aspirational without ever *pricing* by
  fame — the same lesson §A of the v0.7 doc learned about tours. Same
  studio + same songs = same quote, at any fame.
- **The quote comes first**, exactly as the tour picker does: cost,
  weeks, and the projected quality band, shown before booking.

---

## §D — Recording is not pressing

### The problem in the current code

`action_record_single` / `action_record_album` do all of this atomically:
select songs → charge recording **+ pressing + distribution** → build a
`Release` → push to `just_released_music`. A record is manufactured and
in shops the instant it is written. There is no master, and "record"
means "release".

### The fix: a master, then a release

Recording produces a **master** — a finished record the band owns and has
not yet put out:

```text
RecordProject (in progress, §F)  ->  Master (in the vault)  ->  Release
        weeks of studio time            player decides when      pressing +
        + studio + songs                and how big a run        distribution
```

- `Band::masters: Vec<Master>` (`#[serde(default)]`), each carrying its
  songs, release type, release quality, studio used, and the week it was
  finished.
- A separate **Release** action takes a master, chooses the pressing tier
  and distribution channel (the existing pickers), charges *those* costs,
  and produces today's `Release`. This is where `plan_pressing` and
  `plan_distribution` move to.
- A signed act's label still presses on release, as today.
- **Sitting on a master is legal** and sometimes right: finish the record,
  tour behind the last one, release into a better market.

---

## §E — Separate singles and albums charts

Regional charts (v0.8 §C) gain a **format axis**: every territory keeps a
singles Top 100 and an albums Top 100, ranked and decayed independently
with the existing ramp/decay machinery. A single competes with singles.

- `regional_charts` becomes keyed by `(ChartRegion, ReleaseType)`.
- **Save compatibility:** existing `regional_charts` entries migrate into
  the albums board of their region (the pre-cycle economy was
  album-dominated), the same way `migrate_legacy_charts` seeded Local.
  The pre-0.5 fixture must still load.
- The charts modal gains a singles/albums toggle.
- `peak_chart_position` on a `Release` now means "peak on its own format's
  board", which is what a chart peak has always meant in the real world.

---

## §F — The calendar never skips

### The problem in the current code

`process_turn` advances **one** week, but three actions reach in and add
more behind its back:

- `actions/live.rs`: `self.week += weeks` (a whole tour, instantly)
- `actions/business.rs`: `self.week += offer.weeks` (a support tour)
- `actions/rest.rs`: `self.week += BREAK_WEEKS - 1`

Everything a week is supposed to do — sales ticks, label moves, chart
decay, deal offers, events, upkeep — happens **once** for the whole jump.
A four-week tour is four weeks of rent and zero weeks of everything else.

### The fix: one week, always

**`Game::week` is incremented in exactly one place — `process_turn`.**
No action may touch it. Multi-week commitments become *state*:

```rust
pub enum OngoingActivity {
    Recording { /* project: songs, studio, format, weeks_total, weeks_done */ },
    Tour      { /* region, rig, weeks_total, weeks_done, momentum, rows */ },
    SupportTour { /* offer, weeks_total, weeks_done */ },
    Break     { /* weeks_total, weeks_done */ },
}
```

- Starting one charges its up-front cost and resolves **its first week**.
- While `ongoing_activity` is `Some`, `process_turn` ignores the
  submitted action and resolves **one more week** of the activity, then
  runs the normal weekly tick. The player sees the week pass with a
  progress line; the UI offers "continue" rather than a menu.
- On the final week the activity settles (tour report, master finished,
  etc.) and clears.
- **A tour's shows now resolve five per week** rather than all at once,
  which is what makes §G possible at all.
- **The economics must not change.** A tour's cost, pot, and per-show
  math are the tour-economics pass's, untouched; they are merely
  *spread*. S-SIM asserts a tour's total money delta is within rounding
  of the pre-cycle value for the same seed and booking.
- `#[serde(default)]` on `ongoing_activity`; a save mid-activity resumes.

**Abandoning.** A tour or a session can be walked out of (one keypress,
confirmed): you keep what resolved, you forfeit the rest and the money.
Health/stress emergencies (`process_turn`'s existing death/burnout paths)
force it.

---

## §G — Things go wrong while you work

Random events currently fire only on the weekly tick between actions, so
a four-week tour could never have anything happen *during* it. With §F
resolving week by week, activities get their own event taps:

| Activity | Examples [tune] |
|----------|-----------------|
| Recording | producer clashes (quality −), a breakthrough take (quality +), gear failure (cost/week +), a song rewritten mid-session |
| Gig | equipment failure, a scout in the crowd, a fight, a legendary night |
| Tour | van breaks down, a member ill (shows cancelled), a riot, a sold-out surprise, press pickup (regional fame +) |
| Support tour | headliner friction, poached crowd, a co-headline upgrade offer |

- Events are drawn from `data/incidents.json` (extended, not replaced) so
  content stays data-driven, and are gated by an `activity` field so a
  van breakdown cannot fire in the studio.
- **Determinism is non-negotiable.** Activity events roll from the same
  seeded stream every other roll uses. The determinism trio must pass
  **unmodified**.
- Frequency is per-week and modest [tune: ~12%/week], with the existing
  `last_event_week` cooldown honoured so a tour is not a slot machine.

---

## §H — UI

- **Record picker**: format → songs (bounded spinner) → studio, with a
  live quote (cost, weeks, projected quality band) — the tour picker's
  quote-first pattern, reused.
- **Vault**: masters awaiting release; the Release action starts here.
- **Activity screen**: while an activity runs, the main screen shows what
  it is, week *n* of *m*, what happened this week, and "continue" /
  "abandon".
- **Charts modal**: singles/albums toggle.
- **Discography**: split into singles and albums.

---

## §I — Sim lab, bots, and tests (S-SIM)

New/changed bots and the properties they pin:

- **Every existing bot must keep working** — the action set changes shape,
  so `indie_loop`, `road_dog`, etc. all need the record→release split and
  the continue-activity path.
- **A singles bot** (cheap, fast, frequent) vs **an album bot** (slow,
  expensive, high ceiling): neither should dominate; the album path
  should win on money over a career, the singles path on fame velocity.
- **Studio tiers must ladder**: a better room must be worth its money for
  an act that can afford it, and must not be worth it for one that
  cannot — the same "is this choice ever the best choice?" test the
  tour-economics pass introduced for rigs.
- **Calendar integrity**: over a full career, `weeks elapsed ==
  process_turn calls that consumed a week`. This is the assertion that
  makes §F true rather than aspirational.
- **Economic neutrality of §F**: same seed, same tour booking, same total
  money delta as before the split.

---

## Task board

Lives in `HANDOFF.md` at the repo root, as every cycle does.
