# Rocker — Structure Hardening Handoff (v0.5.1)

> **Active cycle.** Pure structural refactor so the next feature cycle
> (Musician / FUTURE.md §1–§6) lands in small, owned files. **No gameplay
> behavior changes.** No formula tuning. No Musician implementation yet.
>
> Prior cycle: `docs/archive/HANDOFF-v0.5-cycle.md` · What shipped: `CHANGELOG.md`
> · Design north star (later): `FUTURE.md`

Baseline on start of cycle: **0.5.0**, `cargo test` → **42 passed, 2 ignored**,
clippy clean, `thread_rng` gone, `src/game/mod.rs` ~1 060 lines of which
~750 are tests.

---

## How agents claim work

This handoff is the **single coordination surface**. Do not start a task
that is already claimed or whose prerequisites are not ✅.

### One branch for the whole cycle

**All structure work lands on `struct/t4-genre`.** Do not open
`struct/t1-…`, `struct/t5-…`, or any other task branch. Multi-branch
parallelism caused rebase pain and split history; coordination is the
**board + exclusive Owns**, not git branches.

```text
git fetch origin
git checkout struct/t4-genre
git pull --rebase origin struct/t4-genre   # always before claim / before push
# … do one claimed task …
git push origin struct/t4-genre
```

If `pull --rebase` conflicts in files you do **not** own, stop and
unclaim — do not “fix” another agent’s WIP.

### Claim protocol (mandatory)

1. **On `struct/t4-genre`**, up to date with `origin` (see above).
2. **Read this whole file**, then the task section for the ID you want.
3. **Prerequisites:** every ID in `Prereqs` must be `✅ done` (or `—`).
4. **Claim atomically** — edit the Task board row in the **same commit**
   that starts the work (or a tiny commit immediately before code):
   - `Status`: `⬜ open` → `🔒 claimed`
   - `Claimed by`: short agent/session id (e.g. `claude-a3f2`, `grok-struct-1`)
   - `Branch` column: always `struct/t4-genre` (shared; do not invent a name)
5. **While working**, you own every path listed under **Owns**. Do not
   edit another open/`🔒 claimed` task’s owned files. If you must touch
   shared wiring (`src/game/mod.rs` module list, `src/ui/mod.rs`), keep
   the diff to the minimum `mod` / `pub use` lines.
6. **Done means green**, then update the board and **push the same branch**:
   - `Status`: `🔒 claimed` → `✅ done`
   - Fill `Done` with the commit short SHA that completed the task
   - Leave `Claimed by` as historical record
7. **Unclaim** if you abort: set `Status` back to `⬜ open`, clear
   `Claimed by`, and say why in one line under Notes.

### Status legend

| Mark | Meaning |
|------|---------|
| `⬜ open` | Available if prereqs are done |
| `🔒 claimed` | An agent is actively working it — **do not take** |
| `✅ done` | Merged / complete; safe as a prerequisite |
| `⏸ blocked` | Waiting on human decision (should be rare this cycle) |

### Ground rules (every agent)

- **Pure moves only.** No behavior changes, balance tweaks, log-line
  rewrites, or “while I’m here” formula fixes. If a test fails after a
  move, you restored visibility wrong — fix the move, don’t “fix” the game.
- **Verify before claiming done:**
  ```text
  cargo test
  cargo clippy --all-targets -- -D warnings
  cargo fmt --check
  ```
  Same test **names** and **count** (42 + 2 ignored) unless the task
  explicitly relocates tests (names must still match).
- **Serde / saves:** do not rename serialized fields. Re-exports are fine;
  on-disk shape is sacred (`tests/fixtures/pre-0.5.sav` must keep loading).
- **RNG draw order is sacred.** Moving code must not reorder
  `gen_*` calls on a stream. The determinism tests
  (`same_seed_and_same_choices_replay_the_same_career`,
  `seeded_worlds_are_reproducible_in_the_harness`,
  `worldgen_is_reproducible_per_seed`) are the contract — if they fail,
  you reshuffled draws; undo.
- **Visibility:** prefer `pub(super)` / `pub(crate)` over `pub`. Integration
  tests are **not** the goal this cycle; keep unit-test access via
  `#[cfg(test)]` modules under `src/game/`.
- **One shared branch:** `struct/t4-genre` only. Commit message prefix:
  `struct(T#): <short description>`. One open PR from this branch to
  `main` when the cycle closes (or whenever the human merges); agents
  push commits, they do not open parallel task PRs.
