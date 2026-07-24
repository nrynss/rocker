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
| **L-SCALE** | Decompress to one era-real money scale (§B): retire `SALES_INCOME_DIVISOR`, set era-real per-copy income, scale lifestyle/tour/recording/pressing/advance costs in lockstep; re-validate every v0.7 §F ratio in the sim | L | — | income/cost consts across `constants.rs`, `economy.rs` payout math, `data/record_labels.json` advance ranges, coupled cost sites (documented "COUPLED" comments) | ✅ done | agent-lscale (worktree) | dbb803d |
| **L-ENTITY** | `Label` world entity + `world/labels.rs` (§A): struct, worldgen seeding from `record_labels.json`, weekly wholesale ledger booking margin on all roster copies, capital debits on advance/pressing/promo | L | L-SCALE | **new** `src/game/world/labels.rs`, `labels` map on `GameWorld` (`world/mod.rs`), ledger hook in `economy.rs` sales pass, label consts in `constants.rs` | ✅ done | agent-lentity (worktree) | 9106e1d |
| **L-REACH** | Dynamic reach + tier drift (§C): target from capital/hit-window, bounded yearly drift with hysteresis, tier thresholds, live deal reads through to label reach (catalogue tail still freezes `label_market_reach`) | M | L-ENTITY | reach/tier drift in `world/labels.rs`, deal→label read-through in `band.rs`/`economy.rs`, drift consts | ✅ done | agent-lreach (worktree) | e9b329c |
| **L-ROSTER** | Scene-band signing + roster P&L (§D): labels scout/sign scene acts by fit+appetite; roster releases feed label capital; rival act→label visibility | M | L-ENTITY | roster/scouting in `world/labels.rs` + `world/scene.rs` signing hook, roster consts | ✅ done | agent-lroster (worktree) | 3122c25 |
| **L-BID** | Competition + bidding (§E): offers priced from live label state, multiple concurrent offers, bidding escalation bounded by capital/financial_power | M | L-REACH, L-ROSTER | offer generation + pricing in `world/deals.rs`, bidding consts | 🔒 claimed | agent-lbid (worktree) | — |
| **L-MNA** | Mergers, acquisitions, folding (§F): distress state, acquisition (roster+deal transfer), folding (deal void via breach write-off path), optional founding gate | M | L-REACH, L-ROSTER | M&A + status transitions in `world/labels.rs`, deal-transfer/void call-sites in `band.rs`/`economy.rs`, M&A consts | 🔒 claimed | agent-lmna (worktree) | — |
| **L-CHAR** | Label character / A&R (§G): `LabelStrategy` enum + creative-freedom effects wired into offer pricing, single-cut aggression/memos, renewal behavior | M | L-BID | `LabelStrategy` in `world/labels.rs`, hooks in `label_moves.rs` + `world/deals.rs`, strategy consts | ⬜ open | — | — |
| **L-CERT** | Year-gate certifications to the era (§I): no scheme <1973, no Platinum <1976, no multi-platinum <1984; thresholds unchanged, only availability gates on year | S | — | certification pass in `economy.rs`, year-gate consts in `constants.rs` | ✅ done | agent-lcert (worktree) | c2be9a0 |
| **L-UI-1** | **UI slice 1** (user ask, 2026-07-24): **per-record sales in the band's discography** (copies_sold, income, chart peak, cert — the "no way to see each record's sales" gap) + a label-ledger browser (capital/tier/reach/roster of already-built fields). Read-only display; bidding/M&A/news annotations deferred to L-UI-2. | M | L-ROSTER | discography modal `src/ui/render/modals/file.rs`, **new** `src/ui/render/modals/{discography,labels}.rs` + mod/input wiring in `src/ui/app.rs`+`input/main.rs` | ✅ done | agent-lui1 (worktree) | 0cae070 |
| **L-UI-2** | UI slice 2 (§H): deal-picker health/trajectory annotations + industry news lines (mergers/foldings/distress/rival breakouts) at their source sites | S | L-BID, L-MNA, L-UI-1 | deal-picker annotations in `pickers.rs`, news lines at their source sites | ⬜ open | — | — |
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
  both flagged calls settled — §B **one honest scale**, §F **founding
  off** to start.

