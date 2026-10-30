# Research — the UK music industry, 1970–1979

An evidence base for game mechanics, events and economic balancing,
commissioned 2026-10-16 and landed 2026-10-30. This is *research*, not
design: it proposes abstractions but decides nothing, and it stays out
of the version cycle.

The pass covered eight dimensions — the live circuit, musician
earnings, studio and manufacturing costs, record deals, discovery and
promotion, industry change, band pressures, and retail/chart economics
— through parallel research and three targeted resolution passes, then
cross-verified the results and distilled them.

## What is here

| File | What it is |
|---|---|
| `findings.csv` | **The evidence base.** 40 findings, one per row: year, location, category, finding, amount with its original currency and year, what the amount is per, what it includes, a direct source URL, source type, evidence class, confidence, and the gameplay use it suggests. Read this one first — it is diffable and reviewable in a PR. |
| `report.md` | Ten historically grounded game events and five economic mechanics, each linked to finding IDs, plus a check of the game's current assumptions against the evidence. |
| `unresolved-questions.md` | What the sources would not settle: open questions with concrete next-search leads, plus every contradiction found and its resolution status. |
| `evidence/` | The full research trail — eight per-dimension notes with verbatim quotes (`01-live-circuit.md` … `08-retail-charts.md`), the `cross-verification.md` record that assigned each finding's confidence, and three `resolution-*.md` passes that chased the biggest contradictions into contemporary trade press. Everything in `findings.csv` can be traced back through here. |

The CSV is the copy of record: it diffs cleanly in review, while a
spreadsheet's binary format does not. A `findings.xlsx` built from the
same table — with hyperlinked source URLs and sized columns — is kept
outside git for reading and filtering; the two were generated from one
dataset and agree row for row.

## How to read an amount

Amounts keep the currency and year the source gave, and every row says
what the figure is *per* (musician, band, show, ticket, day, week,
album, release, year, market) and what it *includes*. No figure has been
silently converted, and no gap has been filled with an invented price.

Every row also carries an **evidence class**:

- **documented fact** — the source states it as fact: a contract, a
  trade key, official statistics, a news report;
- **anecdote** — recalled by a participant or reported second-hand;
- **estimate** — the source's author inferred it.

Rows drawn from famous acts carry an explicit caveat and were not used
to price beginners. The beginner anchors are F11–F13 (self-released
singles, 1977–78) and F16 (an unproven band signing to a major, 1977).

## The headline for balance work

The single biggest gap is the one the beginner economy leans on hardest:
**no public source gives a per-night fee for an unknown band in a
1970s pub**, and support-slot money terms and booking-agent commission
are likewise unsourced. Everything money-related that *is* documented
about the 1970s live circuit describes established acts — 10cc's
~£1,000 a gig in 1974 (F03) — and says nothing about what an unknown
band was paid. The report handles this by anchoring the early game on
what is documented (self-release costs, wage baselines, the dole) and
keeping the famous-act figures for the mid and late ladder.

Two other findings are worth knowing before anyone touches numbers:

- **BPI awards were value-based, not unit-based, for most of the
  decade** (F38). Album awards were denominated in £ of sales from
  April 1973; unit thresholds of 300k/100k/60k only appear in trade keys
  "as of Jan '79". The widely repeated claim that the change happened in
  January 1978 is contradicted by contemporaneous *Music Week* charts.
  Singles were unit-based throughout (250k/500k/1m).
- **Singles did not collapse in the 1970s** (F32). UK singles deliveries
  hit successive records in 1978 (88.8m) and 1979 (89.1m). Any
  late-decade shift the game models should be value- and margin-driven,
  not a units collapse.

## Provenance and limits

Sources are contemporary where possible — *Sounds* (1974), *Music Week*
scans (1973–1980), Hansard, *Studio Sound*, BBC archives — and are
labelled encyclopedic summary, blog reprint, or recollection where that
is what they are. Confidence was assigned by cross-verification, not by
any single researcher: High means two independent sources agreed or a
primary document carries it.

This is a first pass on one decade. It is deliberately strongest on
costs and prices and weakest on the unknown tier's income, and
`unresolved-questions.md` says exactly where the holes are and what
would close them. Treat the proposed events and mechanics in `report.md`
as design proposals to be argued with, not as settled history.