- **Update this file** when claiming and when completing, on the same
  branch as the code.

### Do-not-undo (still in force from v0.5)

- World + action RNG injection from `world_seed` — never reintroduce
  `thread_rng` under `src/`.
- Live fame double-cap, idle decay, pressing/unit economics, label marketing
  ownership, news-from-state, `MusicGenre` as the single genre enum.
- Support slots remain deliberately uncapped by catalog fame.

### Out of scope this cycle

- FUTURE.md Musician / abilities / personality
- Decade pacing, event-fame audit, tour bots, difficulty modes
- New features, UI chrome, balance passes

Those wait for a **Musician cycle handoff** after this board is all ✅.

---

## Task board (source of truth)

**How to read prereqs:** `—` = none (start when ready). Multiple IDs =
all must be ✅. Tasks with **disjoint Owns** and satisfied prereqs may
run **in parallel on the same branch** — never via extra branches.

| ID | Task | Size | Prereqs | Owns (exclusive) | Status | Claimed by | Branch | Done |
|----|------|------|---------|------------------|--------|------------|--------|------|
| **T1** | Extract `game` unit tests out of `mod.rs` into `src/game/tests/` | M | — | `src/game/mod.rs` *(tests module only + `mod tests` wiring)*, **new** `src/game/tests/**` | 🔒 claimed | claude-t1 | struct/t4-genre | |
| **T2** | Extract tuning knobs → `src/game/constants.rs` | S | T1 | `src/game/constants.rs` *(new)*, `src/game/mod.rs` *(const block → re-export)*, imports in modules that referenced parent consts | ⬜ open | | struct/t4-genre | |
| **T3** | Extract `Game` / `GameAction` / lifecycle → `src/game/game.rs`; thin `mod.rs` | S | T2 | `src/game/game.rs` *(new)*, `src/game/mod.rs` *(shell)*, `src/game/tests/**` *(paths/`use` only if needed)* | ⬜ open | | struct/t4-genre | |
| **T4** | Extract `MusicGenre` → `src/game/genre.rs` | S | — | `src/game/genre.rs` *(new)*, `src/game/world.rs` *(remove genre)*, all `use` sites of `MusicGenre` | ✅ done | grok-struct-t4 | struct/t4-genre | 860fb6f |
| **T5** | Split `actions.rs` → `actions/{mod,studio,live,business,rest}.rs` | M | — | `src/game/actions.rs` → `src/game/actions/**` only | ⬜ open | | struct/t4-genre | |
| **T6** | Split event *outcomes* out of `turn.rs` → `events_apply.rs` | S | — | `src/game/turn.rs`, **new** `src/game/events_apply.rs`, `src/game/mod.rs` *(one `mod` line)* | ⬜ open | | struct/t4-genre | |
| **T7** | Split `world.rs` → `world/{mod,scene,charts,deals,venues}.rs` | L | T4 | `src/game/world.rs` → `src/game/world/**`, world unit tests relocate with code | ⬜ open | | struct/t4-genre | |
| **T8** | Optional: `src/game/rng.rs` (action-stream helpers only) | S | T3 | `src/game/rng.rs` *(new)*, `src/game/game.rs`, `src/game/turn.rs` *(import paths)* | ⬜ open | | struct/t4-genre | |
| **T9** | Split UI input handlers out of `app.rs` | M | — | `src/ui/app.rs`, **new** `src/ui/input/**` (or `src/ui/input.rs` + submodules), `src/ui/mod.rs` | ✅ done | antigravity | struct/t4-genre | |
| **T10** | Split UI drawing out of `render.rs` | M | — | `src/ui/render.rs`, **new** `src/ui/render/**`, `src/ui/mod.rs` | ⬜ open | | struct/t4-genre | |
| **T11** | Cycle close: line-count report, board audit, archive note | S | T1–T7, T9–T10 *(T8 optional)* | `HANDOFF.md`, optional short note in `CHANGELOG.md` under Internal | ⬜ open | | struct/t4-genre | |

### Parallelism map (waves)

```text
Wave 0 (immediate, fully parallel — disjoint owns):
  T1 (game tests)     T4 (genre)     T5 (actions)     T6 (events_apply)
  T9 (ui input)       T10 (ui render)

Wave 1:
  T2 (constants)      ← after T1
  T7 (world split)    ← after T4

Wave 2:
  T3 (game.rs shell)  ← after T2
  T8 (rng.rs)         ← after T3, optional

Wave 3:
  T11 (close)         ← after required tasks
```

