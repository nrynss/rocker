# Making the world living — a synthesis

> **Status: design proposals, not decided design.** Unlike the
> `DESIGN-v0.x` cycle documents, nothing here is agreed or scoped. This is
> the output of six research/design passes run on 2026-10-31 against the
> evidence base in `findings.csv` (72 findings), and it exists to be argued
> with and then turned into a cycle brief.

Six design passes were run on 2026-10-31 against the evidence base in
`findings.csv` (72 findings) and the shipped code. This document is the
synthesis: what they found, what it means, and what to do about it in
what order. Each pass has its own note in this directory; this one
exists so a reader does not have to hold six documents at once.

| Pass | Note | Headline |
|---|---|---|
| Gap analysis | `design-gap-analysis.md` | **14 places the code contradicts the evidence**, 14 unanchored, 8 fine |
| Open issues | `open-issues-audit.md` | 12 real open items; **the v0.9 headline feature is unreachable in play** |
| Living scene | `design-living-scene.md` | Bands die by a silent coin-flip; give them a visible lifecycle |
| Living industry | `design-living-industry.md` | Money is destroyed at the mechanical step; managers, agents and publishing are absent |
| Survival layer | `design-survival-layer.md` | A 1975 band cannot survive on gigs alone — and the game charges nothing to live |
| Live ladder | `design-live-ladder.md` | Gig pay **falls 2.67×** as venues get bigger; the evidence says it should rise 7× |

---

## The blocker that was, and where it now stands

**Resolved 2026-10-31: S-UI shipped.** When this synthesis was written, the
Studio Cycle did not exist in the build — `pickers.rs` booked
`RecordingStudio::default()` at the format's default song count every time, so
the five-room ladder, the song choice, the per-song cost and the multi-week
calendar were all present in the engine and unreachable from the UI.

The record picker now opens from both Record menu rows: songs on ↑↓, studio on
←→, and a live quote recomputed from the same deterministic core the booking
charges from. The activity screen and the discography split landed with it, so
§H is closed in `HANDOFF.md`.

**S-SIM is now the open item that blocks the cycle's close.** The balance work
is only measurable against a reachable picker — a studio ladder no player could
climb could not be balanced, which is exactly why it could not be done first.

---

## What the world is missing, in one paragraph

The player stands outside the world. Scene bands are statistics that
appear and vanish without ceremony (`scene.rs:192-195`); there is no
geography, so London costs the same as Leeds; money arrives gross with
no promoter, VAT, agent, manager or publisher taking a cut
(`live.rs:303-327`, `economy.rs:548`); the mechanical royalty is
deducted from the label's margin and **destroyed with no recipient**
(`constants.rs:894`, `economy.rs:431` — and `grep -i publish` finds only
the comment naming who should have received it); and a band member can
live for nothing and rehearse for free (`LIFESTYLE_UPKEEP_PER_WEEK[0] =
0`, `studio.rs:345-363`), so the early game has no economic pressure at
all.

That is the shape of the problem: not one missing feature, but a world
with no other people in it and no cost to standing still.

---

## The four changes that would do the most

### 1. Give a gig a gate, and split it

Today a gig pays `base_payment × attendance × market × era` gross
(`live.rs:303-327`), and the base payment *falls* per head as venues
grow: $6.00/head at a 50-cap room down to $2.25/head at 20,000
(`venues.rs:34`). The evidence is the opposite — ticket price rises 7×
from a 50p pub gig to £3.50 at Wembley (F01) — and the band sees roughly
42% of the gate once the promoter (~⅓), VAT (8%), agent (10–20%) and
manager (20%) have taken their cuts (F02, F44, F41; arithmetic in
`design-gap-analysis.md`).

**Proposal:** put `ticket_pence` and a `DealKind` on `Venue`; compute a
gate; split it in the documented order. This single change flips the
per-head slope from −2.67× to +1.67× and makes the whole venue ladder
mean something (`design-live-ladder.md`).

**Warning:** do this *only together with* the survival layer (below).
Cutting entry-rung pay from $300 to a realistic ~$41 while the player
has no dole, no day job and a $0 upkeep floor would make the opening
unplayable.

### 2. Charge the player to exist

The game currently charges nothing to live and nothing to rehearse,
which is why the early game has no pressure. The evidence now prices it:
rehearsal £4/hr in London against 75p/hr in the provinces (F59); the
dole at £9.80 → £11.10 → £12.90/week for a single man (F61),
contribution-gated so school-leavers get nothing (F62), and forfeit if
you are not *available for work* (F63); gear at £36.57–£45 for a copy
guitar (F56) with the counter-intuitive 1975 ladder where new-budget
undercut used-professional (F58).

