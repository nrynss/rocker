# Rocker — Money Cycle Handoff (v0.7)

> Feature cycle: tour economics that quote instead of ambush, the
> lifestyle ladder, charts with depth-40 stability and climbers, record
> certifications (silver/gold/platinum), label recoupment, re-pressing,
> purchasable indie distribution, and contracts with a real clock —
> terms, breach, renewals, and a label that actively chases its money.
> **Design is decided** — read `docs/DESIGN-v0.7-money-cycle.md` before
> claiming anything. Numbers marked [tune] there are sim-lab candidates
> validated in M7, not invented per-agent.
>
> Prior cycles: `docs/archive/HANDOFF-v0.6-cycle.md` (life),
> `docs/archive/HANDOFF-v0.5-cycle.md` (features),
> `docs/archive/HANDOFF-v0.5.1-structure.md` (structure) · Shipped:
> `CHANGELOG.md` · North star for a later cycle: `FUTURE.md` (Musician).
> **Deferred:** addiction (§9.1), vacations picker (§9.3), manager
> (§9.4) — do not implement, do not remove the dormant fields.

Baseline at cycle start: **0.6.0**, `cargo test` → **102 passed,
4 ignored**, clippy clean, fmt clean. Cycle closes as **0.7.0**.

---

## How agents claim work

Same protocol as the life cycle — this file is the **single
coordination surface**. Do not start a task that is claimed or whose
prerequisites are not ✅.

### One branch for the whole cycle

**All work lands on `money/v0.7`.** No per-task branches, no parallel
task PRs — the cycle PR merges once, at cycle close, when the human
says so. Only commits to the shared branch matter.

```text
git fetch origin
git checkout money/v0.7
git pull --rebase origin money/v0.7   # always before claim / before push
# … do one claimed task …
git push origin money/v0.7
```

If `pull --rebase` conflicts in files you do **not** own, stop and
unclaim — do not "fix" another agent's WIP.

### Claim protocol (mandatory)

1. **On `money/v0.7`**, up to date with `origin`.
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

Commit prefix: `money(M#): <short description>`.

### Ground rules (every agent)

- **This cycle changes behavior on purpose** — but only *your task's*
  behavior. No drive-by balance edits outside your Owns; every [tune]
  number you set must match the design doc or carry a Notes line saying
  what you changed and why.
- **Tests:** counts will grow and some assertions will legitimately
  change. Never delete a test to make it pass; adapt it and say so in
  the commit. The determinism tests
  (`same_seed_and_same_choices_replay_the_same_career`,
  `seeded_worlds_are_reproducible_in_the_harness`,
  `worldgen_is_reproducible_per_seed`) must pass **unmodified**.
- **Serde / saves:** every new field `#[serde(default)]`. Never rename
  serialized fields (`energy` and addiction fields stay, dormant).
  `saves_from_v0_4_still_load` and the pre-0.5 fixture are sacred.
- **World RNG is injected, not ambient.**

---

## Task board