**Conflict warnings (same branch — Owns are the mutex):**

| Pair | Issue |
|------|--------|
| T1 ∥ T2 ∥ T3 | All touch `mod.rs` — **serialized** by prereqs (T1→T2→T3) |
| T4 ∥ T7 | T7 must wait for T4 so genre is not moved twice |
| T9 ∥ T10 | Both may touch `ui/mod.rs` — only add `mod` lines; never reformat the other’s files. Prefer one claimed at a time if unsure |
| T6 | May add one `mod events_apply;` line in `game/mod.rs` while T1–T3 run — pull --rebase; keep that line-only |
| Any ∥ any | **Pull --rebase before push.** Do not force-push unless the human says so |

---

## Target tree (end state)

```text
src/game/
  mod.rs                 # module list + pub use only (≪ 80 lines)
  constants.rs           # all tuning knobs + design comments
  game.rs                # Game, GameAction, SupportTourOffer, new/init/save/load
  rng.rs                 # optional (T8): action_rng_for_week + salts
  genre.rs               # MusicGenre + Display + aliases (+ room for ability_weights later)
  band.rs
  player.rs
  music.rs
  events.rs              # EventManager / trigger selection (unchanged role)
  events_apply.rs        # apply_random_event / apply_historical_event bodies
  economy.rs
  timeline.rs
  turn.rs                # process_turn, visibility, offer hooks, game-over
  actions/
    mod.rs               # execute_action match only
    studio.rs            # write, practice, record, quality helpers
    live.rs              # gig, tour, live_fame_cap, regions
    business.rs          # deals, marketing, support
    rest.rs              # laze, break, doctor
  world/
    mod.rs               # GameWorld, market, update_week conductor
    scene.rs             # SceneBand, population, momentum, scene releases
    charts.rs            # ChartEntry, decay, submit
    deals.rs             # PotentialDealOffer, buzz, generate, poach
    venues.rs            # Venue + generation
  tests/                 # #[cfg(test)] only
    mod.rs               # harness: test_game, test_release, helpers
    fame.rs
    releases.rs          # pressing, charts entry, genre sales, marketing
    deals.rs
    support.rs
    determinism.rs
    save_compat.rs
    …                    # split by concern; keep test fn names stable
  sim.rs                 # balance lab (already test-only) — leave path or
                         #   `pub mod sim` under tests/ only if T1 chooses to;
                         #   default: keep `src/game/sim.rs` as today

src/ui/
  mod.rs
  app.rs                 # App struct, run loop, thin dispatch
  input/…                # key handlers by screen
  render/…               # draw_* by panel/modal
```

Line-count **soft caps** after the cycle (not enforced by CI):

| Area | Soft max |
|------|----------|
| Any single production `.rs` under `game/` or `ui/` | ~400 lines preferred, ~500 hard |
| `game/mod.rs` | ~80 |
| `world/mod.rs` conductor | ~200 |
| `actions/mod.rs` | ~80 |

---

## Task details

### T1 — Tests out of `mod.rs`

**Why:** ~750 lines of tests bury the real module surface and force every
game PR to load a novel.

**Do:**

1. Create `src/game/tests/mod.rs` with the shared harness currently at
   the top of `mod.rs`’s `#[cfg(test)] mod tests` (`test_game`,
   `test_release`, `best_open_venue`, `test_deal_offer`, …).
2. Split tests into concern files (suggested): `fame`, `releases`,
   `deals`, `support`, `determinism`, `save_compat`, `smoke` — group by
   what they assert, not 1:1 with production modules.
3. Wire with:
   ```rust
   #[cfg(test)]
   mod tests;
   ```
   in `src/game/mod.rs` (or `game.rs` after T3 — for T1, keep wiring in
   `mod.rs`).
4. **Keep every `#[test] fn` name identical** so `cargo test` output and
   muscle memory stay stable.
5. Leave `sim.rs` where it is (`src/game/sim.rs` + `#[cfg(test)] mod sim`)
   unless moving it is trivial; do not merge sim into unit tests.

**Acceptance:**

- `src/game/mod.rs` has **no** `#[test]` functions left
- `cargo test` → still 42 passed + 2 ignored, **same names**
- Harness helpers are not `pub` outside `#[cfg(test)]`

**Claim note:** while 🔒, nobody else edits the old tests block in `mod.rs`.

---

### T2 — `constants.rs`

**Why:** balance knobs and stream salts should not live beside `Game`’s
serde shape.

**Do:**

