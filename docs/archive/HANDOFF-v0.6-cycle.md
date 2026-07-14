> **Archived record (v0.6) — historical reference only.** This cycle is
> complete (shipped as 0.6.0, including the L12 follow-up); do not claim
> tasks or follow the branch/claim protocol below. Check the repo root
> for whichever `HANDOFF.md` is active for the current cycle.

# Rocker — Life Cycle Handoff (v0.6)

> **Cycle closed 2026-07-14 — shipped as 0.6.0.** Feature cycle: the four bars (health/stress/happiness/
> creativity), per-show concert analysis, fame gravity with peak floors,
> living sales tail, endless post-rockstar play, and JSON-driven incidents.
> **Design is decided** — read `docs/DESIGN-v0.6-life-cycle.md` before
> claiming anything. Numbers marked [tune] there are sim-lab-validated, not
> invented per-agent.
>
> Prior cycles: `docs/archive/HANDOFF-v0.5-cycle.md` (features),
> `docs/archive/HANDOFF-v0.5.1-structure.md` (structure) · Shipped:
> `CHANGELOG.md` · North star for the *next* cycle: `FUTURE.md` (Musician).
> **Deferred:** drugs/addiction (§9.1), vacations picker (§9.3), manager
> (§9.4) — do not implement, do not remove the dormant fields.

Baseline at cycle start: **0.5.1**, `cargo test` → **43 passed, 2 ignored**,
clippy clean, fmt clean. Cycle closes as **0.6.0**.

---

## How agents claim work

Same protocol as the structure cycle — this file is the **single
coordination surface**. Do not start a task that is claimed or whose
prerequisites are not ✅.

### One branch for the whole cycle

**All work lands on `life/v0.6`.** No per-task branches, no parallel task
PRs — the cycle PR is **#18**, already open; it merges once, at cycle
close, when the human says so. Only commits to the shared branch matter.

```text
git fetch origin
git checkout life/v0.6
git pull --rebase origin life/v0.6   # always before claim / before push
# … do one claimed task …
git push origin life/v0.6
```

If `pull --rebase` conflicts in files you do **not** own, stop and
unclaim — do not "fix" another agent's WIP.

### Claim protocol (mandatory)

1. **On `life/v0.6`**, up to date with `origin`.
2. **Read the design doc**, then this whole file, then your task section.
3. **Prerequisites:** every ID in `Prereqs` must be `✅ done` (or `—`).
4. **Claim atomically** — flip the board row in the same commit that
   starts the work: `Status` → `🔒 claimed`, `Claimed by` → agent id.
5. **You own every path under Owns** while claimed. Shared wiring
   (`mod` / `pub use` lines in `mod.rs` files) — minimum diff only.
6. **Done means green** (`cargo test`, clippy `-D warnings`, fmt) plus
   the task's own acceptance line. Flip to `✅ done`, fill `Done` SHA,
   push the shared branch.
7. **Unclaim** if you abort: back to `⬜ open`, clear `Claimed by`, one
   line under Notes saying why.

Commit prefix: `life(L#): <short description>`.

### Ground rules (every agent)

- **This cycle changes behavior on purpose** — but only *your task's*
  behavior. No drive-by balance edits outside your Owns; every [tune]
  number you set must match the design doc or carry a Notes line saying
  what you changed and why.
- **Tests:** unlike the structure cycle, test counts will grow and some
  assertions will legitimately change. Never delete a test to make it
  pass; adapt it and say so in the commit. The determinism tests
  (`same_seed_and_same_choices_replay_the_same_career`,
  `seeded_worlds_are_reproducible_in_the_harness`,
  `worldgen_is_reproducible_per_seed`) must pass **unmodified**.
- **Serde / saves:** every new field `#[serde(default)]`. Never rename
  serialized fields (`energy` and addiction fields stay, dormant).
  `saves_from_v0_4_still_load` and the pre-0.5 fixture are sacred.
- **RNG:** new rolls draw on the existing streams — per-show and incident
  rolls on the **action stream**, world evolution on the **world stream**.
  Never reintroduce `thread_rng` under `src/`.
- **Visibility:** prefer `pub(super)` / `pub(crate)`; keep unit tests in
  `src/game/tests/` with the existing harness.

### Do-not-undo (carried from v0.5 / v0.5.1)

- World + action RNG injection from `world_seed`.
- Pressing/unit economics, label marketing ownership, news-from-state,
  `MusicGenre` as the single genre enum, module structure from v0.5.1.
- Support slots uncapped by catalog fame.

---

