# Rocker — Money Cycle Handoff (v0.7)

> Feature cycle: tour economics that quote instead of ambush (rig ×
> length × region), the lifestyle ladder (player-only moves), regional
> Top 100 charts (UK/Europe/America/Japan territories, a Local scene
> board inside the UK, a derived Worldwide), presence-scaled sales,
> record certifications (silver/gold/platinum),
> label recoupment, re-pressing, purchasable indie distribution, and
> contracts with a real clock — terms, breach, a recoupment-dependent
> renewal window, and a label that actively chases its money.
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
| **M1** | Tour economics: rig picker, length picker (1–4 wks), itemized up-front quote, fame-decoupled costs, wear table (design §A) | L | — | `src/game/actions/live.rs` (tour fn + quote helper), tour/rig consts in `src/game/constants.rs`, `touring_costs` section of `data/markets.json`, touring structs in `src/data_loader.rs`, rig/length/quote UI in `src/ui/render/modals/pickers.rs` + its input wiring | ✅ done | sonnet-m1 | money/v0.7 | 024356b |
| **M2** | Lifestyle ladder: tiers, weekly upkeep, stat effects, image, player-only move modal (+10 up / −15 down / −20 eviction), broke eviction (design §B) | M | — | `src/game/lifestyle.rs`, `LifestyleTier` + field in `src/game/player.rs`, `ChangeLifestyle` action in `src/game/actions/rest.rs`, lifestyle consts in `constants.rs`, **new** `src/ui/render/modals/lifestyle.rs` + `mod`/input wiring | ✅ done | sonnet-m2 | money/v0.7 | ed5578b |
| **M3** | Regional Top 100 charts: UK/Europe/America/Japan territories + Local scene board (UK subset, no double-count) + Worldwide aggregation of the four territories, `regions.rs`, presence-gated entry, territory filler (chart-only ambient releases), ramp-in climbers, peak tracking, legacy-save seeding, calmer scene odds + scene territory spread, region-tab UI, movement news (design §C) | L | — | `src/game/world/charts.rs`, **new** `src/game/world/regions.rs`, `regional_charts` on `GameWorld` in `world/mod.rs`, release/submission section of `src/game/world/scene.rs`, `src/ui/render/modals/charts.rs` + tab/scroll input | ✅ done | sonnet-m3 | money/v0.7 | c52cca8 |
| **M4** | Certifications: thresholds, weekly check, award effects, discography badges (design §D) | S | — | `certified` field in `src/game/music.rs`, certification pass + consts (own section of `src/game/economy.rs`), badge lines in `src/ui/render/modals/file.rs` | ✅ done | haiku-m4 | money/v0.7 | 64d9b0a |
| **M5** | Label recoupment (advance + pressing + promo) + label auto-repress (design §E-2, §E-1 label half) | M | M4 | `unrecouped` on `RecordDeal` in `src/game/band.rs`, advance-to-ledger line in `action_sign_deal` (`business.rs`), release-resolution + royalty sections of `src/game/economy.rs`, recoup consts in `constants.rs` | ✅ done | opus-m5 | money/v0.7 | 494c302 |
| **M6** | Indie re-press action + distribution tiers (design §E-1 indie half, §E-3) | M | M5 | `RePress` + distribution choice in `src/game/actions/business.rs`, `distribution_multiplier`/`plan_pressing` in `economy.rs`, distribution consts, re-press/distribution picker UI in `pickers.rs` | ✅ done | sonnet-m6 | money/v0.7 | 6f9e58b |
| **M9** | Deal lifecycle: contract term + albums, free agency at the later of both, breach + `deal_cooldown`, recoupment-dependent renewal window (new contract / extension / silence, opens 26 wks pre-expiry), label memos & recoup pressure (design §E-4, §E-5) | M | M5 | term/`signed_week` fields + fulfillment logic in `src/game/band.rs`, term generation + renewal in `src/game/world/deals.rs`, memos + pressure scaling in `src/game/label_moves.rs`, deal-completion call-site in `economy.rs`, `deal_cooldown` on `Band`, deal-term consts in `constants.rs` | ✅ done | sonnet-m9 | money/v0.7 | 196a17a |
| **M10** | Regional sales wiring: player chart submissions via presence, demand as sum-over-regions in `calculate_release_outcome`, region-named news (design §C — presence + regional sales) | M | M3, M6 | player-side submission + demand sections of `src/game/economy.rs`, presence-related consts in `constants.rs` | ⬜ open | | money/v0.7 | |
| **M7** | Sim-lab validation: homebody / road-dog / indie-lifer bots, measured targets from design §F, [tune] sweeps | M | M1–M6, M9, M10 | `src/game/sim.rs`, `src/game/tests/**` (new test files), Notes below | ⬜ open | | money/v0.7 | |
| **M8** | Cycle close: board audit, CHANGELOG, bump 0.7.0, PR to main | S | all | `HANDOFF.md`, `CHANGELOG.md`, `Cargo.toml`/`Cargo.lock` | ⬜ open | | money/v0.7 | |

### Known overlaps