1. Move every tuning `const` / `pub const` currently at the top of
   `mod.rs` (quality, sales, pressing, fame caps, idle, deals, genre
   trend thresholds, `ACTION_STREAM_SALT`, `SETUP_STREAM_WEEK`, …) into
   `src/game/constants.rs`, **keeping comments**.
2. Re-export anything the UI or others need at the old path if useful:
   ```rust
   pub use constants::{PRESSING_TIERS, BREAK_WEEKS};
   ```
   so call sites outside `game` need not all change — or update call
   sites in one go; either is fine if compile-clean.
3. Modules that used bare `PRESSING_TIERS` via `super::*` should
   `use crate::game::constants::*` or `super::constants::*` as appropriate.

**Acceptance:** no gameplay consts left in `mod.rs` / `game.rs` except
re-exports; tests pass unchanged.

---

### T3 — `game.rs` thin shell

**Why:** `mod.rs` should only declare submodules.

**Do:**

1. Move `GameAction`, `SupportTourOffer`, `Game`, `default_seed`,
   `impl Game` lifecycle (`new`, `log`, `take_turn_log`, `action_rng*`,
   `initialize_player`, `save_game`, `load_game`) into `src/game/game.rs`.
2. `mod.rs` becomes module declarations + `pub use game::{Game, GameAction, …}`
   as needed by `main` / `ui`.