## Task board (source of truth)

| ID | Task | Size | Prereqs | Owns (exclusive) | Status | Claimed by | Branch | Done |
|----|------|------|---------|------------------|--------|------------|--------|------|
| **L1** | Stat engine: four bars, weekly tick, stress economy | M | — | `src/game/player.rs`, `src/game/actions/rest.rs`, **new** `src/game/lifestyle.rs`, stat consts in `src/game/constants.rs`, one call-site line in `turn.rs` | ✅ done | sonnet-l1 | life/v0.6 | 936b42b |
| **L2** | Writing & quality rework (creativity-driven, consumption rules, happiness multiplier) | S | L1 | `src/game/actions/studio.rs`, `writing_streak` field on `Game` in `core.rs` | ✅ done | haiku-l2 | life/v0.6 | 56b2b82 |
| **L3** | Per-show engine: reception, box office, momentum; gig + tour rework; report storage | L | L1 | **new** `src/game/shows.rs`, `src/game/actions/live.rs`, `last_tour_report` field in `core.rs` | ✅ done | sonnet-l3 | life/v0.6 | faa854f |
| **L4** | Tour report modal + four-bar panel UI | M | L3 | **new** `src/ui/render/modals/tour.rs`, `src/ui/render/panels.rs`, `src/ui/input/main.rs`, `mod`/`pub use` lines in `ui/render/modals/mod.rs` | ✅ done | sonnet-l4 | life/v0.6 | 6075c3c |
| **L5** | Fame gravity: peak floors, tiered grace, ramp, comeback ×2, activity rules, hit tracking | L | — | `src/game/turn.rs` (visibility + game-over fns), `peak_fame` in `band.rs`, `peak_chart_position` in `music.rs`, chart-position write-back in `economy.rs` (that line only), fame consts in `constants.rs` | ✅ done | opus-l5 | life/v0.6 | 8a7a22b |
| **L6** | Label single-cuts (label releases a single on its own volition) | S | L5 | **new** `src/game/label_moves.rs`, one call-site line in `turn.rs`, single-cut tracking on `Release`/album in `music.rs` | ✅ done | haiku-l6 | life/v0.6 | fb5d0af |
| **L7** | Living sales tail (gentler divisor, marketing/fame-responsive, fix dead post-launch marketing) | S | — | `src/game/economy.rs` (catalog-tail section), tail consts in `constants.rs` | ✅ done | haiku-l7 | life/v0.6 | 5da6c79 |
| **L8** | Incidents → `data/incidents.json`: schema, loader, weighted selection, migrate + new content, cadence up | M | L1 | **new** `data/incidents.json`, `src/data_loader.rs`, `src/game/events.rs`, `src/game/events_apply.rs` | ✅ done | opus-l8 | life/v0.6 | d5e1477 |
| **L9** | Endless game: rockstar becomes milestone, not ending | S | L5 | `rockstar_achieved` in `core.rs`, milestone logic in `turn.rs` (game-over fn), `src/ui/render/game_over.rs` | ✅ done | haiku-l9 | life/v0.6 | 59240b0 |
| **L10** | Sim-lab validation: sweeps for [tune] values, income/fame trajectory report, new-system bot coverage | M | L1–L3, L5–L8 | `src/game/sim.rs`, `src/game/tests/**` (new test files), Notes below | ✅ done | sonnet-l10 | life/v0.6 | ae399c4 |
| **L11** | Cycle close: board audit, CHANGELOG, bump 0.6.0, PR to main | S | all | `HANDOFF.md`, `CHANGELOG.md`, `Cargo.toml`/`Cargo.lock` | ✅ done | claude-l11 | life/v0.6 | da70473 |
| **L12** | Fix L10's open finding: live reception never improves — grow `average_member_skill()` via Practice, `reputation.live_performance` via playing shows | S | L1–L3 | `src/game/actions/studio.rs` (Practice member-skill growth), `src/game/actions/live.rs` (`apply_show_verdict_rewards`), live-skill consts in `src/game/constants.rs`, `src/game/tests/shows.rs` | ✅ done | claude-l12 | life/v0.6 | bb1a4dd |

### Parallelism map (waves)

```text
Wave 0 (fully parallel — disjoint owns):
  L1 (stat engine)    L5 (fame gravity)    L7 (sales tail)

Wave 1:
  L2 (writing/quality) ← L1        L3 (per-show engine) ← L1
  L8 (incidents JSON)  ← L1        L6 (label cuts)      ← L5
  L9 (endless game)    ← L5

Wave 2:
  L4 (tour report UI)  ← L3
  L10 (sim validation) ← L1–L3, L5–L8

Wave 3:
  L11 (close)          ← all
```

