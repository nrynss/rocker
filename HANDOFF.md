# Rocker — Ecosystem Refactor Handoff

Status as of this session. The project is **mid-refactor and does not currently
compile** (5 errors, all in `src/game/mod.rs`, all from the in-flight world
rewrite). This document is the pick-up point.

The goal of this refactor: turn the "world" from an authored backdrop into a
living ecosystem — hundreds of generatively-named scene bands acting as agents
(releasing, charting, signing, breaking up), real genre identity matched
against era trends, venue/region-driven gigs and tours, one-shot historical
events, and a reproducible per-run seed.

Terminology note from the user: these are **scene bands / acts**, NOT "rivals".
"Rival" is reserved for a future feud mechanic. The struct has been renamed
`CompetingBand` → `SceneBand` and the field `competing_bands` → `bands`
(with `#[serde(alias = "competing_bands")]` for old saves). Any remaining
"rival" wording in the UI must go (see task #9).

---

## Build status: DOES NOT COMPILE

`cargo build` → 5 errors, all stale callers of the new `world.rs` API in
`src/game/mod.rs` (plus matching ones in `src/ui/render.rs` the compiler hasn't
reached yet). These are the **critical path** — fixing them is tasks #1 and #3.

| Location | Error | Fix |
|---|---|---|
| `src/game/mod.rs:122` | `GameWorld::new(&data_files)` takes 2 args now | pass a seeded RNG: `GameWorld::new(&data_files, &mut rng)` |
| `src/game/mod.rs:879` | `world.update_week(&self.timeline, &self.data_files)` takes 3 args now | pass RNG: `update_week(&self.timeline, &self.data_files, &mut rng)` |
| `src/game/mod.rs:692` | no field `competing_bands` | rename to `self.world.bands` (in `update_support_tour_offer`) |
| `src/game/mod.rs:1374` | no field `competing_bands` | rename to `game.world.bands` (in a test) |
| `src/ui/render.rs:279,291` | no field `competing_bands` | rename to `game.world.bands`; also rename the local `rivals` var (terminology) |

Run `cargo build 2>&1 | rg '^error' -A6` to see them fresh.

---

## Done this session

- **Task #2 — Generative names (COMPLETE, compiles, tested).**
  Added `tracery` (0.2.1, no default features) in `Cargo.toml`.
  `GameDataFiles` now holds two `tracery::Grammar`s built at load from the word
  lists plus two new editable pattern files, auto-created on first run:
  `data/band_name_patterns.txt`, `data/song_title_patterns.txt` (patterns use
  `//` comments because `#` is tracery's tag marker).
  New API: `generate_band_name(&mut impl Rng)`, `generate_song_title(&mut impl Rng)`,
  `era_genre_modifier(year, &[aliases])` (nearest-year lookup, absent genre =
  0.85). `random_song_title()` now delegates to the grammar. Old
  `adjective_noun_pattern`/`verb_pattern`/etc. helpers deleted.
  Test `data_loader::tests::name_generation_is_seeded_and_varied` passes.
  Sample output verified: "Electric Dragons", "The Moon", "Radio Static", etc.

- **Task #4 — One-shot historical events (COMPLETE, code + caller + tests;
  UNVERIFIED because the crate won't compile yet).**
  `MusicTimeline` gained `triggered_events: HashSet<String>` (serde default).
  `should_trigger_historical_event(&self)` → `take_historical_event(&mut self,
  &mut impl Rng)`: fires only events not yet seen, records them, returns None
  once an era's history is exhausted. Caller wired at `src/game/mod.rs:874`.
  Tests added in `timeline.rs` (`historical_events_fire_at_most_once`,
  `advancing_year_unlocks_new_events`) — will run once the crate compiles.

- **Task #3 — Scene rewrite (world.rs is WRITTEN but NOT integrated).**
  `src/game/world.rs` fully rewritten. It compiles on its own terms; the
  breakage is only the two stale callers above. What's in it:
  - `SceneBand { name, fame, peak_fame, latest_release, genre, label, momentum }`
  - Scene starts at **180 bands** (`SCENE_START_BANDS`), bounded 120–260, with a
    realistic fame pyramid and some pre-signed to labels.
  - `update_scene_bands`: momentum + trend-aware fame drift; releases scored by
    fame/quality/era-genre-modifier/label-reach that enter a top-10 chart;
    rising unsigned acts get signed.
  - `ChartEntry` + `charts: Vec<ChartEntry>`, weekly `decay_charts`
    (`CHART_DECAY` 0.85, floor 25), `submit_chart_entry` returns chart position.
  - `update_scene_population`: low-fame + no-momentum bands break up (notable
    ones make news), newcomers arrive (chasing trends 40% of the time),
    refills hard below the floor.
  - `poach_rejected_deal(label_name, rng)` — for task #8.
  - `MusicGenre` now has `#[default] Rock`, `ALL`, `name()`, `aliases()`
    (→ markets.json genre keys), `random`, `random_trending`, and `Display`.
  - `GameWorld::new` and `update_week` both now require `&mut impl Rng`.
  - Tests written (reproducibility, distinct names, bounded population, chart
    ranking, deal poaching) — unverified pending compile.

---

## Remaining tasks (in dependency order)

### #1 — Seeded worldgen ✅ DONE
SplitMix64 weekly seed derivation implemented. `world_seed: u64` on `Game`,
`ROCKER_SEED` env var support, seeded `StdRng` for worldgen and weekly updates.

### #3 — Scene integration ✅ DONE
`competing_bands` → `bands` rename complete. Seeded RNG wired through
`update_week`. All 16 tests passing.

### #5 — Player genre identity ⛔ SUPERSEDED BY FUTURE.md
The flat `genre_ratings: HashMap<MusicGenre, u8>` approach is replaced by
FUTURE.md's ability-derived genre system. Genre proficiency is now a weighted
sum of 11 musician abilities (vocals, guitar, bass, drums, keys, etc.), not a
stored HashMap. `active_genre` on `Band` stays, but the quality/sales formulas
are driven by the `MusicGenre::ability_weights()` table. **Do not implement
the old #5 spec.** See FUTURE.md §1–§2.


### #6 — Venue-based gigs ✅ DONE
`GameAction::Gig(venue_index)` implemented with prestige-based fame locks, ticket sales attendance, and base payment scaling.

### #7 — Region tours with markets.json economics + regional fame ✅ DONE
`GameAction::GoOnTour(region_index)` implemented with population-tier fame gates, country-specific travel modifiers, and a custom regional fame system where repeat tours yield profit.

### #8 — Scene bands pick up rejected deals ✅ DONE
Wired in `action_reject_deal` — rejecting a deal now triggers the scene poaching logic and prints it to the news logs.

### #9 — UI: pickers, charts modal, setup genre, terminology sweep 🟡 PARTIALLY SAFE

**Build now (stable):**
- Venue picker modal (for #6) — copy existing modal pattern.
- Region picker modal (for #7) — same pattern.
- Charts modal on `c` — show top 10 with player entries highlighted.
- Scene panel: show band count + top act + #1 single.
- Terminology sweep: remove ALL remaining "rival" wording.

**Defer to FUTURE.md §6:**
- Setup flow (Solo/Band toggle, genre picker, ability allocation).
- Band panel redesign (musician abilities, relationship indicators).
- Practice picker (ability training selection).
- Band Management modal (`j` hotkey — Go Solo / Form Band / Join Band).

`App`/`Screen` enums are in `src/ui/app.rs`; drawing in `src/ui/render.rs`.
Existing modal pattern (Deals, Marketing, SupportOffer) is the template.

### #10 — Tests, balance, e2e, README ⏳ DEFERRED
Unit tests for genre modifier lookup, venue/tour economics, regional fame growth.
Update the expect script `scratchpad/drive_tui.exp` for the new gig picker flow
(Gig is no longer a single keypress — it opens a picker). Balance pass on the
scene (fame drift rates, chart scoring). Update README + bump version to 0.5.0.

**Note:** Wait until after FUTURE.md's Musician refactor lands. The quality
formulas, practice action, and setup flow will all change — writing tests
against the current code would produce throwaway work.

---

## Key design decisions already baked in — do not undo

- **World RNG is injected, not ambient.** `GameWorld::new` and `update_week` take
  `&mut impl Rng`. This is deliberate for seeding (#1). Don't revert to
  `thread_rng()` inside world.rs.
- **`MusicGenre` is the single genre enum** (in world.rs), now `Default` +
  `Hash`. `aliases()` maps it to markets.json's snake_case genre keys. The player
  band, scene bands, and releases should all use it. **FUTURE.md adds
  `ability_weights()` to MusicGenre** for derived genre proficiency.
- **Charts are the shared surface** where player and scene compete. Player
  releases should call `world.submit_chart_entry(title, band, is_player=true,
  score)` when their sales window closes (hook this in
  `process_music_releases_and_marketing`), so the player appears on the same
  chart as scene bands. Not yet wired — add during #9 (charts modal).
- **FUTURE.md is the design authority** for the Musician/Abilities/Personality
  system, solo/band identity, and relationship mechanics. Task #5's original
  `genre_ratings` spec is obsolete. See FUTURE.md for the full design.
- **News is derived, not scripted.** `update_week` returns `Vec<String>` of
  events that actually happened (econ shifts, chart hits, signings, notable
  splits/debuts). Keep generating news from state, not from canned lines.
- Pattern files (`data/*_patterns.txt`) are user-editable content; the defaults
  are written on first run. `.gitignore` already has `*.sav`; decide whether to
  commit the generated pattern files (recommend yes, as documented defaults).

## How to verify once compiling
- `cargo test` — unit tests across data_loader, timeline, world, game.
- `cargo clippy --all-targets` — was clean before this refactor; keep it clean.
- `expect scratchpad/drive_tui.exp` — full TUI playthrough (needs the #10 update
  for the gig picker). Run in a pty ≥ 80×24; the script sets `stty rows 30
  columns 100`.
- `ROCKER_SEED=42 cargo run` twice should produce an identical opening scene.