3. Fix `actions` / `economy` / `turn` / `tests` / `sim` imports (`super::Game`
   still works if they are children of `game` module tree — today they are
   `mod actions` siblings; keep them as siblings of `game.rs` via
   `mod game;` and `use crate::game::game::Game` **or** the common pattern:
   ```rust
   // mod.rs
   mod game;
   pub use game::*;
   ```
   Prefer the pattern that minimizes churn in `actions.rs` (`use super::*`).

**Acceptance:** `mod.rs` ≪ 80 lines; `Game::new` / save-compat tests still pass.

---

### T4 — `genre.rs`

**Why:** Musician §2 will hang `ability_weights()` on `MusicGenre`. It must
not live inside a 1k-line world file.

**Do:**

1. Move `MusicGenre` + its `impl` (ALL, name, aliases, random*, Display,
   Default, Hash, …) to `src/game/genre.rs`.
2. `pub use` from `world` **or** update all imports to `crate::game::genre::MusicGenre`.
   Prefer **one** canonical path: `crate::game::genre::MusicGenre`, and a
   temporary re-export from `world` only if it reduces churn — document
   which you chose in the PR.
3. Do **not** move charts/scene/deals yet (that’s T7).

**Acceptance:** `rg "enum MusicGenre" src/` → only `genre.rs`; all tests green.

---

### T5 — `actions/` package

**Why:** practice / studio quality will grow with Musician; live and
business actions should not share a 640-line file.

**Do:** pure file split:

| File | Contents (from current `actions.rs`) |
|------|--------------------------------------|
| `actions/mod.rs` | `mod studio; …` + `execute_action` |
| `actions/studio.rs` | write, practice, record_*, quality helpers, song selection |
| `actions/live.rs` | gig, tour, `live_fame_cap`, `get_sorted_regions` |
| `actions/business.rs` | deals, marketing, support accept/decline |
| `actions/rest.rs` | laze, break, doctor |

Keep methods on `impl Game` with the same `pub(super)` visibility.
Parent still `mod actions;` in `game/mod.rs` (directory replaces file).

**Acceptance:** no `src/game/actions.rs` file left (only directory);
`execute_action` still the single match hub; tests green.

---

### T6 — `events_apply.rs`

**Why:** `apply_random_event` / historical outcomes are a slab inside
`turn.rs`; personality will modulate outcomes later.

**Do:**

1. Move `apply_random_event` and `apply_historical_event` (and any private
   helpers only they use) to `src/game/events_apply.rs` as `impl Game`.
2. Leave trigger selection in `events.rs` and week orchestration in `turn.rs`.
3. One new `mod events_apply;` in the game module tree.

**Acceptance:** `turn.rs` drops substantially; event-related tests still pass
(including full-season smoke / determinism).

---

### T7 — `world/` package

**Why:** scene / charts / deals / venues are separate agent surfaces for
Musician (lazy rosters) and future deal work.

**Prereq T4** so genre is already outside.

**Do:**

| File | Owns |
|------|------|
| `world/mod.rs` | `GameWorld`, `MusicMarket`, `EconomicState`, `MusicTrend`, `update_week`, market helpers |
| `world/scene.rs` | `SceneBand`, generate scene, `update_scene_*`, population bounds consts if scene-local |
| `world/charts.rs` | `ChartEntry`, decay, submit, chart consts |
| `world/deals.rs` | `PotentialDealOffer`, buzz consts, `generate_deal_offers`, `poach_rejected_deal` |
| `world/venues.rs` | `Venue`, `generate_venues` |

Move `#[cfg(test)]` world tests into `world/tests` or bottom of the file
they assert on — keep test **function names**.

**Acceptance:** no monolithic `src/game/world.rs`; `update_week` remains a
short conductor; world + deal + chart tests green; scene size still 120–260.

---

### T8 — `rng.rs` (optional)

**Why:** documents the sacred stream contract in one place.

**Do only if Wave 2 is calm:** move `ACTION_STREAM_SALT`, `SETUP_STREAM_WEEK`,
`action_rng_for_week`, `action_rng` into `rng.rs` (constants may stay in
`constants.rs` with functions in `rng.rs` — pick one and don’t split salts
from their docs).

**Skip** if T3 already leaves these easy to find; mark board ✅ with note
`skipped — low value` only with human OK, or leave ⬜ open.

---

### T9 — UI input split

**Why:** setup / pickers / modals will grow for Musician UI (FUTURE §6).

**Do:** move `handle_*_key` families out of `app.rs` into e.g.
`ui/input/{setup,main,deals,marketing,pickers,file}.rs`. Keep `App` and
`run` / top-level `handle_key` dispatch in `app.rs`.

**Acceptance:** `app.rs` clearly under ~500 lines; setup genre test still
passes; no input behavior change.

---

### T10 — UI render split

**Why:** same as T9 for draw code.

**Do:** split `draw_*` into `ui/render/{layout,panels,modals,…}.rs`.
Shared style helpers can live in `render/mod.rs`.

**Acceptance:** `render.rs` file gone or thin; visual structure unchanged
(no drive-by color/layout redesigns).

---

### T11 — Cycle close

**Do:**

1. Confirm board: all required tasks ✅, claims consistent with git history.
2. Paste a short **line-count table** (production files only) under Notes
   below.
3. Add 2–4 lines under CHANGELOG **Internal** (unreleased / 0.5.1) listing
   the structural split — no fake player-facing notes.
4. Optionally move this file to `docs/archive/HANDOFF-v0.5.1-structure.md`
   when the human opens the Musician cycle (human trigger).

**Acceptance:** `cargo test` / clippy / fmt clean on `main`; HANDOFF board
all required ✅.

---

## Shared files cheat sheet

| Path | Who may touch |
|------|----------------|
| `src/game/mod.rs` | T1 (tests wiring), T2, T3, T6 (`mod` line only), T4/T5/T7 (`mod` / `pub use` lines only) — **serialize** via prereqs + rebase |
| `src/game/world.rs` → `world/` | T4 then T7 only |
| `src/game/actions.rs` → `actions/` | T5 only |
| `src/game/turn.rs` | T6 primarily; T8 imports only |
| `src/ui/app.rs` | T9 |
| `src/ui/render.rs` | T10 |
| `HANDOFF.md` board rows | **Every agent** for their claim/complete only — do not rewrite other tasks’ details |
| `FUTURE.md` | nobody this cycle |
| `data/**` | nobody this cycle |
| `tests/fixtures/**` | nobody this cycle |

---

## Verification cheatsheet

```bash
cargo test
cargo clippy --all-targets -- -D warnings
cargo fmt --check
# determinism / save still in the suite:
cargo test same_seed_and_same_choices_replay_the_same_career
cargo test saves_from_v0_4_still_load
cargo test worldgen_is_reproducible_per_seed
```

After T7:

```bash
cargo test scene_
cargo test chart_
cargo test independent_labels_scout
```

---

## Notes (agents append one-liners here)

_Example:_  
`2026-07-13 T5 claimed by grok-struct-1 on struct/t4-genre`

- 2026-07-13 T4 done by grok-struct-t4 on `struct/t4-genre`: canonical path `crate::game::genre::MusicGenre`; no world re-export; `random`/`random_trending` are `pub(crate)`.
- 2026-07-13 T9 done by antigravity on `struct/t4-genre`.
- 2026-07-13 **Policy:** single shared branch `struct/t4-genre` for the whole structure cycle — no per-task branches. Abandoned names like `struct/t1-tests-out` must not be used; rebase any stray work onto `struct/t4-genre`.
