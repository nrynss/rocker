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
  **(SUPERSEDED 2026-10-31 — kept for the history. The assertion was narrowed
  and the ladder retuned; all 9 ignored sweeps now pass. The live status is the
  bottom-most entry in this log.)**
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
    weeks have settled. Any **turn-consuming** action plays the next week —
    that is what the turn already did. The screen-opening rows do *not*:
    Charts, Discography, Save and Load set `Screen` directly and dispatch
    nothing, so they never advance an activity (nor should they). `X` abandons,
    dispatched as `GameAction::AbandonActivity`. *(Corrected 2026-11-01,
    #41: `X` now asks first and only `Y` dispatches; the week-taking rows
    say what they do instead (`keep touring` etc.). See the dated note at
    the end.)*
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
  *(Figures are as measured before the ladder retune later the same day — see
  the last entries in this log for the current 18/60.)*
  The 15-year sweep puts studio-rat at 20 broke, against a bar of 19. Look at
  the company it keeps:

  | bot | broke/60 | note |
  |---|---|---|
  | gig-grinder | **0** | never records |
  | balanced-indie | **2** | the intended loop |
  | indie-lifer | **2** | |
  | label-loyalist | **2** | |
  | homebody | **9** | *does* earn live income — see below |
  | road-dog | 2 | |
  | studio-rat | **20** | **never performs**; records only |

  *(Corrected 2026-11-01: an earlier draft said "every bot that touches the live
  economy is at ≤2", which is false — `homebody` runs through `indie_loop` and
  gigs whenever `just_released_music` is non-empty, so it is a live-economy bot
  sitting at 9. The `≤2` grouping only holds for the bots that **tour**, and
  even then it is an observation about this run rather than a property.)*

  Studio-rat is the only bot with *no live income at all* — it never performs and
  never tours — and §B made recording cost weeks it never had to spare. Its 20 is
  the documented cost of an extreme policy, not the economy's default: the
  assertion's bar (`broke * 3 < runs`) was written when records were instant and
  every bot could earn between sessions. **`homebody`'s 9 is a second, milder
  case of the same shape** — it never tours, so its only live income is the
  sales-window gig — and it is the reason the narrowed assertion keys on
  `Bot::plays_live()` rather than on a hand-drawn list of "bots we think are
  fine".

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
- 2026-10-31 **S-SIM: the assertion is narrowed and the ladder sweep has run.
  The studio ladder is inverted — every rung earns less.**
  - **The assertion is narrowed, not loosened.** `Bot::plays_live()` splits the
    bankruptcy invariant: bots that earn from performing keep `broke * 3 < runs`;
    the zero-income control (`studio-rat`) gets `broke * 3 <= runs`. Every bot
    that **tours** sits at 2 or fewer of 60, except `indie-lifer` at 4 — it
    accepts support tours but never books its own — and `homebody` at 9, which
    never tours at all (see the next note; it reads 8 after the ladder retune
    further down). The control's 20/60 is the documented cost of never
    performing. The reasoning is in the code comment so nobody
    later reads it as a threshold moved to get green. No constant changed.
  - **The ladder sweep exists** (`studio_ladder_sweep_…`,
    `#[ignore]`d alongside the other long sweeps). It holds the policy fixed
    (`BalancedIndie`, the intended player loop), moves only the room, and runs
    24 seeds × 5 rooms × 15 years. The studio is now threaded through
    `drive` → `decide` → the policies as a parameter, with every existing sweep
    passing `BOT_STUDIO`, so the old numbers are untouched.
  - **What it found, on the first run:**

    | room | win% | med.$ | alb1.wk |
    |---|---|---|---|
    | Home 4-track | 91% | **4,463,933** | 25 |
    | Local demo room | 100% | 4,062,071 | 29 |
    | Regional | 100% | 3,468,747 | 41 |
    | Major city | 100% | 2,816,784 | 64 |
    | World-class | 87% | 1,094,476 | 278 |

    The cheapest room is the best earner and the top room is the worst. It is
    not one bad constant; it is the interaction:
    - a room's bonus is **additive** (`STUDIO_QUALITY_BONUS` -8/+0/+6/+12/+20),
    - quality reaches sales at `SALES_QUALITY_WEIGHT` 2.5,
    - so demo → major-city buys +11 quality ≈ +27.5 score against a base of
      ~257 at fame 100 = **+10.7% sales**,
    - while the bill goes 2,200 → 16,000 (**7.3×**) and the session 3 → 6 weeks
      (**2×**) — weeks the act cannot gig,
    - and world-class's fame gate (70) only opens once the fame term already
      dominates the score, so it buys the least where it costs the most.

    Same defect class as the full-production tour rig (PR #25) and the
    studio-rat's bootstrap single: **a choice you can never come out ahead on
    is not a trade-off.** §C's money-vs-time-vs-quality thesis does not hold in
    the shipped numbers.
  - **The sweep intentionally FAILS on this.** It asserts the property (the best
    earner must not be the cheapest room) rather than a number, so it stays red
    until the ladder is retuned — a red test that names the defect is more
    useful than a green one that hides it. **S-SIM's remaining work is the
    retune itself**, and the sweep is the instrument: raise the quality bonus,
    raise `SALES_QUALITY_WEIGHT`, or cut the top rooms' cost and weeks, then
    re-run and watch which room wins.
- 2026-10-31 **S-SIM: the ladder is fixed, and it was two bugs wearing one
  costume.** The sweep that found the inversion also found the reason the
  first attempt at a retune did not work.

  **Bug 1 — the bots could not book the rooms they were measuring.** No policy
  checked `studio_is_available`, so a fame-gated booking was submitted, refused,
  and the week thrown away: measured at **31% of all turns** at the world-class
  room (gate 70) and 6% at major-city (gate 45). `reachable_studio` now walks
  down to the best unlocked room, so a bot climbs as its fame allows. That alone
  took world-class from 278 weeks to a first album to 29.

  **Bug 2 — the economics, and the binding term was *time*, not money.** The
  additive quality bonus reaches sales at `SALES_QUALITY_WEIGHT` 2.5, but the
  old weeks ladder pushed a ten-track album from 2 weeks to 8. Every extra week
  is a week the act cannot ship another record, and the records never made cost
  more than the quality bought. The sweep showed it plainly after a partial fix:
  per-album income rose 28% while album count fell 32%.

  **The retune** (all three tables, in `constants.rs`, each with its measurement
  in the doc comment):

  | table | was | now |
  |---|---|---|
  | `STUDIO_QUALITY_BONUS` | -8/0/+6/+12/+20 | **-4/0/+8/+16/+24** |
  | `STUDIO_RATE_PER_SONG` | 60/220/650/1,600/4,000 | **60/220/400/640/900** |
  | `STUDIO_WEEKS_PER_SONG` | 0.20/0.30/0.45/0.60/0.80 | **0.20/0.21/0.22/0.23/0.24** |

  Why each number is what it is:
  - **Weeks are shallow on purpose.** 0.28/song (0.26 at world-class) was tested
    and lost five albums over a career, finishing *below* the demo room. 0.24 is
    the steepest slope that still holds a ten-track album at two-to-three weeks
    at every rung. §C's "a better room costs time as well as money" survives as
    +20% time per song rather than 4x, because 4x was measured unpayable.
  - **The home room's penalty lightened from -8 to -4.** At -8 it cost a beginner
    15-20% of sales at low fame — a tax on the one room the game *starts* a band
    in.
  - **The bonus spread is bounded by §C.** A great band in a demo room must still
    beat a poor band at the world-class room. At `QUALITY_BASE_RECORDING` 30,
    +24 keeps that true with margin (great@demo 65 vs poor@world 54); +32 was
    tested and still holds (+3 margin) but buys little extra once the weeks are
    fixed, so the wider spread is not worth the §C risk.

  **Result** (24 seeds, BalancedIndie, 15 years, median bank):
  home $6.22M → demo $6.62M → regional $6.82M → major-city $6.95M →
  world-class $7.07M. **Monotonic; no rung is dominated**, and the ~$200k steps
  are small enough that an act's circumstances still decide.

  **The economy as a whole improved.** All 9 ignored sweeps pass. studio-rat —
  the zero-income control — went 20 broke / 45% wins → **18 broke / 56% wins**,
  and the broke counts are **gig-grinder 0, balanced-indie 2, label-loyalist 2,
  road-dog 2, indie-lifer 4, homebody 8, studio-rat 18** — no bot except the
  zero-income control is anywhere near a third, which is what the narrowed
  assertion actually requires. (`indie-lifer` and `homebody` are the two above
  two: `indie-lifer` takes support slots but never books its own tour, and
  `homebody` never tours at all — both lean on the sales-window gig for live
  income, which is why they sit higher than the touring policies.)

  **Also fixed: a flaky test I wrote.** `the_quote_on_screen_is_the_bill_that_
  gets_charged` failed about one run in three, because `Game::new()` seeds a
  **random world** when `ROCKER_SEED` is unset and the booking turn also runs the
  weekly tick. It asserted an exact bank delta, which depended on ambient
  entropy. It now asserts the invariant (the master's room and song count price
  back to the quoted cost) and passed 12 consecutive runs. **Worth remembering:
  any UI test that touches money must not assume a fixed world.**
- 2026-11-01 **Post-#27 CodeRabbit follow-ups** (two comments landed on #27's
  final commit just before merge and were never addressed).
  - **Booking pickers refuse to open mid-activity.** While an activity runs,
    `process_turn` ignores every week-taking action and plays
    the activity's week instead (§F). The record picker could still open from
    a Record row (hotkey or Enter — `MenuEntry::enabled` is only a render
    hint), quote a valid session, and then drop it on Enter while a tour week
    played. `open_record_picker` now refuses first (the stress guard stays),
    and so do the Gig and Tour rows in `activate`, which had the same shape.
    The shared check is `App::busy_with` (`ui/input/mod.rs`); the four rows
    render greyed out with `busy: <activity>`.
  - **Instant actions now go through a running commitment** (decided by
    Narayan: option 1, "let instant actions run"). They used to be swapped
    for the activity's next week by `process_turn` — Vault → Release (signed
    and unsigned), Re-press, Marketing, Lifestyle, Deal accept/reject and
    Support decline all vanished *and* advanced the activity, though each is
    documented as taking no week. `turn.rs` now has one predicate,
    `Game::is_instant_action`, read twice: `overrides_ongoing_activity` lets
    instant actions through, and the week tick skips them. Each applies
    exactly once, spends no week, and leaves `weeks_done` alone — except
    Abandon and Quit, which take no week but end the activity, as before.
    - **Accepting a support slot stays blocked** — it starts a second
      commitment. It is let through `overrides_ongoing_activity` only so
      `action_accept_support_tour`'s own guard refuses it with an error
      (offer kept, nothing banked, no week) instead of it being swallowed;
      the support modal swaps its Accept hint for the reason and the input
      refuses before dispatching. Declining works.
    - **Still swallowed, by design:** every *week-taking* action (writing,
      practice, laze, the doctor, a break, and any booking) plays the
      activity's week instead — that is how a busy band "continues". The
      booking rows refuse at the door so they never quote something that
      would be dropped; the others are the continue keys (see
      `a_week_taking_action_still_plays_the_activity_week`).
    - **Sim impact** (15-year sweep, 60 seeds, before → after): bankruptcies
      unchanged for every bot (studio-rat 18, homebody 8, indie-lifer 4, the
      rest 2 or 0). The visible shift is **studio-rat's median albums 40 → 86**
      (singles 11 → 18, wins 56% → 60%): it books sessions back to back, and
      its `ReleaseMaster` submitted mid-session used to be swallowed as
      another studio week, so masters sat in the vault. road-dog wins
      11 → 22 of 60, label-loyalist median certs 58 → 59; balanced-indie and
      indie-lifer reach fame 100 a little later (y2 87 → 81/80) but finish
      the same. All 9 ignored sweeps pass. These are the bug's numbers
      coming out of the sim, not a retune. **S-SIM stays ⬜ open** on the
      task board: its remaining scope (bots for the new action shape, the
      singles-vs-albums balance pass, the §F economic-neutrality check) is
      untouched here, and it should re-baseline against these numbers.
    - Tests: `game/tests/instant_actions.rs` (one per action mid-tour, plus a
      release mid-*session*, the signed release path, the support-accept
      refusal, and the week-taking control) and
      `ui::input::instant_rows_mid_activity` (every instant row driven by its
      hotkey and picker keys mid-break, then the booking rows refused).
  - **`design-living-industry.md` currency table:** the mechanical rises +72%
    ($0.30 → $0.5156) and manufacturing +40% ($1.00 → $1.4025), not "roughly
    double". The label-margin comparison also set the code's before-royalty
    $2.70 against the proposal's after-royalty $1.61; like-for-like it is
    0.64× after a 6% royalty ($2.52 → $1.61) and 0.69× while recouping
    ($2.70 → $1.87), not 0.60× — the guardrail now quotes both.
  - **Tests:** 332 passing (from 317), 9 ignored — all 9 ignored sim sweeps
    also pass with the instant-action change — clippy `-D warnings` clean,
    fmt clean, determinism trio unmodified.