**Proposal:** a subsistence floor, a dole gated on contribution history,
a day job that buys that history, and gear as a real purchase. The
survival note's worked weeks show a 1975 band surviving *only* by
combining two or three local gigs with the dole, at zero margin — which
is exactly the documented scene condition (F72).

### 3. Put people between the player and the money

There is no manager and no agent. `FUTURE.md` §9.4 holds the deferred
design and `band.rs:33-37` already reserves `deal_cooldown` for it. The
evidence is now strong enough to build it: 20% of **all earnings,
gross**, before expenses and before recoupment (F41); agent ~10%
established, 15–20% for a first agency (F44); and management and agency
as **two separate contracts** even when one company holds both (F42).

**Proposal:** deduct manager first on gross, then agent on the fee, then
label recoupment, then the artist — and sell the softer order (recoup
first) as a negotiated concession worth exactly 20% of the advance
(`design-living-industry.md`).

### 4. Give bands a life you can watch

Bands die silently at `fame < 8` on a 5% roll, and only a `peak_fame >=
40` split is ever announced (`scene.rs:192-195`). There is no formation
event, no geography, no reason to care about any name on the chart.

**Proposal:** a six-phase lifecycle (Forming → Hunting → Climbing →
Peaking → Slipping → Split) with a news line on every transition, three
fields (`home_town`, `local_draw`, `phase`), six towns, and shared bills
so the player actually meets the bands they are competing with
(`design-living-scene.md`).

---

## What the evidence says is *right*, so nobody "fixes" it

- `PRESSING_SETUP_SINGLE = 75` sits inside the documented 1978 bundle of
  mastering £40 + plating £27 + labels £8 (F12).
- A DIY single at $185 ≈ £112 against Desperate Bicycles' £153 all-in
  (F11) — same order of magnitude, and the gap is sleeves, which the
  game does not model.
- `STUDIO_RATE_PER_SONG[0] = 60`, `LABEL_INCOME_PER_COPY = 3`, and the
  venue *capacity* ladder (50/200/500/2000/20000) are all defensible.

## What cannot be reconciled, and should be documented instead

1. **One currency, one decade, no inflation index.** The code is `$`
   throughout (`data/mod.rs:46-52`); the evidence is a 1970–79 `£`
   series. Pick one and record the choice; do not silently treat `$300`
   as `£300`.
2. **The ×3 L-SCALE rescale.** v0.8 tripled the economy on purpose and
   validated it in the sim lab. Ratios survive it; levels must be
   re-validated by the same sweep.
3. **Certification is a progression reward, not a trade body.** F38's
   value-based scheme is awkward to simulate; the code's unit thresholds
   are more legible. Keep them, but *state the simplification* next to
   `CERT_*`, because `economy.rs:565` currently reads as a historical
   claim and F38 contradicts it.
4. **F69 is out of period.** £25–45/band/night is c.1980–84 fieldwork.
   It is the only documented pub fee, so it is used as a *ceiling*; the
   1970s entry rung remains unresearched and is still the single most
   valuable open question.

---

## Suggested order of work

| # | Work | Why now | Size |
|---|---|---|---|
| ✅ | ~~**S-UI** record picker~~ | **done 2026-10-31** — see `HANDOFF.md` | — |
| 1 | **S-SIM** re-validation + close 0.9 | balance unproven, red assertion open | M |
| 2 | **Currency/unit decision** | blocks every number below | S (decision) |
| 3 | **Gig gate + deduction chain** | the largest single realism win | L |
| 4 | **Survival layer** | supplies the pressure the gate change removes | L |
| 5 | **Manager + agent** | best-evidenced deferred feature | M |
| 6 | **Scene lifecycle** | cheapest way to make the world inhabited | M |
| 7 | **Publishing** | money currently destroyed; new second stream | L |
| 8 | **Retail price layer** | unlocks value-based certification | M |
| 9 | **M&A distress floor, certification retune** | small, self-contained | S |

The done row is marked ✅ rather than numbered, so the list has no gap.

Items 3 and 4 must ship together — 3 removes income, 4 supplies the
floor.

---

## Honest limits of this synthesis

- **The evidence is UK 1970–79; the game runs 1970–1990.** Everything
  here is strongest for the first decade and says nothing about the
  1980s.
- **The base currency question is unresolved.** Every `$`/`£` conversion
  in these notes uses an assumed 1.65; ratios and percentages do not
  depend on it, absolute levels do.
- **Four of the six notes contain clearly labelled invention** (towns,
  phases, failure rates, basket lines). They are marked in place; none
  of them should be quoted later as if they were findings.
- **The single most valuable missing number remains the 1970s pub fee.**
  Two research rounds failed to find it. Until it exists, the entry rung
  of the whole economy is a deflated estimate rather than a fact.