**Conflict warnings (Owns are the mutex):**

| Pair | Issue |
|------|-------|
| L1 ∥ L5 | Both add one line to `turn.rs` — L1 gets exactly one call-site line for the weekly tick; L5 owns the rest of `turn.rs`. Rebase, keep diffs line-scoped. |
| L5 → L6 → L9 | All touch `turn.rs` — serialized by prereqs. |
| L2 ∥ L3 | Both add a field to `core.rs` — field + accessor lines only; rebase. |
| L5 ∥ L7 | Both touch `economy.rs` — L7 owns the catalog-tail section; L5 only the chart-position write-back line at launch resolution. Coordinate via Notes if unsure. |
| L1 ∥ L2 ∥ L3 | Stress costs for studio/live actions belong to L2/L3 respectively, using L1's constants — L1 does not edit `studio.rs`/`live.rs`. |
| Any ∥ any | Pull --rebase before push. No force-push without the human. |

---

## Task acceptance (one line each; details in the design doc)

- **L1:** four bars move per §A weekly tick; energy unread anywhere under
  `src/` (field intact); laze/break rework in; guards swapped; old saves load.
- **L2:** songwriting uses creativity + happiness multiplier; consumption
  only on streak ≥ 3 or stress > 50; `writing_streak` resets on any
  non-writing action.
- **L3:** every gig/tour resolves per-show (5/tour-week) with reception +
  box office + momentum; `last_tour_report` populated; tour money within
  ballpark of 0.5.1 in the sim lab; great shows feed creativity.
- **L4:** report modal readable for a 20-show tour; main panel shows the
  four bars (energy gone from UI); no other visual redesign.
- **L5:** floors/grace/ramp/comeback exactly per §C tables; charting and
  establishment rules freeze the idle clock; `peak_chart_position`
  recorded at launch resolution; worked example (fame 15 → 0 at week 7)
  encoded as a test.
- **L6:** cuts only when signed + un-singled album tracks + quiet; caps
  respected; the cut is a real release (chartable, royalties, activity).
- **L7:** post-launch marketing measurably moves catalog sales in a test;
  tail decays per §D; lifetime income sane in sim lab.
- **L8:** zero incident content hardcoded in Rust; conditions + weights +
  effect ranges honored; DrugOffer absent; cadence per §F; determinism
  tests green.
- **L9:** hitting the thresholds logs the milestone once, sets the flag,
  game continues; death/broke endings unchanged.
- **L10:** every [tune] value either confirmed or adjusted with a Notes
  line; sweep comparing 0.5.1 vs 0.6 fame/income trajectories pasted below.