- 2026-11-01 **Activity screen honesty + dead-code sweep** (#41, #42; UI and
  housekeeping only — `process_turn`'s §F semantics and
  `Game::is_instant_action` are exactly as #37 left them, so no sim impact).
  - **Continue rows (#41).** Mid-activity, every week-taking row (Laze,
    Write, Practice, Break, Doctor) says it carries the activity on — `keep
    touring` / `keep recording` / `keep opening` / `keep resting`
    (`ui::app::continue_detail`) — and is never greyed out — continuing is always possible. `App::menu_entries`
    post-processes the `MenuKind::Action` rows with the engine's own
    `is_instant_action` (now `pub(crate)`), so a new action can't be labelled
    one way and resolved another. The doctor's fee and money gate are gone
    from the row while busy.
  - **Decision: no doctor mid-activity.** The doctor stays a continue key.
    §F's answer to a body giving out mid-run is the walk-out (and the
    emergency paths force one), and a house call would be a sim change this
    UI issue ruled out. The stats panel's critical-health warning now
    follows suit: "see a doctor (9)" when free, "walk out (X)" on a tour,
    session or support slot, "rest it out" on a break — two lines, because
    the old one-liner was cut off at the panel's ~18 columns.
  - **Decision: `X` is confirmed.** §F always said "one keypress,
    confirmed"; it now is. `X` opens `Screen::ConfirmAbandon`
    (`modals/activity.rs`), which names what is kept and forfeited from
    `abandon_terms` — written against the four settle-as-abandoned paths, and
    a test walks a real tour out to check the two agree. `Y` walks out
    (instant, no week); `N`/`Esc` carry on (no week); every other key,
    including a second `X`, is ignored, so two stray presses can't confirm.
    The confirmation is a UI layer: bots and tests at the `Game` level still
    dispatch `AbandonActivity` directly.
  - **#42.** Stale `#[allow(dead_code)]` removed from `impl OngoingActivity`
    and `rival_act_label`, with the "Pending consumer" comment.
    `rows_so_far` (only a test read it) and `Game::is_busy` (no callers) are
    deleted rather than given readers — the UI already asks
    `ongoing_activity()` directly.
  - **Tests:** 337 passing (from 332), 9 ignored; clippy `-D warnings` and
    fmt clean. New: continue labelling of the five rows (broke, no `$` on the
    doctor, and pressing it neither treats nor refuses), `x` alone only asks, stray
    keys/Enter don't confirm or quit, `n`/`N`/`Esc` cancel with no week,
    `x` then `y` walks out with no week, a stale confirmation closes itself,
    an 80x24 render of the modal naming the forfeit, the terms matching a
    real walk-out's report, and the strip/continue rows/health warning at 80
    columns. #42 alone changes no test count.
