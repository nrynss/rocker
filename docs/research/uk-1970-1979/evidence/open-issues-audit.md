## Open issues audit

Audited 2026-10-31 against the code at `main` (plus this branch). The repo has
**no open GitHub issues** — both filed issues (#20, #21) are closed — so the
real open work lives in the docs. This audit verified every doc claim against
the code rather than repeating it.

### Every open item found

| # | Source | What it says | Still real? | Blocking? | Size |
|---|---|---|---|---|---|
| 1 | `HANDOFF.md:65` | **S-UI** — record picker (format → songs → studio, quote-first), activity screen, discography split | **Yes, and worse than the board says.** `ui/render/modals/pickers.rs:13-16` books `RecordingStudio::default()` (the neutral demo room) with the format's default song count, always. The five-room ladder (`studios.rs:49`), the player's song choice and the multi-week session are unreachable in play. | **Yes** — the v0.9 Studio Cycle's headline feature cannot be used | M |
| 2 | `HANDOFF.md:66` | **S-SIM** — bots for the new action shape, studio-ladder and singles-vs-albums balance, calendar-integrity assertion, §F economic-neutrality check | **Yes.** `HANDOFF.md:140` records a red studio-rat assertion (20/60 vs ≤19). Not re-measured this pass: the relevant test is `#[ignore]`d and the release sweep did not finish. | Yes — balance is unvalidated for the new action shape | M |
| 3 | `CHANGELOG.md` "Known follow-ups" | M&A distress floor is `$0`, so it never fires in natural play | **Yes — but it is working as designed; the design is the problem.** `constants.rs:1131` `DISTRESS_CAPITAL_FLOOR = 0` only ticks on strictly negative capital, while `sim.rs:1430` asserts `min_capital_any > 0`; `sim.rs:1320-1332` records that raising the floor was *rejected*. So label attrition cannot occur in natural play — only under injected distress. | No | S |
| 4 | `CHANGELOG.md` "Known follow-ups" | Certification frequency is ~71 events over a 15-year signed career, an order above the retired v0.7 target | **Yes, as a judgement call.** Not a bug: it follows from the international-reach fix. Needs a design decision, not a code fix. | No | S |
| 5 | `FUTURE.md` §9.1 | Addiction with teeth — deferred | **Yes, dormant.** Explicitly deferred by every cycle to date. | No | L |
| 6 | `FUTURE.md` §9.3 | Vacations picker — deferred | **Yes, dormant.** | No | M |
| 7 | `FUTURE.md` §9.4 | **The Manager** — deferred, and now directly supported by new evidence (F41, F42, F44) | **Yes, dormant** — and this is the best-evidenced of the deferred features. | No | M |
| 8 | `FUTURE.md` §1–§8 | The Musician plan (struct, genre proficiency, solo/band, relationships, chart hardening, UI) | **Yes, dormant.** Current code has no `Musician` struct. | No | XL |
| 9 | `FUTURE.md` §10.1 | Music rights and ownership | **Yes, dormant.** Related to the new publishing evidence (F19, F20, F47). | No | L |
| 10 | `FUTURE.md` §5 | Uncapped scene population, genre tag on chart entries | **Yes, dormant** — note §5's "no special eviction" already shipped. | No | S |

**Housekeeping (2):** the v0.9 cycle has not closed (version is still 0.8.0
per `Cargo.toml`, though the board treats `studio/v0.9` as merged and `main`
sits past #25) — the CHANGELOG has no 0.9.0 entry yet; and `docs/archive/`
holds four closed cycle boards that could take the v0.9 board once it closes.

### Items already fixed or stale

- **Full-production tours never profiting** — struck through in the CHANGELOG,
  resolved by the tour-economics pass (covers #25).
- **No open GitHub issues.** #20 (label recoupment impossible below 30%) and
  #21 (sold-out log ordering) are both closed.
- **Zero `TODO`/`FIXME`/`HACK`/`XXX` markers** in `src/` — the codebase does
  not carry that kind of debt.
- **`cargo test` = 306 passed, 8 ignored** at the time of audit.

### Dormant fields that must not be removed

`FUTURE.md` §9.1 carries a standing instruction repeated in every cycle
header: *deferred explicitly — do not implement them here; do not remove the
dormant fields.* The dormant surface is the addiction/happiness/vacation/
manager state on the band. **Any new work must not delete or repurpose these
fields**, even where a new feature overlaps — the Manager work in particular
must extend the dormant structure rather than replace it.

### Ignored tests

Eight tests are `#[ignore]`d, including the S-SIM studio-rat balance assertion
(`HANDOFF.md:140`). These are balance-lab tests, not correctness tests: they
are expensive and are run deliberately. **They still matter** — item 2 above
cannot be closed without re-running them.

### Ranked backlog: what to do next

| Rank | Item | Why now | Depends on | Size | Evidence support |
|---|---|---|---|---|---|
| 1 | **S-UI** record picker | A whole cycle's headline is unreachable — players cannot choose a studio, songs or session length. Everything else is polish by comparison. | — | M | — (UI work) |
| 2 | **S-SIM** re-validation | Balance for the new action shape is unproven, and one assertion is known red. Cannot ship 0.9 without it. | S-UI | M | — |
| 3 | **Close the v0.9 cycle** | Version still 0.8.0; CHANGELOG has no 0.9.0 entry; the board has open rows. | 1, 2 | S | — |
| 4 | **The Manager** (FUTURE §9.4) | Now the best-evidenced deferred feature: 20% of all earnings gross (F41), agent vs manager as separate contracts (F42), 15–20% for a first agency (F44). No more guessing. | — | M | **Strong** (F41, F42, F44) |
| 5 | **Publishing as a second income stream** | Currently absent entirely; the evidence now gives a full picture (F47 double split, F48 6.25% mechanical, F49 writer's share, F19/F20 terms). | Manager | L | **Strong** (F19, F20, F47–F49) |
| 6 | **Retail chain in the money model** | The £5.00 → £2.61 → 40% chain is penny-exact (F51, F52); the label/distributor split is a documented gap to tune (F53). | — | M | **Strong** (F51–F53) |
| 7 | **Scene lifecycle** | The scene has no named exits or arrivals; a state machine with news lines is the cheapest way to make the world feel inhabited. | — | M | Moderate (F05, F22, F69, F72) |
| 8 | **Survival layer** (dole, day job, gear, van) | The early game has no cost of living; evidence now prices it (F56–F64). | — | L | **Strong** (F56–F64) |
| 9 | **M&A distress floor** | Small, self-contained, and the code comment already records the rejected fix — a design decision is needed, not analysis. | — | S | — |
| 10 | **Certification frequency** | A judgement call on how generous a signed career should feel; cheap to retune once the sim is re-validated. | S-SIM | S | Moderate (F38) |

### The one thing to do first

**Build S-UI.** The Studio Cycle was designed around a player choosing a
format, choosing songs and choosing one of five studios, with a quote before
booking — and the build does none of it: `pickers.rs:13-16` silently books the
default room with the default song count every time. Until that picker
exists, the studio ladder, the per-song cost, the weeks-per-song calendar and
the quote-first flow are all dead code that the player never meets. Every
other item on this list — balance, the cycle close, the evidence-backed
features in ranks 4–8 — is worth less while the headline feature is
unreachable.
