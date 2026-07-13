# Changelog

Notable changes to Rocker, newest first. Format loosely follows
[Keep a Changelog](https://keepachangelog.com/); versions follow
[SemVer](https://semver.org/) in spirit — this is a game, the API is vibes.

## 0.5.1 — 2026-07-14

Structure-hardening cycle: pure refactor, no gameplay changes — the big
files were split into owned packages so the next feature cycle lands in
small, reviewable diffs.

### Internal

- Structure-only refactor, no behavior change: the four monoliths were
  split into owned packages. `src/game/mod.rs` (~1,060 lines) → a 27-line
  module shell plus `core`, `constants`, `rng`, `genre`, `events_apply`,
  and a by-concern `tests/` package; `world.rs` (~1,090) →
  `world/{mod,scene,charts,deals,venues}`; `actions.rs` (~720) →
  `actions/{studio,live,business,rest}`; event outcomes out of `turn.rs`
  into `events_apply.rs`.
- UI split to match: `render.rs` (~1,050) →
  `render/{layout,panels,modals/*,setup,game_over}` and `app.rs` (~990) →
  `app` + `input/*`. Every production `.rs` is now under ~500 lines.
- Same 42 tests (2 ignored), relocated by concern under `src/game/tests/`;
  `cargo test` / `clippy -D warnings` / `fmt --check` all green. No
  save-format, RNG-stream, or gameplay changes.

## 0.5.0 — 2026-07-13

The cycle where the scene pushes back: records gate your rise, the charts
keep score, labels scout like labels, and every career can be replayed
from its seed.

### Added

- **Genre identity.** Pick your genre when founding the band; releases
  carry it, the era's tastes swing sales for and against it, and the
  press says so when your scene heats up or the times move on.
- **The charts are a shared scoreboard.** Player releases compete with
  the scene's on the weekly chart — `c` opens the top 10 with your
  entries highlighted, and the scene panel shows the current No. 1.
- **Pressing runs.** Recording independently now means choosing a run
  (500–50,000 copies) with setup plus per-copy costs. You can't sell
  copies that were never pressed; sell-outs are called out. Signed bands
  press on the label's dime, sized by its reach and your name.
- **Deal offers with a shelf life.** Offers expire after a few weeks if
  ignored — and expiry is not rejection: nobody poaches a lapsed deal,
  and scouting resumes once the slate is clear.
- **A real scouting arc.** Independent labels court working acts early,
  boutiques come mid-career, majors only chase genuinely big acts — the
  catalog and a charting record weigh in alongside fame.
- **Deterministic careers.** Every roll — worldgen, the weekly scene,
  and now every player action — derives from the world seed. Same
  `ROCKER_SEED`, same choices, same career, same log lines.
- **The balance lab.** Bot-driven career sims (gig-grinder, studio-rat,
  balanced-indie, label-loyalist) run whole careers headless; fast
  invariants run with the suite, full 15-year sweeps behind
  `cargo test -- --ignored --nocapture`.
- **Save compatibility, proven.** A real v0.4.0 save is committed as a
  fixture; a test keeps it loading and playable forever.
- **CI gates.** Tests on Linux/macOS/Windows, `clippy -D warnings`, and
  `rustfmt --check` on every push and PR.

### Changed

- **Fame is earned in layers.** Live fame is capped twice — by the
  venue (its prestige plus headroom) and by your catalog (a base
  ceiling that each single and album lifts). Gigging alone plateaus;
  records raise the roof.
- **The spotlight fades.** After one quiet week with no shows and
  nothing on sale, fame decays weekly until you're back in the
  public eye. Take a Break is now a real four-week disappearance with
  full recovery.
- **Unit economics.** Demand is sales score × distribution reach;
  indies earn $2 a copy against their own fame-limited reach, labels
  $3 × royalty through their network. The long tail draws down the
  pressing, not an infinite well.
- **Marketing belongs to the label when signed** — their promo machine
  auto-pushes every release; running your own campaign is an indie move.
- **First tuning pass from the balance lab:** slower fame climb so
  records, not gig-grinding, gate the win.

### Fixed

- Cross-track semantic conflicts from merging the cycle's parallel
  work (player chart entries now score with genre modifiers applied;
  setup seeding and genre pickers compose).

### Internal

- `src/game/mod.rs` (2,300 lines) split into `actions`, `economy`, and
  `turn` modules; the Game struct, constants, and tests stay put.
- Crate-wide `#![allow(dead_code)]` removed and the 24 methods it hid
  deleted; `thread_rng` no longer appears anywhere in the tree.
- Repo-wide `rustfmt`; this changelog.

## 0.4.0 — 2026-07-13

Baseline for the cycle above: the ratatui career sim — write songs,
record, gig, tour, sign deals, weather random and historical events,
save/load — with data-driven worldgen and a living scene of hundreds of
bands. Relicensed under AGPL-3.0-or-later.