| ID | Task | Size | Prereqs | Owns (exclusive) | Status | Claimed by | Branch | Done |
|----|------|------|---------|------------------|--------|------------|--------|------|
| **M1** | Tour economics: rig picker, itemized up-front quote, fame-decoupled costs, wear table (design §A) | M | — | `src/game/actions/live.rs` (tour fn + quote helper), tour/rig consts in `src/game/constants.rs`, `touring_costs` section of `data/markets.json`, touring structs in `src/data_loader.rs`, rig/quote UI in `src/ui/render/modals/pickers.rs` + its input wiring | ⬜ open | | money/v0.7 | |
| **M2** | Lifestyle ladder: tiers, weekly upkeep, stat effects, image, move modal, broke eviction (design §B) | M | — | `src/game/lifestyle.rs`, `LifestyleTier` + field in `src/game/player.rs`, `ChangeLifestyle` action in `src/game/actions/rest.rs`, lifestyle consts in `constants.rs`, **new** `src/ui/render/modals/lifestyle.rs` + `mod`/input wiring | ⬜ open | | money/v0.7 | |
| **M3** | Charts that breathe: depth 40, ramp-in climbers, decay 0.92, peak tracking, calmer scene odds, movement news (design §C) | M | — | `src/game/world/charts.rs`, release-odds consts in `src/game/world/scene.rs`, `src/ui/render/modals/charts.rs` + scroll input | ⬜ open | | money/v0.7 | |
| **M4** | Certifications: thresholds, weekly check, award effects, discography badges (design §D) | S | — | `certified` field in `src/game/music.rs`, certification pass + consts (own section of `src/game/economy.rs`), badge lines in `src/ui/render/modals/file.rs` | ⬜ open | | money/v0.7 | |
| **M5** | Label recoupment (advance + pressing + promo) + label auto-repress (design §E-2, §E-1 label half) | M | M4 | `unrecouped` on `RecordDeal` in `src/game/band.rs`, advance-to-ledger line in `action_sign_deal` (`business.rs`), release-resolution + royalty sections of `src/game/economy.rs`, recoup consts in `constants.rs` | ⬜ open | | money/v0.7 | |
| **M6** | Indie re-press action + distribution tiers (design §E-1 indie half, §E-3) | M | M5 | `RePress` + distribution choice in `src/game/actions/business.rs`, `distribution_multiplier`/`plan_pressing` in `economy.rs`, distribution consts, re-press/distribution picker UI in `pickers.rs` | ⬜ open | | money/v0.7 | |
| **M9** | Deal lifecycle: contract term + albums, free agency at the later of both, breach + `deal_cooldown`, recoupment-dependent renewal window (new contract / extension / silence, opens 26 wks pre-expiry), label memos & recoup pressure (design §E-4, §E-5) | M | M5 | term/`signed_week` fields + fulfillment logic in `src/game/band.rs`, term generation + renewal in `src/game/world/deals.rs`, memos + pressure scaling in `src/game/label_moves.rs`, deal-completion call-site in `economy.rs`, `deal_cooldown` on `Band`, deal-term consts in `constants.rs` | ⬜ open | | money/v0.7 | |
| **M7** | Sim-lab validation: homebody / road-dog / indie-lifer bots, measured targets from design §F, [tune] sweeps | M | M1–M6, M9 | `src/game/sim.rs`, `src/game/tests/**` (new test files), Notes below | ⬜ open | | money/v0.7 | |
| **M8** | Cycle close: board audit, CHANGELOG, bump 0.7.0, PR to main | S | all | `HANDOFF.md`, `CHANGELOG.md`, `Cargo.toml`/`Cargo.lock` | ⬜ open | | money/v0.7 | |

### Known overlaps

| Pair | Issue |
|------|-------|
| M4 → M5 → M6/M9 | All touch `economy.rs` — serialized by prereqs. M4 owns only its new certification section; M5 the release-resolution/royalty paths; M6 the distribution/pressing helpers; M9 only the deal-completion call-site. |
| M6 ∥ M9 | Both unblock on M5 and both touch `business.rs` — M6 owns RePress/distribution, M9 only the sign-action term-stamping lines. Rebase, keep diffs section-scoped. |
| M5 ∥ M9 | M5 lands first (prereq). M9 extends the same `RecordDeal` struct — take fields, don't reshape M5's. |
| M1 ∥ M6 | Both touch `pickers.rs` — M1 owns the tour/rig picker, M6 the pressing/distribution picker. Rebase, keep diffs picker-scoped. |
| M1 ∥ M2 | Both add consts to `constants.rs` — separate sections, rebase. |
| M2 ∥ M3 | Both add a modal/input wiring — `mod`/`pub use` lines minimum-diff, rebase. |
| Any ∥ any | Pull --rebase before push. No force-push without the human. |

### Notes

- (cycle start) Board created from `docs/DESIGN-v0.7-money-cycle.md`;
  baseline verified green on `main` @ ad4563f.
- (cycle start, same day) Scope amended before any claims: design §E
  grew §E-4 (contract term) and §E-5 (active label), the advance now
  joins M5's recoupment ledger, and M9 was added. Rationale: deals
  cleared instantly on the release beat (`fulfill_album_obligation`),
  1-album deals + unrecouped advances made sign-and-run free money.