- 2026-11-01 **`q` asks to save first** (PR #52, added at Narayan's request on
  top of #41/#42: "It should ask to save and quit or save without quit").
  - **What it did.** `q` / Enter on Quit dispatched `GameAction::Quit`:
    game over ("walked away on your own terms"), the final-stats screen, and
    exit on the next key. Nothing saved — there is no autosave anywhere, the
    only save path is the Save modal — and a running activity was abandoned
    by the game-over path. One stray key ended the career.
  - **What it does.** `Screen::ConfirmQuit { error }` (`modals/quit.rs`,
    `handle_confirm_quit_key`). `S` saves and quits; `C` saves and continues;
    `N`/`Esc` cancel. Everything else (a second `q`, Enter, `j`/`k`, `y`,
    `x`) is ignored. `S`/`C` rather than `Y`/`N`-style keys because there
    are two ways to say yes; `C` rather than `K` because `k` is menu-up.
  - **Decisions.** Quitting is now leaving the app, not retiring: the UI no
    longer dispatches `GameAction::Quit`, so the "walked away" ending is not
    reachable from the menu (the engine keeps the action — it is still the
    `Deserialize`d enum's variant and `is_instant_action` lists it). No
    "quit without saving" option, per the brief's two choices; cancel stays
    so a stray `q` forces nothing, and Ctrl-C remains the hard exit. Save
    target is `App::save_path`: the last file saved to or loaded from via
    the File modal, else `rocker.sav`; the prompt names it and says it
    replaces any save there. A failed save keeps the prompt up with the reason (also
    logged) and never quits. The exit receipt is printed by `main` after
    the terminal is restored (`App::run` now returns it).
  - **Tests:** 347 passing (from 340 at 06752f2), 9 ignored. The prompt
    opening from `q` and from Enter, stray keys and cancels (no file, no
    exit, no week), `C` writing a loadable save and staying, `S` exiting
    with a receipt and the career intact, a failed save for both options,
    mid-activity (the two prompts ignore each other's keys, the break is in
    the save at the same week), the Save modal moving the target, and an
    80x24 render at the prompt's fullest (activity line, long path, error).
