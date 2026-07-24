# Rocker — Label Cycle Handoff (v0.8)

> **Feature cycle: the record labels become a living economy.** A label
> is no longer inert JSON — it is a world entity with a balance sheet that
> rises on hits and falls on flops, a roster of real acts, reach and tier
> that grow and decline, competing offers and bidding for talent, mergers
> and failures, and — underneath it all — a money scale where a superstar
> and their label both earn what the era actually paid. Record companies
> and big bands made gangbuster money; this cycle puts that money on the
> board and lets it move.
>
> **Design is proposed, awaiting coordinator sign-off** — read
> `docs/DESIGN-v0.8-label-cycle.md` before claiming anything. Numbers
> marked [tune] there are sim-lab candidates validated in L-SIM, not
> invented per-agent. Two coordinator calls are flagged in the doc (§B
> money-scale: one honest scale vs dual-scale; §F: label founding on/off)
> — settle those before L-SCALE starts.
>
> **Foundation already in the branch:** the record-sales fix that gave a
> label's `market_reach` real international distribution
> (`international_floor_for`, all four presence sites). This cycle makes
> that reach *move*. Do not revert it.
>
> Prior cycles: `docs/archive/HANDOFF-v0.7-cycle.md` (money),
> `docs/archive/HANDOFF-v0.6-cycle.md` (life),
> `docs/archive/HANDOFF-v0.5-cycle.md` (features),
> `docs/archive/HANDOFF-v0.5.1-structure.md` (structure) · Shipped:
> `CHANGELOG.md` · North star for a later cycle: `FUTURE.md` (Musician).
> **Deferred:** addiction (§9.1), vacations picker (§9.3), manager
> (§9.4), the whole Musician plan (FUTURE §1–§8) — do not implement, do
> not remove the dormant fields.

Baseline at cycle start: **0.7.1 + the sales fix**, `cargo test` →
**188 passed, 5 ignored** (+ balance sweeps green), clippy `-D warnings`
clean, fmt clean, determinism trio unmodified. Cycle closes as **0.8.0**.

---

## How agents claim work

Same protocol as prior cycles — this file is the **single coordination
surface**. Do not start a task that is claimed or whose prerequisites are
not ✅.

### One branch for the whole cycle

**All work lands on `label/v0.8`.** No per-task branches, no parallel
task PRs — the cycle PR merges once, at cycle close, when the human says
so. Only commits to the shared branch matter.

```text
git fetch origin
git checkout label/v0.8
git pull --rebase origin label/v0.8   # always before claim / before push
# … do one claimed task …
git push origin label/v0.8
```

If `pull --rebase` conflicts in files you do **not** own, stop and
unclaim — do not "fix" another agent's WIP.

### Claim protocol (mandatory)

1. **On `label/v0.8`**, up to date with `origin`.
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

Commit prefix: `label(L-XXX): <short description>`.

### Ground rules (every agent)

- **This cycle changes behavior on purpose** — but only *your task's*
  behavior. No drive-by edits outside your Owns; every [tune] number you
  set must match the design doc or carry a Notes line saying what you
  changed and why.
- **L-SCALE goes first and touches every dollar.** Until L-SCALE is
  `✅ done`, do not hardcode dollar figures in later tasks — read the
  constants it establishes. It is the one task allowed broad reach into
  cost/income constants across modules; coordinate around it.
- **Tests:** counts will grow and some assertions will legitimately
  change. Never delete a test to make it pass; adapt it and say so in the
  commit. The determinism trio
  (`same_seed_and_same_choices_replay_the_same_career`,
  `seeded_worlds_are_reproducible_in_the_harness`,
  `worldgen_is_reproducible_per_seed`) must pass **unmodified**.
- **Serde / saves:** every new field `#[serde(default)]`. Never rename
  serialized fields (`energy` and addiction fields stay, dormant).
  `saves_from_v0_4_still_load` and the pre-0.5 fixture are sacred.
- **World RNG is injected, not ambient** — label scouting, bidding, and
  M&A rolls derive from `world_seed`.

---

## Task board

