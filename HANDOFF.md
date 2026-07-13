# Rocker — Multi-Track Handoff (v0.5 cycle)

Written 2026-07-13. This replaces the ecosystem-refactor handoff (see git
history for it). The project is healthy: **25 tests pass, clippy is
warning-free**, and the fame/economy rework just landed — live-fame caps,
idle-week fame decay, 4-week breaks, pressing runs with sell-outs, and
label-run marketing.

This document splits the next cycle into **five tracks (A–E) designed for
parallel agents**. Each track lists the files it owns, exact entry points,
acceptance criteria, and its conflict surface. Read *Ground rules* and your
track; skim the others' "files owned" so you don't wander into them.

---

## Track 0 — coordinator prerequisite (NOT parallel)

Before any agent branches:

1. **Commit the working tree.** Everything since `f8e5eb9` is uncommitted.
   Natural split: (1) repo hygiene (dead file, README title, clippy,
   HANDOFF truth-fixes), (2) live-fame caps + gig rebalance, (3) fame
   decay + breaks + pressing economy + label marketing.
2. **Bump `Cargo.toml` to 0.5.0** and change "Current Features (v0.4.0)" in
   README.md to v0.5.0.

Agents branch from the result. One branch per track, named `track/<letter>-<slug>`.

---

## Ground rules for every agent

- `cargo test` and `cargo clippy --all-targets` must be clean before you
  stop. Zero warnings is the bar, not "no new warnings".
- Behavioral tests over plumbing tests. Name them like sentences
  (`gigging_alone_cannot_make_you_a_star`).
- Every new field on a serialized struct (`Game`, `Band`, `Release`, …)
  takes `#[serde(default)]` — old `.sav` files must keep loading.
- Log lines are the player's window into mechanics: when a rule blocks or
  caps something, say why in the log, in-fiction
  (see "The regulars know every word — you've outgrown this stage").
- Constants live at the top of `src/game/mod.rs` with a comment explaining
  the design intent, not the arithmetic.
- Don't touch files another track owns; if you must, note it in your PR
  description so the coordinator sequences the merge.

### Do-not-undo design decisions

- **World RNG is injected** (`&mut impl Rng` through `GameWorld::new` /
  `update_week`), seeded from `world_seed` (`ROCKER_SEED`). Never
  `thread_rng()` inside `world.rs`.
- **Live fame is capped twice**: by venue (`prestige + VENUE_FAME_HEADROOM`)
  and by catalog (`LIVE_FAME_BASE_CAP + 8/single + 15/album`, releases in
  their sales window count). Tours obey the catalog cap. **Support slots are
  deliberately uncapped** — they borrow the host's audience and are
  opportunity-gated, not grindable.
- **Idle decay contract**: a week is "visible" iff the action was
  Gig/Tour/Support **or** `just_released_music` is non-empty. One grace
  week, then −1 fame per idle week.
- **Unit economics anchors**: demand = `score × distribution_multiplier ×
  UNITS_PER_SCORE_POINT`; indie income $2/copy, label $3/copy × royalty.
  Sales (first run and long tail) never exceed `copies_pressed`
  (`copies_pressed == 0` = legacy uncapped). Long tail draws down
  `copies_pressed − copies_sold`.
- **Marketing belongs to the label when signed** — the player action errors,
  the label auto-push is `market_reach / 2` clamped 10–45.
- **News is derived, not scripted** — generate news from state.
- `MusicGenre` (world.rs) is the single genre enum; `aliases()` maps to
  markets.json keys. `ReleaseType` is `Copy`.
- `PRESSING_TIERS` and `BREAK_WEEKS` are `pub` — the UI renders them.

### How to verify