- 2026-07-24 **L-SCALE scouting (measured, not yet committed).** Retiring
  `SALES_INCOME_DIVISOR` alone (3 → 1) was built and swept: it works
  directionally — record income triples, so med net worth over 15 yrs
  rose (label-loyalist $2.56M → $3.96M; studio-rat climbed out of the red,
  −$116 → +$92.9k, bankruptcies 10 → 6). **But it skews the game toward
  records:** tour/gig income stayed flat, so road-dog's wins collapsed
  2 → 0. Confirmed: the divisor must be retired *only in lockstep* with a
  uniform ×3 of every other dollar. Reverted to keep the branch green; the
  full renumbering below is L-SCALE's actual job.

  **Income sources (must all end at the same scale):** record catalog gross
  (`economy.rs:~849`, already ×3 once the divisor is retired) · tour box
  office (`TOUR_GROSS_COEFFICIENT` 0.15 → ×3, `live.rs` `base_gross`) · gig
  earnings (`live.rs:308` — trace its formula/constant) · support-slot pay
  (`business.rs:288` `offer.pay` — scale at its generation site) · signing
  advance (`data/record_labels.json`, see the caveat).

  **Cost constants to ×3:** `SINGLE_RECORDING_COST` (100), `ALBUM_RECORDING_
  BASE_COST` (1000), `DOCTOR_VISIT_COST` (50) in `src/data/constants.rs` ·
  `LIFESTYLE_UPKEEP_PER_WEEK` · `TOUR_RIG_COST_PER_WEEK` ·
  `PRESSING_SETUP_SINGLE/ALBUM` · `DISTRIBUTION_CHANNEL_FEE` in
  `constants.rs` · marketing campaign `cost:` values in `music.rs` (100…2000).
  `PRESSING_PER_COPY_*` auto-triple with the divisor; **copy counts do NOT
  scale** (`PRESSING_TIERS`, cert thresholds, `UNITS_PER_SCORE_POINT`).

  **`LABEL_RECOUP_PRESSING_PER_COPY`** 0.05 → 0.15 (un-divide, keep recoup
  coupling — see its comment).

  **Caveat / cross-check with the recoupment concern (parked from PR #22):**
  scaling advances ×3 preserves the advance-to-income ratio (recoupment stays
  as hard as today). *Leaving advances unscaled* instead makes recoupment ~3×
  easier and shrinks the windfall — directly addressing "major advances never
  recoup." Pick one deliberately and let L-SIM's "signed nets ≥ indie over a
  full deal" + "median recoup < half term" targets referee it.

  **Exact-value tests will break** (income/recoup dollar assertions in
  `tests/recoupment.rs`, `tests/deals.rs`, smoke). Recompute, don't delete.
  Determinism trio stays unmodified.

- 2026-07-24 **L-UI-1 integrated (`0cae070`), clean fast-forward.** 218 pass,
  clippy/fmt clean, determinism green. Per-record sales now visible: a **new
  Discography browser** (`d`) shows each record's copies/earnings/peak/cert;
  a **label-ledger browser** (`b`) shows tiers/reach/roster, exact capital for
  the player's own label, qualitative bands for others (§H). **Board pointer
  correction:** the L-UI-1 row (and my note) said `modals/file.rs` — that's the
  Save/Load modal; no discography view existed, so the agent correctly built a
  dedicated `modals/discography.rs`. **Coupling flag:** the UI's qualitative
  capital bands duplicate the game's `TIER_*_MIN_CAPITAL` thresholds ($200k/$2M)
  as UI-local consts (the game ones are `pub(super)`); keep in step if they move.

- 2026-07-24 **Wave 4 dispatched + L-UI reordered forward.** User asked (mid-
  cycle) for **per-record sales in the UI** ("no way to see the sales of each
  record") and to stop checkpoint-asking and just drive the loop. So: L-UI split
  into **L-UI-1** (per-record sales in the discography + a label-ledger browser
  of already-built fields — buildable now on L-ROSTER) and **L-UI-2** (bidding/
  M&A/news annotations, needs wave 4). Dispatched **three parallel worktree
  agents**: L-UI-1 (`ui/` only), L-BID (`world/deals.rs`), L-MNA (`world/labels.rs`
  status + `band.rs`/`economy.rs` deal-transfer call-sites) — mostly disjoint
  files, integrate sequentially. L-CHAR still gated on L-BID (both touch
  `deals.rs`). Coordinator owns the board; agents ff-merge origin/label/v0.8
  first, commit, report SHA, don't push, don't touch HANDOFF.md.

- 2026-07-24 **Wave 3 complete: L-ROSTER integrated (`3122c25`), 3-way over
  L-REACH.** Three additive conflicts (the `use` block, the const block, the
  test block in `constants.rs`/`world/labels.rs`/`tests/labels.rs`) resolved by
  keeping both sides; fmt fix folded in. Suite **216 pass** (+4), determinism
  trio unmodified, clippy/fmt clean, save-compat green. **Combined solvency
  sweep** (reach drift + active rosters together): **0 insolvent**, capital
  $401k–$24.8M — sane, no explosion. The industry now lives: labels sign scene
  acts by fit+appetite (233/260 signed), rosters book chart-presence margin, and
  reach/tier drift off the resulting capital. **Deferred (not a gap):** scene
  chart-hits are not wired into L-REACH's `hit_window` recency bonus — roster
  success already drives reach via the `capital` pathway; the explicit
  `record_chart_hit` feed for scene acts needs stable scene-release ids, left as
  a minor L-SIM/L-UI-era enhancement. **Half the cycle done (5/11).** Remaining:
  wave 4 = L-BID (priced offers + bidding) + L-MNA (distress/acquisition/folding)
  + L-CHAR (strategy/creative-freedom effects — **prereq L-BID**), then L-UI,
  with L-SIM the continuous [tune] referee. L-BID + L-MNA parallelize (deals.rs
  vs labels.rs status); L-CHAR follows L-BID (both touch deals.rs).

- 2026-07-24 **Wave 3a integrated: L-REACH (`e9b329c`), clean cherry-pick.**
  212 pass (+8), determinism trio unmodified, clippy/fmt clean, solvency sweep
  still 0 insolvent. Reach/tier now drift: target ≈ ∛(capital) (the inverse of
  L-ENTITY's seed) + a bounded recent-hits bonus, moving ≤2 pts/month with a
  3-pt hysteresis dead-band; tier promotes on reach≥70 & capital≥$2M (Major) /
  reach≥38 & capital≥$200k (Indie), demotes on reach alone. A signed act's NEW
  releases read the label's live reach (via new `Game::new_release_frozen_reach`
  in `studio.rs`); catalogue tails keep their frozen reach (invariant intact).
  New GameWorld field `label_reach_tick` + `drift_labels()` called in
  `update_week` AFTER the scene passes (so L-ROSTER's scene hits are in the
  window first). **Reconciliation notes for L-ROSTER:** L-REACH added
  `Label::{record_chart_hit,reach_target,drift_reach_and_tier,tier_for}` and
  `GameWorld::drift_labels`; `record_chart_hit` is the entry point for
  L-ROSTER's scene-hit feed (hook left in `drift_labels` + the economy
  hit-recording site). **L-SIM [tune] flags:** seeded majors settle ~2 pts off
  their JSON reach at rest (target is capital-derived); hit_window is
  capacity-based (Vec<u32>, can't reshape) so a cold-but-rich label plateaus
  rather than cooling — real cooling needs L-ROSTER/L-MNA flop-bleed.

- 2026-07-24 **L-ENTITY verified at scale** (`label_capital_trajectories_over_fifteen_years`, an L-SIM-preview `#[ignore]` sweep committed to `sim.rs`; run it with `--ignored --nocapture`). 24 seeds × 15 yrs, label-loyalist: **0 insolvent labels** (lowest capital anywhere $25,920, a static boutique seed; highest $12.86M), and the player's label nets **+$2.4M…+$7.5M** — a productive signed act makes its label millions; ledger sound, gangbuster money sane. **Finding that shapes L-ROSTER/L-BID:** the loyalist only ever signs *indies* (renewal keeps it on its first label — PR #22's parked item) so **majors sit fully static** at seed capital (no roster activity until scene bands sign). No bot exercises the major-label path yet — L-ROSTER makes majors book margin off rosters; the major path needs L-BID (a big act reaching a major) before its economics can be swept.

- 2026-07-24 **Wave 2 integrated: L-ENTITY (`9106e1d`), fast-forward.** The
  agent ff-merged `origin/label/v0.8` first (worktree-base lesson applied), so
  it landed clean. Suite **204 pass**, determinism trio unmodified, clippy/fmt
  clean, save-compat covered (labels + `label_id` round-trip; pre-0.8 load;
  v0.4 fixture green). Labels are now world entities with a balance sheet:
  seeded from `record_labels.json` (capital = financial_power³ × 15 → Columbia
  ~$12.9M … SST ~$26k), and a signed act's sales book the ~$2.70/copy wholesale
  spread to its label's `capital` (Gold ~$405k, Platinum ~$1.08M; a sustained
  hit booked $1.79M). `label_id` added to `RecordDeal` links the player's deal
  to its entity. **L-SIM watch-item / accounting seam:** per-copy
  `manufacturing_cost` ($1.00 COGS on a sold copy) and the pressing outlay
  debited to capital ($0.15/copy pressed, recoup-coupled) both touch the
  physical run at different magnitudes — implemented per §A, flagged [tune] in
  `constants.rs` for L-SIM to referee. Forward-compat stubs left with
  `// L-REACH/L-ROSTER/L-MNA/L-CHAR will` markers. **Wave 3 (L-REACH + L-ROSTER)
  now unblocked** — both extend `world/labels.rs`, so integrate sequentially.

