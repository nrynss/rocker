# Research — the UK music industry, 1970–1979

An evidence base for game mechanics, events and economic balancing,
commissioned 2026-10-16, landed 2026-10-30, and extended by a second
round on 2026-10-30 that went after the gaps the first pass named. This
is *research*, not design: it proposes abstractions but decides nothing,
and it stays out of the version cycle.

The first pass covered eight dimensions — the live circuit, musician
earnings, studio and manufacturing costs, record deals, discovery and
promotion, industry change, band pressures, and retail/chart economics.
The second pass attacked six clusters of open questions and closed
several of them outright, including the retail chain, which is now
documented end to end for one release.

A third, **design-facing** pass (2026-10-31) asked what the evidence means
for the game: it audited the code against the findings, audited the open
issues, and proposed what would make the world feel inhabited and the
economy honest. That work is in `DESIGN-living-world.md`, not in the
evidence below — it is design, and it is labelled as such.

## What is here

| File | What it is |
|---|---|
| `findings.csv` | **The evidence base.** 72 findings, one per row: year, location, category, finding, amount with its original currency and year, what the amount is per, what it includes, a direct source URL, source type, evidence class, confidence, and the gameplay use it suggests. **F01–F40 are round 1; F41–F72 are round 2.** Read this first — it diffs cleanly and reviews well in a PR. |
| `report.md` | Ten historically grounded game events and five economic mechanics, each linked to finding IDs, plus a check of the game's current assumptions against the evidence. |
| `unresolved-questions.md` | What the sources still will not settle, with the specific archive that would close each one, plus every contradiction found and its resolution status. |
| `evidence/` | The full research trail in two rounds. Round 1: `01-live-circuit.md` … `08-retail-charts.md`, `cross-verification.md`, three `resolution-*.md` passes. Round 2: `round2-*.md`, six notes aimed at the round-1 gaps. Everything in `findings.csv` can be traced back through here. |
| `DESIGN-living-world.md` | **What to build from this evidence.** A synthesis of six design passes: where the code contradicts the history, what the world is missing, and a suggested order of work. Proposals only — nothing here is decided design. Its six backing notes (`design-gap-analysis.md`, `design-living-scene.md`, `design-living-industry.md`, `design-survival-layer.md`, `design-live-ladder.md`, `open-issues-audit.md`) are in `evidence/`. Item 1 of its suggested order, the **S-UI record picker**, shipped 2026-10-31. |

The CSV is the copy of record: it diffs cleanly in review, while a
spreadsheet's binary format does not. A `findings.xlsx` built from the
same table — with hyperlinked source URLs and sized columns — is kept
outside git for reading and filtering.

## How to read an amount

Amounts keep the currency and year the source gave, and every row says
what the figure is *per* (musician, band, show, ticket, day, week,
album, release, year, market) and what it *includes*. No figure has been
silently converted, and no gap has been filled with an invented price.

Every row also carries an **evidence class**:

- **documented fact** — the source states it as fact: a contract, a
  trade rate card, official statistics, a news report;
- **anecdote** — recalled by a participant or reported second-hand;
- **estimate** — the source's author inferred it, or the figure is
  derived by arithmetic on documented prices (and says so).

Rows drawn from famous acts carry an explicit caveat and were not used
to price beginners. The beginner anchors are F11–F13 (self-released
singles, 1977–78) and F16 (an unproven band signing to a major, 1977).

## What the evidence now supports

**The retail chain is documented on the retail side.** One 1979 release
gives retail price, dealer price and margin that reconcile to the penny:
Island's *Broken English* at £5.00 retail, £2.61 dealer price ex-VAT, a
40% dealer margin (F51). The standard trade term was a "full 36⅔ per
cent" margin on sale-or-return (F52). The shop therefore buys at roughly
63% of RRP and keeps the rest, with VAT inside the ticket. What remains
undivided is the wholesale block itself — nothing splits the label's
share from the distributor's (F53), so hold the dealer price as fact and
treat the internal split as a tunable.

**Representation has real numbers.** A Court of Appeal recital of an
actual 1970 UK management agreement gives 20% of *all* earnings, gross,
with production thrown in free (F41) — and the same file shows a
management contract and a booking-agency contract signed the same day
as two separate instruments (F42). Agency commission ran about 10% for
an established act but 15–20% for a young band's first agency (F44): early
representation cost more, not less.

**Equipment is priced properly at last.** New copy-grade guitars ran
£36.57–£45, a 4-drum kit £122.94, a basic PA £71.30 (F56); used
professional gear was £160–£170 for a guitar or kit (F57). The
counter-intuitive find that should drive the equipment ladder: in 1975 a
new-but-budget rig was *cheaper* than a used-professional one (F58), so
bands moved from new cheap to used pro — quality, not novelty, was the
upgrade.

**Two findings correct assumptions likely to be in the code.** BPI album
awards were denominated in £ of sales from April 1973; unit thresholds
(300k/100k/60k) only appear in *Music Week* chart keys "as of Jan '79",
contradicting the commonly repeated January 1978 date (F38). And singles
did not collapse: UK deliveries hit successive records in 1978 (88.8m)
and 1979 (89.1m) (F32), so any late-decade shift has to be value- and
margin-driven.

## What is still missing

The beginner's night's pay is still not nailed. The closest documented
figure is £25–£45 per band per night for a small local band — but its
fieldwork window is c.1980–84, not the 1970s, and it is flagged as such
rather than backdated (F69). Support slots have no documented money term
at all in two rounds of searching (F71), and producer points defeated
both passes (F50). The honest state of knowledge is that the *cost* side
of a 1970s career is now well evidenced and the *income* side of the
unknown tier still is not.

## Provenance and limits

Sources are contemporary where possible — *Sounds* (1974), *Music Week*
scans (1973–1980), *Beat Instrumental* price lists (1971–1977), Hansard,
primary legislation, *Studio Sound*, BBC archives — and are labelled
encyclopedic summary, blog reprint, or recollection where that is what
they are. Confidence was assigned by cross-verification, not by any
single researcher: High means two independent sources agreed or a
primary document carries it.

The strongest single sources this pass are a Court of Appeal judgment
reciting real contract terms, primary legislation on benefit rates, and
trade rate cards printed inside the papers themselves. The weakest are
recollections and retrospective trade books, which are labelled as such
and never used alone for a headline number.

Treat the proposed events and mechanics in `report.md` as design
proposals to be argued with, not as settled history.