- `cargo test` — 25 tests across data_loader, timeline, world, game.
- `cargo clippy --all-targets` — warning-free.
- `ROCKER_SEED=42 cargo run` twice → identical opening scene (worldgen
  determinism; full-run determinism is Track B's job).
- Manual smoke: setup → write songs → record (pressing picker) → gig
  (venue picker) → marketing (indie only) → save/load.

---

## Track map

| Track | Goal | Size | Owns | Merge order |
|---|---|---|---|---|
| A | Player on the charts + charts UI | S–M | render.rs, app.rs, one mod.rs hook | 1st |
| B | Deterministic gameplay (seeded action RNG) | M | mod.rs action fns, events.rs | 3rd (rebase last of A–C) |
| C | Genre identity stepping stone | M | band.rs, record actions, setup UI | 2nd |
| D | Balance lab: headless sim + tuning | M | new src/game/sim.rs only | anytime |
| E | Structure & infra (mod.rs split, dead-code, CI, save-compat) | L | everything | strictly last |

A, C, D can start immediately in parallel. B can start immediately but
expect to rebase over A and C (all three touch `src/game/mod.rs` in
different functions). E starts only after A–D are merged.

---

## Track A — the charts are a shared scoreboard

**Why:** scene bands chart every week, but the player never appears — the
one surface where you're supposed to compete with the living scene is
write-only. The machinery is already built and tested; it's just unfed.

**Current state:**
- `GameWorld::submit_chart_entry(title, band_name, is_player, score)` —
  src/game/world.rs:452 — returns the chart position; `decay_charts`
  already handles player entries (world.rs:437).
- The hook point: src/game/mod.rs:1029, where a closing release gets
  `initial_sales_score`. Submit there with `is_player = true`.
- Modal pattern to copy: Deals modal (`Screen::Deals` in app.rs,
  `draw_deals_modal` in render.rs).

**Tasks:**
1. Submit player releases to the chart when their sales window closes; log
   the position if it charts ("📈 '…' enters the charts at #4").
2. Charts modal on hotkey `c`: top 10, `is_player` rows highlighted, weeks
   on chart shown.
3. Scene panel: show the current #1 record next to the top act.
4. Terminology sweep: the last "rival" comment — src/game/mod.rs:942.

**Acceptance:** a test proving a high-scoring player release lands on the
chart and a flop doesn't; modal renders with a player entry highlighted;
`rg -i rival src/` returns only world.rs:20 (the "not rivals" doc comment).

---

## Track B — finish the determinism story

**Why:** worldgen and weekly world updates are seeded, but every player
action rolls `thread_rng()` — same seed + same inputs ≠ same run. Full
determinism makes runs shareable and bugs reproducible, and Track D's sim
gets vastly more useful on top of it.

**Current state:**
- Seeded: worldgen and `update_week` via splitmix64 key derivation —
  src/game/mod.rs:1165 is the pattern to reuse.
- Ambient `thread_rng()`: song/recording quality (mod.rs:243, 282), write
  count (401), tour rolls (789), support offers (900–960), random events
  (events.rs:34, apply at mod.rs:1187, royalty event ~1272).

**Tasks:**
1. Derive a per-week *action* RNG from `world_seed` + week (+ a distinct
   stream constant so it doesn't correlate with the world RNG).
2. Thread `&mut impl Rng` through the action helpers and `EventManager` —
   same injection style as world.rs. No `thread_rng()` left under src/game/.
3. Test: two games, same seed, same scripted 20-action sequence → identical
   money, fame, week, and log text.

**Acceptance:** that test passes; `rg thread_rng src/game/` is empty;
existing 25 tests still pass (they don't assert specific random values, so
reseeding is safe — if one does, fix the test, not the design).

**Conflict note:** touches many mod.rs functions — coordinate to merge
after A and C, rebasing over them.

---

## Track C — genre identity (the bridge, not the cathedral)

**Why:** every release is hardcoded Rock (src/game/mod.rs:472 and :532,
marked `// Placeholder`), while the scene has full genre identity matched
against era trends. The player should face the same "play the trend or play
yourself" choice. FUTURE.md §1–2 (ability-derived genre proficiency) stays
the end state — this track is the thin bridge, so keep the surface small.

**Tasks:**
1. `Band.genre: MusicGenre` (`#[serde(default)]` — defaults Rock for old
   saves). Setup flow gains a genre picker step (Setup screen,
   src/ui/app.rs `handle_setup_key`).
2. Stamp releases with the band's genre instead of the placeholder.
3. Apply the era-genre modifier to player sales scores in
   `calculate_release_sales_score` via `era_genre_modifier(year, aliases)`
   (already in data_loader) — the dynamic modifier is already applied;
   the era one isn't.
4. Weekly news nudge when your genre is hot/cold ("Punk is exploding —
   right place, right time").

**Acceptance:** tests that a trend-matched genre outsells an off-trend one
under identical inputs; setup can pick every `MusicGenre::ALL` entry;
old saves load with genre Rock.

**Conflict note:** shares the record-action region of mod.rs with B, and
the Setup screen with nothing. Merge before B.

---

## Track D — balance lab (zero-conflict, start anytime)

**Why:** the new economy (caps, decay, pressing, label promo) was tuned by
arithmetic, not play. Nobody has watched 100 careers unfold. Numbers that
probably need eyes: early game under decay (bleeding fame while saving for
the first single), pressing tier costs vs. 1970 money, label pressing size
(`reach×100 + fame×50`), win-by-year distribution.

**Tasks:**
1. New `src/game/sim.rs`, `#[cfg(test)]`-only, plus the one `mod sim;` line
   in game/mod.rs (your only shared-file touch). Bots drive `process_turn`
   directly with simple policies:
   - *gig-grinder* (never records),
   - *studio-rat* (records constantly, never performs),
   - *balanced-indie* (write → press club runs → gig the window),
   - *label-loyalist* (signs the first deal, delivers albums).
2. Run each bot over many seeds × 15 game-years (`#[ignore]`-tag the long
   ones; a fast smoke subset runs in CI). Collect: fame/money curves,
   weeks-to-first-album, sell-out frequency, win/lose rates, panic count.
3. Report findings in the PR; propose constant changes as a separate commit
   with the sim numbers before/after. Invariants to encode as tests:
   gig-grinder never exceeds `LIVE_FAME_BASE_CAP`; balanced-indie can win
   by year ~12 on a majority of seeds; nobody panics.

**Acceptance:** sim module + invariant tests green; a written tuning
proposal (even if the proposal is "numbers are fine").

---

## Track E — structure & infra (after A–D merge)

**Why:** src/game/mod.rs is **1,895 lines** — Game struct, constants, 15
action handlers, the sales pipeline, events, save/load, and tests in one
file. Every track above collides inside it. Splitting *now* would serialize
the other tracks, so this lands last and clears the ground for the
FUTURE.md Musician arc.

**Tasks:**
1. Split mod.rs: `game/actions.rs` (action_* handlers),
   `game/economy.rs` (costs, pressing, sales pipeline, outcome),
   `game/turn.rs` (process_turn, advance_week_events, visibility/decay),
   keeping `Game` + constants + module wiring in mod.rs. Pure moves — no
   behavior changes mixed in.
2. Remove `#![allow(dead_code)]` (src/main.rs:3); triage what surfaces —
   delete or justify with a targeted allow + comment.
3. Save-compat fixture: commit a pre-0.5 `.sav`; test that `load_game`
   accepts it (idle_streak/copies_pressed default correctly).
4. CI: green on all three OSes, add `cargo clippy --all-targets --
   -D warnings` and `cargo fmt --check` gates.
5. CHANGELOG.md for 0.4.0 → 0.5.0.

**Acceptance:** identical test list passes post-split; `git log` shows
mechanical-move commits separate from any fix; CI gates active.

---

## Backlog (unassigned, roughly ordered)

- **FUTURE.md §1–§6** — the Musician/abilities/personality arc. Blocked on
  Track E's split; §2 supersedes Track C's bridge when it lands.
- "Outgrown" tag in the venue picker (the cap is invisible until you hit it).
- Count only non-flop releases toward the catalog fame cap.
- Player choices inside random events (currently auto-resolved).
- Difficulty levels; chart-position rewards (fame/deal interest from
  charting); label contract renegotiation.
- The e2e expect-script driver from the old handoff's #10 (drive the TUI in
  a pty) — pairs well with Track B's determinism.