| Pair | Issue |
|------|-------|
| M4 → M5 → M6/M9 → M10 | All touch `economy.rs` — serialized by prereqs. M4 owns only its new certification section; M5 the release-resolution/royalty paths; M6 the distribution/pressing helpers; M9 only the deal-completion call-site; M10 the submission/demand rewrite last. |
| M3 ∥ M10 | M3 defines the `regions.rs` presence API and owns the scene side; M10 consumes that API for the player side in `economy.rs` — M10 does not edit `regions.rs`/`charts.rs`; API gaps go back to M3 via Notes. |
| M1 ∥ M10 | Both read `regional_fame` — read-only for both; neither reshapes it. |
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
- (cycle start, same day) Second amendment, still pre-claim: charts
  went regional (Top 100 × Local/UK/Europe/America + derived
  Worldwide — M3 now L, new M10 wires player presence/sales),
  certification thresholds scaled ~×3–4 for regional sales, tours
  gained a length picker (M1 now L), and lifestyle moves became
  strictly player-initiated with one-shot happiness swings
  (+10 up / −15 down / −20 eviction).
- **M6 + M9 landed together (integration commit `839d8c2`).** Both Sonnet,
  parallel isolated worktrees, cherry-picked M9 then M6. Only `constants.rs`
  conflicted (both appended a section) plus one M6 test-literal needed M9's
  new `RecordDeal` fields. Review caught one M9 bug: legacy deals
  (`term_weeks == 0`) took phantom deadline stress because `weeks_left`
  saturated to 0 — gated on a real term, regression test added. **182
  passed, 4 ignored**; all 4 balance sweeps green; clippy/fmt clean;
  determinism trio unmodified. M6 review clean (its `current_distribution_channel`
  side-channel deliberately keeps `GameAction::RecordSingle` unchanged so
  the verbatim determinism test still compiles). **Downstream for M10:**
  `distribution_multiplier` now takes the release's own channel and the tail
  loop reaches per-release via `Self::reach_for(fame, market_reach, channel)` —
  preserve that per-release floor in the sum-over-regions rewrite; reuse
  `plan_distribution(channel)` for the fee/gate at any new submission site.
- **M5 landed (`494c302`).** Opus, isolated worktree, cherry-picked clean.
  Review caught one structural bug and sent it back before integrating:
  `label_auto_repress` was wired only into the first-run block, but a label
  run is ~12k copies while Silver is 50k and certifications accrue on the
  catalog tail — so a signed act ran out of stock and could never certify.
  Fixed: the tail loop now re-presses signed releases on stock depletion
  (`wanted >= remaining`) or tail certification, deferred-applied after the
  loop by id, deduped, self-limiting (never over-presses a slow seller).
  Post-integration: **151 passed, 4 ignored**; all 4 balance sweeps green;
  clippy/fmt clean; determinism trio unmodified. **Downstream:** M6 passes
  its royalty income through the new `apply_recoupment` the same way (see
  the release-resolution payout); M9 owns making the ledger survive
  `fulfill_album_obligation` clearing the deal (flagged in `band.rs`).
- **M1–M4 landed together (integration commit `b86f50b`).** Each was
  built in an isolated worktree and cherry-picked onto the branch in the
  order M2 → M1 → M3 → M4; `b86f50b` carries the shared integration fixes
  on top of the four task commits. Post-integration: `cargo test` → **142
  passed, 4 ignored**; all 4 balance-lab sweeps green; clippy `-D warnings`
  and fmt clean; determinism trio unmodified.
  - Review turned up and fixed: **M1** rig capacity multiplier didn't reach
    the gross (bigger rigs cost more for identical earnings) — now folded
    into `total_potential_gross`. **M3** `worldwide_chart` sorted a HashMap
    (non-deterministic tie order) — added a `(title, band_name)` tiebreak;
    scene chart news collapsed to one line per release. **M4** multi-platinum
    over-counted awards past ×2 and the badge was off by one — both fixed,
    regression tests added. **M2** reviewed clean.
  - **Board-file notes for downstream tasks:** M4's badge went in
    `modals/marketing.rs`, not `modals/file.rs` (that's the save/load
    modal — the design/board filename was wrong; there is no discography
    modal yet, so certified back-catalog that isn't a marketing target
    shows no badge — **follow-up for a later UI task**). M3 left a
    documented `TODO(M10)` shim in `economy.rs` (player submits to Local +
    UK only) and, to keep pre-existing mechanics alive, repointed
    `deals.rs::band_buzz` and `turn.rs`'s fame-decay pause at a new
    `GameWorld::player_is_charting()` — **M9 should sanity-check the
    `deals.rs` change when it claims that file.**
- (cycle start, same day) Third amendment, still pre-claim: the home
  scene is a UK city, so **Local is a UK subset** — a scene board, not
  a territory; it never adds into Worldwide or demand (UK gets a 0.1
  home floor instead). **Japan added** as the fourth sales territory
  (second-largest market of the era; already a tour destination) —
  Worldwide aggregates exactly UK/Europe/America/Japan. Territory
  filler added to M3 so four Top-100 boards fill without simulating
  four scenes. Australia stays tour-only.