| ID | Task | Size | Prereqs | Owns (exclusive) | Status | Claimed by | Done |
|----|------|------|---------|------------------|--------|------------|------|
| **L-SCALE** | Decompress to one era-real money scale (§B): retire `SALES_INCOME_DIVISOR`, set era-real per-copy income, scale lifestyle/tour/recording/pressing/advance costs in lockstep; re-validate every v0.7 §F ratio in the sim | L | — | income/cost consts across `constants.rs`, `economy.rs` payout math, `data/record_labels.json` advance ranges, coupled cost sites (documented "COUPLED" comments) | ⬜ open | — | — |
| **L-ENTITY** | `Label` world entity + `world/labels.rs` (§A): struct, worldgen seeding from `record_labels.json`, weekly wholesale ledger booking margin on all roster copies, capital debits on advance/pressing/promo | L | L-SCALE | **new** `src/game/world/labels.rs`, `labels` map on `GameWorld` (`world/mod.rs`), ledger hook in `economy.rs` sales pass, label consts in `constants.rs` | ⬜ open | — | — |
| **L-REACH** | Dynamic reach + tier drift (§C): target from capital/hit-window, bounded yearly drift with hysteresis, tier thresholds, live deal reads through to label reach (catalogue tail still freezes `label_market_reach`) | M | L-ENTITY | reach/tier drift in `world/labels.rs`, deal→label read-through in `band.rs`/`economy.rs`, drift consts | ⬜ open | — | — |
| **L-ROSTER** | Scene-band signing + roster P&L (§D): labels scout/sign scene acts by fit+appetite; roster releases feed label capital; rival act→label visibility | M | L-ENTITY | roster/scouting in `world/labels.rs` + `world/scene.rs` signing hook, roster consts | ⬜ open | — | — |
| **L-BID** | Competition + bidding (§E): offers priced from live label state, multiple concurrent offers, bidding escalation bounded by capital/financial_power | M | L-REACH, L-ROSTER | offer generation + pricing in `world/deals.rs`, bidding consts | ⬜ open | — | — |
| **L-MNA** | Mergers, acquisitions, folding (§F): distress state, acquisition (roster+deal transfer), folding (deal void via breach write-off path), optional founding gate | M | L-REACH, L-ROSTER | M&A + status transitions in `world/labels.rs`, deal-transfer/void call-sites in `band.rs`/`economy.rs`, M&A consts | ⬜ open | — | — |
| **L-CHAR** | Label character / A&R (§G): `LabelStrategy` enum + creative-freedom effects wired into offer pricing, single-cut aggression/memos, renewal behavior | M | L-BID | `LabelStrategy` in `world/labels.rs`, hooks in `label_moves.rs` + `world/deals.rs`, strategy consts | ⬜ open | — | — |
| **L-CERT** | Year-gate certifications to the era (§I): no scheme <1973, no Platinum <1976, no multi-platinum <1984; thresholds unchanged, only availability gates on year | S | — | certification pass in `economy.rs`, year-gate consts in `constants.rs` | ⬜ open | — | — |
| **L-UI** | Label ledger/browser modal, deal-picker health/trajectory annotations, industry news lines (§H) | M | L-REACH, L-BID, L-MNA | **new** `src/ui/render/modals/labels.rs` + mod/input wiring, deal-picker annotations in `pickers.rs`, news lines at their source sites | ⬜ open | — | — |
| **L-SIM** | Sim-lab validation (§I): label-solvency + gangbuster-target + bidding + graceful-failure bots and assertions; scale re-validation harness for L-SCALE | M | L-SCALE (continuous thereafter) | `src/game/sim.rs`, `src/game/tests/**` (new files) | ⬜ open | — | — |
| **L-CLOSE** | Cycle close: board audit, CHANGELOG, bump 0.8.0, archive this board, PR to main | S | all | `HANDOFF.md`, `CHANGELOG.md`, `Cargo.toml`/`Cargo.lock` | ⬜ open | — | — |

Dependency spine: **L-SCALE → L-ENTITY → {L-REACH, L-ROSTER} → {L-BID,
L-MNA, L-CHAR} → L-UI**. L-CERT is independent (claimable anytime).
L-SIM opens with L-SCALE and runs continuously as the [tune] contract.

### Known overlaps

- **`world/labels.rs`** is written by L-ENTITY and extended by L-REACH,
  L-ROSTER, L-MNA, L-CHAR. L-ENTITY lands the module and its public shape
  first; downstream tasks add methods, minimum diff, no reshaping the
  struct without a Notes line.
- **`economy.rs` sales pass** is touched by L-SCALE (payout math),
  L-ENTITY (ledger hook), and L-CERT (year gate). These are different
  regions of the pass; coordinate on the shared borrow structure and land
  L-SCALE first.
- **`world/deals.rs`** is touched by L-BID (pricing) and L-CHAR
  (strategy) — L-BID establishes the priced-offer shape, L-CHAR layers
  strategy onto it.

### Notes

- 2026-07-24: board opened by the coordinator. Foundation (sales fix) is
  in the branch. Design doc `docs/DESIGN-v0.8-label-cycle.md` proposed;
  the two flagged coordinator calls (§B scale model, §F founding) should
  be settled before L-SCALE is claimed.