- 2026-07-24 **Wave 1 integrated.** Both L-CERT (`c2be9a0`) and L-SCALE
  (`dbb803d`) are on `label/v0.8`; combined suite **195 pass**, determinism
  trio unmodified, clippy/fmt clean. L-SCALE cherry-picked over L-CERT with
  one trivial conflict (the `SALES_INCOME_DIVISOR` comment block, resolved to
  L-SCALE's `=1`). **Validation sweep:** every bot's median 15-yr net worth
  ≈3× the pre-scale baseline (2.8×–3.8×) — uniform decompression confirmed,
  the step-(a) record-skew gone (road-dog's money tripled with everyone's).
  Gangbuster money real: signed loyalist $6.3M. No deaths; broke-counts
  normal. **L-SIM watch-items (formalize as assertions, then tune):**
  (a) road-dog now out-earns the release grinders by ~1.4× and lands 0/60
  wins — the sales fix's touring-carries-records-abroad interacts with the
  ×3 tour gross; decide whether §F's "road-dog must not out-earn grinders"
  still holds or is superseded by the touring-carry design; (b) studio-rat
  marginal ($4k median, 15/60 broke — never dies). Neither blocks the cycle.

- 2026-07-24 **Note on worktree bases:** the Agent-tool worktrees checked
  out at `main`, not `label/v0.8`. L-CERT self-aligned (ff-merge) so it
  landed as a clean fast-forward; L-SCALE did not (its edits sat on main,
  uncommitted), so the coordinator committed them in the worktree and
  cherry-picked onto `label/v0.8` (3-way against the common base). Future
  waves: tell dispatched agents to `git merge --ff-only origin/label/v0.8`
  into their worktree FIRST, and to commit + report their SHA.

- 2026-07-24 **Wave 1 dispatched to worktree agents.** L-SCALE and L-CERT
  are the only tasks with prereqs met, and they touch only different
  *regions* of the two shared files (`constants.rs`, `economy.rs`), so they
  run in parallel in isolated worktrees off `label/v0.8` HEAD. **During
  worktree dispatch the coordinator owns this board** — agents do NOT edit
  `HANDOFF.md` (it would conflict across worktrees); they commit only their
  Owns paths, don't push, and report their SHA. The coordinator integrates
  into `label/v0.8`, runs the full suite + sweeps, and flips the rows to
  `✅ done`. Next wave (L-ENTITY) unlocks once L-SCALE integrates.
