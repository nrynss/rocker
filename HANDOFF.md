# Rocker — Studio Cycle Handoff (v0.9)

> **Feature cycle: making a record becomes a project, not a purchase.**
> Pick a format, pick the songs, pick a studio — then you are *in* that
> studio for weeks, one week at a time, with the world still turning
> around you. Singles and albums stop being the same keypress with a
> different constant; recording stops being pressing; the calendar stops
> skipping; and things start going wrong while you work.
>
> Read `docs/DESIGN-v0.9-studio-cycle.md` before claiming anything.
> Numbers marked [tune] there are sim-lab candidates validated in S-SIM,
> not invented per-agent.
>
> **Stacked on `tour/economics-pass`** (PR #25) — this branch targets that
> branch, not `main`. Do not rebase onto `main` until #25 merges.
>
> Prior cycles: `docs/archive/HANDOFF-v0.8-cycle.md` (labels),
> `HANDOFF-v0.7-cycle.md` (money), `HANDOFF-v0.6-cycle.md` (life),
> `HANDOFF-v0.5-cycle.md` (features),
> `HANDOFF-v0.5.1-structure.md` (structure) · Shipped: `CHANGELOG.md` ·
> North star for a later cycle: `FUTURE.md` (Musician).
> **Deferred:** addiction (§9.1), vacations picker (§9.3), manager
> (§9.4), the whole Musician plan (FUTURE §1–§8) — do not implement, do
> not remove the dormant fields.

Baseline at cycle start: **0.8.0 + the tour-economics pass**, `cargo test`
→ **248 passed, 8 ignored**, clippy `-D warnings` clean, fmt clean,
determinism trio unmodified. Cycle closes as **0.9.0**.

---

## Ground rules (every agent)

- **This cycle changes behavior on purpose** — but only *your task's*
  behavior. No drive-by edits outside your Owns; every [tune] number you
  set must match the design doc or carry a Notes line saying what you
  changed and why.
- **Tests:** counts will grow and some assertions will legitimately
  change. Never delete a test to make it pass; adapt it and say so in the
  commit. The determinism trio
  (`same_seed_and_same_choices_replay_the_same_career`,
  `seeded_worlds_are_reproducible_in_the_harness`,
  `worldgen_is_reproducible_per_seed`) must pass **unmodified**.
- **Serde / saves:** every new field `#[serde(default)]`. Never rename
  serialized fields. `saves_from_v0_4_still_load` and the pre-0.5 fixture
  are sacred.
- **RNG is injected, not ambient** — every new roll derives from the
  seeded stream, or the determinism trio will catch you.
- **Done means green**: `cargo test`, `cargo clippy --all-targets -D
  warnings`, `cargo fmt --check`, plus the task's own acceptance line.

Commit prefix: `studio(S-XXX): <short description>`.

---

## Task board

| ID | Task | Size | Prereqs | Owns (exclusive) | Status |
|----|------|------|---------|------------------|--------|
| **S-RECORD** | §A + §C — singles/albums diverge (2–3 vs 8–14 songs, player-chosen, best-N selection, per-song cost, single sales factor) **and** the five recording studios (quality bonus, per-song rate, weeks/song, fame gates, quote) | L | — | `game/studios.rs` (**new**), `actions/studio.rs`, `economy.rs` recording/pressing cost fns, `band.rs` `can_record_*`, recording consts in `constants.rs`, `data/mod.rs` song-count consts | ✅ done |
| **S-TIME** | §F — the calendar never skips. `Game::week` incremented in exactly one place; tours / support tours / breaks become `OngoingActivity` state resolved one week per turn. Tour economics must be *spread*, not changed | L | — | `core.rs`, `turn.rs`, `actions/live.rs`, `actions/business.rs`, `actions/rest.rs` | ✅ done |
| **S-CHARTS** | §E — separate singles and albums charts per territory, with save migration (existing entries → albums board) and the charts modal toggle | M | — | `world/charts.rs`, `regional_charts` on `world/mod.rs`, `ui/render/modals/charts.rs` | ✅ done |
| **S-MASTER** | §D — recording is not pressing. Masters vault, Release action, pressing/distribution move off the record action | M | S-RECORD, S-TIME | `music.rs` `Master`, `band.rs` `masters`, release action in `actions/studio.rs`, vault UI | ✅ done |
| **S-EVENTS** | §G — random events during recording / gigs / tours / support tours, data-driven and activity-gated | M | S-TIME | `events.rs`, `events_apply.rs`, `data/incidents.json` | ✅ done |
| **S-UI** | §H — record picker (format → songs → studio, quote-first), activity screen, discography split | M | S-RECORD, S-TIME, S-MASTER | `ui/render/modals/pickers.rs`, `ui/input/*`, `ui/app.rs`, discography modal | ✅ done |
| **S-SIM** | §I — bots for the new action shape, studio-ladder and singles-vs-albums balance, **calendar-integrity assertion**, §F economic-neutrality check | M | all | `sim.rs` | ⬜ open |

### Wave plan

Waves keep `Owns` disjoint so parallel agents don't collide:

1. **Wave 1** (parallel): S-RECORD · S-TIME · S-CHARTS
2. **Wave 2** (parallel): S-MASTER · S-EVENTS
3. **Wave 3**: S-UI
4. **Wave 4**: S-SIM

`core.rs`'s `GameAction` enum is the one known cross-wave contact point
(S-RECORD reshapes the recording variants, S-TIME adds the activity
machinery). S-TIME owns the file; S-RECORD states the variant shape it
needs in its handoff notes and the coordinator integrates.

---

## Notes

- 2026-07-25 **Cycle opened.** Design doc written from the user's brief
  (`docs/DESIGN-v0.9-studio-cycle.md`), board drawn, stacked on
  `tour/economics-pass`.
- 2026-07-25 **Wave 1 integrated** (S-RECORD, S-TIME, S-CHARTS). All three
  agents were cut off by a session limit with their work uncommitted; the
  worktrees were snapshotted verbatim, verified independently green, then
  merged. Only two conflicts, both trivial (an import in `core.rs`, adjacent
  helpers in `tests/mod.rs`). **282 tests** (from a 248 baseline), clippy
  `-D warnings` clean, fmt clean, determinism trio passing.
  - S-RECORD reshaped `GameAction::{RecordSingle, RecordAlbum}` to
    `{ songs, studio, pressing }`. `pressing` stays for now — the §D split
    that removes it is S-MASTER's, deliberately not done here.
  - S-RECORD also landed a first pass on the pressing picker so the new
    quote is visible; the full format→songs→studio picker is still S-UI's.
  - S-TIME's `action_go_on_tour` no longer takes an `rng` — activity weeks
    draw from a stream that doesn't interleave with the weekly world tick.
    `finish_activity_in_one_sitting` (in `tests/mod.rs`) is the reference
    path the economic-equivalence test measures the spread run against.
  - The balance sweeps were the step all three agents were cut off before.
    Run and reconciled after the merge — see the next note.
- 2026-07-25 **Sim lab reconciled with §F.** Three breakages, each a real
  consequence of the calendar no longer skipping, not a flaw in the sweeps:
  1. Measurement sweeps booked a tour with one `process_turn` and read
     `last_tour_report` — which since §F is one week in and not yet written.
     They now play the commitment out (`play_out_the_commitment`).
  2. road-dog died **12/60**: a commitment owns the turn, so `self_care`
     submitted a doctor/break every week of a tour and had it discarded
     every week. `AbandonActivity` is the one action a commitment yields
     to. `plan_biggest_tour` also refuses a length whose wear would land
     the band under the health floor.
  3. Fixing (2) indiscriminately then killed homebody **28/60** and
     studio-rat **19/60** — a break *is* the treatment, so abandoning one
     cancels the cure and thrashes. Only the road gets walked out of.
  - After: **deaths 0/60 for every bot**. Bankruptcies rise for the two
    rest-heavy bots (studio-rat 8 → 15, homebody 4 → 10) — that is §F
    working as intended: rent now falls due every week a break or tour
    passes, where a multi-week jump charged it once.
  - **For S-SIM (Wave 4):** the cross-bot money picture has moved and is
    *not* yet re-validated — label-loyalist $18.63M, road-dog $16.06M,
    gig-grinder $4.67M, balanced-indie $3.66M, indie-lifer $3.34M,
    homebody $305k, studio-rat $488. The loyalist is still top and the
    hard assertions pass, but §D and §G aren't in yet; re-tune at the end
    of the cycle, not now.
- 2026-07-25 **Wave 2 integrated** (S-MASTER, S-EVENTS). Both agents were
  again cut off by a session limit — this time mid-sweep, with their work
  committed and their trees clean. Each verified green on its own, then
  merged; **no conflicts at all**. **306 tests**, clippy `-D warnings`
  clean, fmt clean.
  - S-MASTER filled the `Recording` seam S-TIME left in `OngoingActivity`.
    A session freezes its outcome at booking for the same reason
    `TourInProgress` does — release quality reads stress/happiness/skill,
    all of which the weekly tick moves, so rolling at the *end* would make
    the same booking depend on how the calendar chopped it up.
  - `GameAction::{RecordSingle, RecordAlbum}` lost `pressing`; it lives on
    the new `ReleaseMaster { master, pressing }`, which is instant — the
    weeks a record costs were the session's.
  - S-EVENTS needed only a 5-line hook in `turn.rs` and 6 in
    `actions/live.rs`; the rest is contained in `events*.rs` and a 484-line
    `incidents.json` extension.
- 2026-07-25 **⚠️ ONE SWEEP ASSERTION IS RED — read before claiming S-SIM.**
  `fifteen_year_careers_never_panic_and_print_the_balance_report` fails on
  `studio-rat went broke 20/60` (the bar is `broke * 3 < runs`, i.e. ≤ 19).
  The other 7 sweeps pass. This is **not** dismissed as noise; it is one run
  over a threshold after a long climb, and the diagnosis is recorded here so
  Wave 4 doesn't start from scratch:
  - **`SINGLE_SALES_FACTOR` 0.35 → 0.55 (fixed, real bug).** §A moved two
    things at once — a single went from one song to a two-song minimum *and*
    its income was cut — so revenue per song fell ~5.7× while the bill
    doubled, and a garage-run single could not recoup its own pressing.
    Measured: the studio-rat shipped 53 singles across 346 weeks with its
    bank flat at ~$100 and 343 unreleased songs it could never afford to make
    an album from. Same defect class as the full-production rig in PR #25 —
    a choice you can never come out ahead on isn't a trade-off. This took
    broke 36 → 26 and median fame 7 → 22.
  - **`bot_songs` now books the format minimum (fixed).** It was booking the
    *player-facing* default (10 for an album); pre-§A an album was exactly
    `MIN_ALBUM_SONGS`, so the minimum is what keeps a bot comparable to its
    own pre-cycle baseline — and §B prices a session in weeks as well as
    dollars. Took broke 26 → **20**, fame 22 → **62**, peak 87, deaths 0,
    wins 15% → 45%.
  - **Tried and reverted:** throttling the bootstrap single until the album
    was within reach. It made things *worse* (26 → 34) — for a bot that never
    plays live the singles are the only income there is. The comment in
    `studio_rat` records this so it isn't retried.
  - **Open question for S-SIM, deliberately not answered by more constant
    nudging:** the studio-rat never performs, and §B means it now spends ~3
    weeks per album in the studio with no live income where a record used to
    be instant. Its output roughly halved, and 8 → 20 bankruptcies is about
    what that predicts. That may be the *intended* difficulty of "records
    take time" biting an extreme policy, in which case the invariant — written
    when records were instant — is what should move, not the economy. §I
    already calls for a proper singles-bot / album-bot pair; decide it there,
    with the whole cycle in place, rather than tuning global constants to
    rescue one bot.
- 2026-10-31 **S-UI landed — the picker is reachable at last.** Wave 3 done.
  Before this, `ui/input/pickers.rs` booked `RecordingStudio::default()` and
  the format's default song count on every dispatch: the five-room ladder,
  the song choice and the multi-week session all existed in the engine and
  were unreachable from the UI. The cycle's headline feature was dead code.
  - **Record picker** (§H): `Screen::RecordPicker` carries the three axes.
    Songs spin on ↑↓ over the format's bounded range, studio moves on ←→ along
    `RecordingStudio::ALL`, and the quote panel recomputes
    `Game::quote_recording` every frame — the same deterministic core the
    booking charges from, so the number shown is the number taken. Unaffordable
    rooms and short notebooks render as a message, never a panic, and Enter is
    the only thing that reports an error.
  - **The menu rows open the picker.** `MenuKind::{RecordSingle, RecordAlbum}`
    now call `open_record_picker`; `book_recording` survives as the no-choice
    path. This is the actual bug fix — everything else in this task is the
    screen it opens.
  - **Activity screen** (§H): while a commitment runs, a strip above the menu
    shows what it is, week *n* of *m*, a per-week progress bar, and how many
    weeks have settled. Any menu action plays the next week (that is what the
    turn already did); `X` abandons, dispatched as
    `GameAction::AbandonActivity`.
  - **Discography split** (§H): `Tab` cycles all → singles → albums, and the
    title names the current filter. `discography_rows` now takes the filter;
    an empty filter says so rather than showing a blank list.
  - The two `#[allow(dead_code)]` holding patterns — on `RecordingStudio::ALL`
    and on `RecordingQuote` — are **gone**, because they existed precisely
    until this caller arrived. Every quote field is now read: `release_type`
    was the last one with no reader, and the picker's quote line names it.
  - **Tests:** 315 passing (from 306), 8 ignored, clippy `-D warnings` clean,
    fmt clean, determinism trio unmodified. New coverage: the menu row opens
    the picker without booking, songs/studio navigation and wrap-around, the
    quote-equals-bill invariant, a refusal when songs are short, Esc leaving
    without booking, a render smoke test across the picker's state space, the
    discography Tab cycle, and `X` abandoning an activity (and being inert
    when nothing runs).
  - **Still open: S-SIM.** The balance work is now measurable against a
    reachable picker — the studio ladder is only live once a human can climb
    it, which is exactly why the sim's bots need re-running.
- 2026-10-31 **S-SIM triage: the red sweep is not what it looked like.**
  Before touching a constant, the studio-rat's two headline numbers were
  traced to their sources. Both turned out to be *the bot*, not the economy.

  **1. "studio-rat signed 0 deals in 60 runs" is not a scouting defect.**
  `studio_rat` (`sim.rs:387-433`) has no `pending_deal_offers` branch at all —
  a grep for `deal` in its body returns zero hits. Only `label_loyalist`
  accepts offers (`sim.rs:495`), and `balanced_indie` and `indie_lifer` are
  documented as *deliberately* ignoring them. The scouting market is never
  exercised by those bots, so their deal counts say nothing about the gate.
  A 37-album act failing every fame/catalog/buzz threshold at
  `data/record_labels.json`'s `label_requirements` (major fame 40 / albums 1 /
  singles 3 / buzz 30) was never plausible — and it isn't what happened.

  **2. The 20/60 bankruptcies are a real bot, not a broken invariant.**
  The 15-year sweep puts studio-rat at 20 broke, against a bar of 19. Look at
  the company it keeps:

  | bot | broke/60 | note |
  |---|---|---|
  | gig-grinder | **0** | never records |
  | balanced-indie | **2** | the intended loop |
  | indie-lifer | **2** | |
  | label-loyalist | **2** | |
  | homebody | 9 | |
  | studio-rat | **20** | **never performs**; records only |
  | road-dog | 2 | |

  Every bot that touches the live economy is at ≤2. Studio-rat is the only bot
  with *no live income at all*, and §B made recording cost weeks it never had
  to spare. Its 20 is the documented cost of an extreme policy, not the
  economy's default — the assertion's bar (`broke * 3 < runs`) was written when
  records were instant and every bot could earn between sessions.

  **What this means for S-SIM.** The invariant is measuring the wrong thing:
  "no bot may go broke more than a third of the time" is not a property of the
  economy when one bot is a deliberate zero-income control. The choices, in
  preference order:
  - **Narrow the assertion** to the bots that play the live loop, and give
    studio-rat its own documented threshold (it is a *control*, not a
    contender). This is honest and cheap.
  - **Give studio-rat a way to survive** — it cannot tour by design, so the
    only lever is making the release-only path not bankrupt a fifth of the
    time: cheaper sessions, faster songs, or releasing from the vault sooner.
    This changes the economy to rescue a bot, which is exactly the move
    `HANDOFF.md` has twice warned against.
  - **Retire studio-rat** from the sweep. Loses the only control for "is live
    income necessary?", which is a question worth keeping.

  **Recommendation: narrow the assertion.** Do not tune constants to make a
  zero-income control behave like a working band.

  **Still to do for S-SIM proper:** bots for the new action shape (§I), a
  studio-ladder sweep (every bot still books `BOT_STUDIO = DemoRoom`,
  `sim.rs:291` — the ladder S-UI just made reachable is unexercised by the
  sim), the singles-vs-albums balance pass, and the §F economic-neutrality
  check. The calendar-integrity assertion already exists at `sim.rs:743`.
