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
| **S-RECORD** | §A + §C — singles/albums diverge (2–3 vs 8–14 songs, player-chosen, best-N selection, per-song cost, single sales factor) **and** the five recording studios (quality bonus, per-song rate, weeks/song, fame gates, quote) | L | — | `game/studios.rs` (**new**), `actions/studio.rs`, `economy.rs` recording/pressing cost fns, `band.rs` `can_record_*`, recording consts in `constants.rs`, `data/mod.rs` song-count consts | ⬜ open |
| **S-TIME** | §F — the calendar never skips. `Game::week` incremented in exactly one place; tours / support tours / breaks become `OngoingActivity` state resolved one week per turn. Tour economics must be *spread*, not changed | L | — | `core.rs`, `turn.rs`, `actions/live.rs`, `actions/business.rs`, `actions/rest.rs` | ⬜ open |
| **S-CHARTS** | §E — separate singles and albums charts per territory, with save migration (existing entries → albums board) and the charts modal toggle | M | — | `world/charts.rs`, `regional_charts` on `world/mod.rs`, `ui/render/modals/charts.rs` | ⬜ open |
| **S-MASTER** | §D — recording is not pressing. Masters vault, Release action, pressing/distribution move off the record action | M | S-RECORD, S-TIME | `music.rs` `Master`, `band.rs` `masters`, release action in `actions/studio.rs`, vault UI | ⬜ open |
| **S-EVENTS** | §G — random events during recording / gigs / tours / support tours, data-driven and activity-gated | M | S-TIME | `events.rs`, `events_apply.rs`, `data/incidents.json` | ⬜ open |
| **S-UI** | §H — record picker (format → songs → studio, quote-first), activity screen, discography split | M | S-RECORD, S-TIME, S-MASTER | `ui/render/modals/pickers.rs`, `ui/input/*`, `ui/app.rs`, discography modal | ⬜ open |
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