- **L11:** board all ✅, CHANGELOG 0.6.0 written (player-facing notes this
  time — it's a feature cycle), version bumped, cycle PR #18 marked
  ready for the human to merge.

---

## Shared files cheat sheet

| Path | Who may touch |
|------|----------------|
| `src/game/turn.rs` | L5 primarily; L1/L6/L9 one call-site line each (serialized by prereqs where listed) |
| `src/game/core.rs` | L2, L3, L9 — field + accessor lines only |
| `src/game/constants.rs` | L1 (stat consts), L5 (fame consts), L7 (tail consts) — separate blocks, rebase |
| `src/game/economy.rs` | L7 (catalog tail); L5 (chart write-back line only) |
| `src/game/music.rs` | L5 (`peak_chart_position`), L6 (cut tracking) — serialized by prereq |
| `src/ui/**` | L4 only |
| `data/**` | L8 only |
| `docs/DESIGN-v0.6-life-cycle.md` | nobody edits mid-cycle; design changes go through the human |
| `FUTURE.md`, `tests/fixtures/**` | nobody |

---

## Verification cheatsheet

```bash
cargo test
cargo clippy --all-targets -- -D warnings
cargo fmt --check
# contracts:
cargo test same_seed_and_same_choices_replay_the_same_career
cargo test seeded_worlds_are_reproducible_in_the_harness
cargo test saves_from_v0_4_still_load
cargo test worldgen_is_reproducible_per_seed
# balance lab (ignored sweeps, run locally when tuning):
cargo test --release -- --ignored
```

---

## Notes (agents append one-liners here)

_Format:_ `YYYY-MM-DD L# <claimed|done|unclaimed> by <agent> — <one line>`
- 2026-07-14 L7 done by haiku-l7 — living tail per §D; gate green (44 passed / 2 ignored); no existing tests needed adapting; integrated by coordinator as 5da6c79
- 2026-07-14 L5 done by opus-l5 — full §C fame gravity; adapted `idle_weeks_erode_fame_after_a_grace_week` to the tiered model; worked example + floor + comeback + establishment tests added; gate green (48 passed / 2 ignored). Coordinator follow-up 6597a87: `gain_fame_capped` so comeback ×2 respects live caps.
- 2026-07-14 L5 known edge (deferred to L10): ramp onset can skip steps when a very-high-fame act decays across a tier boundary (grace shrinks while idle_streak keeps counting) — a strictly gentle onset needs a serialized decay counter on `Game` (core.rs, L2/L3/L9 territory). Low-fame/worked-example cases ramp smoothly.
- 2026-07-14 L1 done by sonnet-l1 — four bars + lifestyle tick per §A, all [tune] values as specced; energy unread in owned files (studio/live/business/events/sim/ui reads finalize in L2/L3/L4/L10); adapted `a_break_is_a_real_break`. Coordinator follow-up 528f971: gig-grinder sim expectation restored (L5's activity rules + grace re-saturate the cap in the merged tree). ui/app.rs "Laze Around" menu copy fixed by coordinator (L4 unclaimed at the time); remaining energy-guard details in app.rs swap with L2/L3.
- 2026-07-14 **Wave 0 complete** (L1, L5, L7 ✅). Gate on merged tree: 53 passed / 2 ignored, clippy -D warnings clean, fmt clean, determinism + save contracts green. Wave 1 open: L2, L3, L8 (← L1) and L6, L9 (← L5). Known mid-cycle red (expected): ignored release-sweep `balanced_indie_reaches_the_win_screen…` — bots aren't stress-aware yet; L10's job.
- 2026-07-14 L6 done by haiku-l6 — label cuts per §C (10%/wk, idle ≥3, 6-wk cooldown, ≤2/album, roll drawn last); 8 new tests; necessary `singles_cut: 0` wiring in studio.rs/world Release literals (compile-forced, coordinate at L2 pick); integrated as fb5d0af. Gate: 61 passed / 2 ignored.
- 2026-07-14 L9 done by haiku-l9 — rockstar is a one-time serialized milestone, game runs on; only death/broke end it; win sweep now tracks the flag; 5 new tests; integrated as 59240b0. Gate: 65 passed / 2 ignored.
- 2026-07-14 L2 done by haiku-l2 — creativity-driven songwriting + happiness multiplier + forced-only consumption per §A; studio guards on stress; 6 new tests. Agent worktree had a stale base; coordinator resolved 7-file pick conflicts (union) and removed two dead QUALITY_*_MAX_BONUS consts; integrated as 56b2b82. Gate: 73 passed / 2 ignored.
- 2026-07-14 **Wave 1 complete** (L2, L3, L6, L8, L9 ✅). Gate: 93 passed / 2 ignored, clippy/fmt clean, determinism + save contracts green. `energy` now unread everywhere under src/ (L1 acceptance fully closed). Remaining: L4 (tour report UI, ← L3 ✅), L10 (sim validation), L11 (close).
- 2026-07-14 L3 done by sonnet-l3 — per-show engine per §B (5 shows/wk, reception/momentum/report, guards to stress/health, tour money redistributed not rescaled, old ad-hoc tour rolls folded into momentum + flat wear); coordinator follow-up: gear_stolen min_fame 10 (scripted-contract interaction, L8 precedent), rough-verdict label fix. Ballpark: new-band tour gross ~85% of old (skill-linked box office, by design) — L10 to sweep.
- 2026-07-14 L8 done by opus-l8 — 30 incidents in data/incidents.json (19 migrated + 11 new, DrugOffer dropped), fail-fast loader, weighted pick at 35%/wk on the action stream; RandomEvent enum retired; 3 money incidents gated by min_fame (justified: broke unknowns don't face star-sized bills; also keeps the scripted determinism contract green); dead MAX_ENERGY/EQUIPMENT_REPAIR consts removed; 6 new tests; integrated as d5e1477. Gate: 79 passed / 2 ignored.
- 2026-07-14 L4 done by sonnet-l4 — tour report modal (scrollable, summary footer, empty state, 'r' key), four-bar panel (energy gauge gone), 3 UI tests + render smoke; integrated as 6075c3c. Gate: 96 passed / 2 ignored. Flagged: broke_and_unknown_ending test can flake (unseeded test_game + incidents) — coordinator fixing.
- 2026-07-14 L10 done by sonnet-l10 — full [tune] sweep, **zero retunes needed**; win-sweep "failure" was reporting instrumentation (weeks_to_rockstar now tracked since the milestone is non-terminal); studio-rat practice change measured as a regression and reverted with a documented dead-end; 4 new CI-safe invariants (laze-wear kills at wk ~105; verdict tiers by skill; incident cadence 34%≈35%; ramp containment). Coordinator follow-up: `decay_streak` on Game — ramp onset now gentle across tier boundaries, L10's diagnostic flipped to a regression test. Gate: 100 passed / 4 ignored; all release sweeps green (~46s).

### L10 balance report (15-year sweeps, 60 seeds/bot)

| Bot | Peak fame (med) | Money @ yr15 (med) | Rockstar rate | Died | Broke |
|---|---|---|---|---|---|
| gig-grinder | 82 | $1,143,069 | 0% (never records) | 0 | 0 |
| studio-rat | 100 | −$124 | 65% (med. win yr 6) | 0 | 16 (27%) |
| balanced-indie | 100 | $1,359,582 | 93% — 45/48 by yr 12, median milestone wk 115 (~yr 2.2) | 0 | 4 (7%) |
| label-loyalist | 100 | $10,819,973 | 93% (med. win yr 2) | 0 | 4 (7%) |

Sweep spot-checks: tour gross by skill tier $539/$724/$807 (reception drives box office, no runaway); sales-tail lifetime income $48/$260/$510/$928 by quality tier (monotonic); fame-95 vs fame-20 catalog 5.9× (bounded by pressed copies — not runaway). **Open design finding for the Musician cycle:** live reception can never improve over a career — `average_member_skill()` and `reputation.live_performance` are written nowhere in production code; `Practice` raises only the separate `band.skill` (recording). The Musician cycle's derived-skill rework (FUTURE §1–§2) is the natural home for the fix.
- 2026-07-14 **L11 done — cycle close.** Board audit: all 11 tasks (L1–L10, L11) ✅ with real commit SHAs verified via `git cat-file -t`. Full suite green: `cargo fmt --check`, `cargo clippy --all-targets -D warnings`, `cargo test` → 100 passed / 4 ignored (up from the 43/2 baseline). `CHANGELOG.md` 0.6.0 written (player-facing this time — feature cycle). `Cargo.toml`/`Cargo.lock` bumped 0.5.1 → 0.6.0. Cycle PR **#18** marked ready for the human to merge.

### Cycle summary

Ten agents across three model tiers (Opus: L5, L8 · Sonnet: L1, L3, L4, L10 · Haiku: L2, L6, L7, L9), each in an isolated worktree, integrated serially by the coordinator with a full gate re-run after every pick. Notable coordinator interventions beyond routine conflict resolution: comeback ×2 clamped to respect live-fame caps (L5); a gig-grinder sim assertion restored under combined L1+L5 behavior; the "Laze Around" menu copy fixed while L4 was still unclaimed; `gear_stolen` gated by fame and a doubled "night night" log string fixed (L3 integration); the energy seam fully closed — Practice and support-tour accepts, which no task owned, moved onto the stress economy; two flaky tests deflaked (unseeded `test_game` colliding with weekly incidents and historical events); the fame ramp given its own serialized clock (`decay_streak`) so a falling star's decay onset is gentle across every grace-tier boundary, closing L5's deferred edge case that L10 quantified.

**Open finding for the next cycle** (not a regression, a pre-existing gap L10 surfaced): live-show reception can never improve over a career — member skill and live-performance reputation are set at band creation and written nowhere in production code. The natural fix is the Musician cycle's derived-skill rework (FUTURE.md §1–§2).
- 2026-07-14 L12 done by claude-l12 — closes L10's open finding. `average_member_skill()` now grows via Practice (+1/member/week, alongside the existing `band.skill` growth for recording quality); `reputation.live_performance` now grows per show by verdict (solid +1, great +2, transcendent +3, rough +0), via the existing `apply_show_verdict_rewards` hook — no new RNG draws, all four sacred contracts pass unmodified. 2 new tests (102 total). 15-year balance sweeps re-run clean: win rates, fame trajectories, and bankruptcy rates match L10's report — no regression.
- 2026-07-14 **Cycle re-closed after L12.** This file is being archived to `docs/archive/HANDOFF-v0.6-cycle.md`; FUTURE.md trimmed of what v0.6 shipped. See `docs/archive/HANDOFF-v0.6-cycle.md` for the full record.
