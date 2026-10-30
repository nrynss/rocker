# A living industry: design proposal

Scope: how a retail sale becomes money for a label, a distributor, a manager, an agent, a
publisher and the artist — grounded in `docs/research/uk-1970-1979/findings.csv` (F-ids) and
in the current code (`file:line`). Everything not traceable to an F-id is labelled
**design** (invented rule) or **inference** (my arithmetic on documented numbers).

---

## What the money model does now

**There is no retail price anywhere in the simulation.** `grep -rn "retail\|Retail" src/ --include=*.rs`
returns zero matches. The chain starts at a per-copy constant, so the single best-documented
number in the evidence base (F51's £5.00 → £2.61) has nowhere to enter the game.

The current per-copy pipeline, in order:

1. **Royalty base is a flat dollar figure, not a price.** `calculate_release_outcome` computes
   `((units_sold * LABEL_INCOME_PER_COPY) as f32 * deal.royalty_rate) as u32`
   (`src/game/economy.rs:548`), with `LABEL_INCOME_PER_COPY: u32 = 3` (`src/game/constants.rs:58`).
   A 12% deal therefore pays **$0.36/copy** — a rate applied to a number that is not a wholesale
   price, not a "price to dealer", and not derived from anything the player can see.
2. **Recoupment nets the royalty before payment.** `apply_recoupment` takes
   `applied = royalty.min(deal.unrecouped)` and returns only the remainder
   (`src/game/economy.rs:263-274`); the call site pays `to_player` and leaves
   `total_income_generated` at the gross (`src/game/economy.rs:817-823`). This half is already right.
3. **The label's margin re-derives the chain from constants.** `label_wholesale_margin` is
   `wholesale − manufacturing − mechanical` per copy, minus the royalty that actually reached the
   artist: `LABEL_WHOLESALE_PER_COPY 4.0`, `LABEL_MANUFACTURING_PER_COPY 1.0`,
   `LABEL_MECHANICAL_PER_COPY 0.30` (`src/game/constants.rs:892-894`,
   `src/game/economy.rs:429-433`). The comment there calls this "era-real 1970s LP economics:
   wholesale ~$4, manufacturing ~$1, mechanical ~$0.30" (`src/game/constants.rs:883`).
4. **The mechanical is a cost with no recipient.** `LABEL_MECHANICAL_PER_COPY` is subtracted from
   label margin and then vanishes — no publisher, no songwriter, no income line. `grep -rin
   "publish\|mechanical" src/` finds only that constant, its use in the margin, and one test that
   re-derives the same spread (`src/game/tests/labels.rs:22-24`). **There is no publishing model.**
   Money is destroyed at that step, which contradicts the "conserved money" claim in the label
   ledger (`src/game/world/labels.rs:230-236`).
5. **No manager, no agent.** `grep -rin "manager" src/` matches only `EventManager` and
   `events::EventManager`; `grep -rin "manager\|agent_commission\|booking_agent" src/` returns 9
   hits, all of them `EventManager` plus one doc comment. FUTURE §9.4 holds the design
   (`FUTURE.md:632-660`) and `Band::deal_cooldown` is explicitly reserved for it
   (`src/game/band.rs:33-37`). The only forward-compatible field is that cooldown.
6. **The negotiation surface is thin.** `PotentialDealOffer` carries `advance`, `royalty_rate`,
   `albums_required`, `term_weeks`, `carry_forward_unrecouped` (`src/game/world/deals.rs:48-71`).
   The option ladder exists but is **automatic, not negotiated**: `DEAL_EXTENSION_ALBUMS 1`,
   `DEAL_EXTENSION_TERM_WEEKS 52`, `DEAL_EXTENSION_ADVANCE_FRACTION 0.15`
   (`src/game/constants.rs:763-765`), and a renewal bumps the rate inside
   `DEAL_NEW_CONTRACT_ROYALTY_BUMP_MIN..=MAX` (`src/game/world/deals.rs:420-423`). Royalty is
   capped at `ROYALTY_HARD_CAP 0.35` with a reach band of `0.10..=0.32`
   (`src/game/constants.rs:1081-1093`).
7. **The unsigned path skips the industry entirely.** `INDIE_INCOME_PER_COPY: u32 = 2`
   (`src/game/constants.rs:57`) pays a flat $2/copy with no manufacturing, no mechanical, no
   recoupment (`src/game/economy.rs:550-551`) — the artist *is* the label, but the model does not
   say so.

**The net defect:** the game models the label as a bank and the artist as a rate, with the
retail price, the distributor, the publisher, the manager and the agent all missing. F53 says the
dealer price's internal split is undocumented — but that is a reason to make it a *tunable*, not a
reason to omit the tier.

---

## Proposed: the retail chain, end to end

### The one decision that fixes the whole model

**Make retail price the root variable and derive every downstream figure from it.** A release
carries a `list_price` (in pence, integer, so no float drift); the dealer price and every share is
a fraction of it. Documented fractions come first; the undocumented ones are named constants the
designer tunes, exactly as F53 instructs ("hold the dealer price as a hard number and treat the
label/distributor split as a tunable").

### The documented skeleton (1979 top-price LP)

| Step | Rule | Value on £5.00 | Evidence |
|---|---|---|---|
| Retail (incl. 15% VAT) | player-facing sticker | £5.00 | F51 |
| VAT | 15/115 of sticker | £0.6522 (13.04%) | F51 arithmetic, F54 |
| Retail (ex-VAT) | sticker / 1.15 | £4.3478 | F51 arithmetic |
| Dealer price | 52.20% of sticker, ex-VAT | £2.61 | F51 |
| Retailer margin | 40.0% of VAT-inclusive RRP | £1.7378 (39.97% of net) | F51 |
| Standard retailer margin | 36.67% of RRP, sale-or-return | dealer £2.7536 | F52 |

**Check (as F51 states it):** £2.61 × 1.15 = £3.0015 ≈ £3.00; (£5.00 − £3.00)/£5.00 = 40.0%. ✓
The three sticker shares sum to exactly 100.00%: VAT 13.04 + retailer 34.76 + dealer-tier 52.20.

Island's 40% is a **weapon against the 36.67% norm** (F51, F52). The design consequence is a
cost with a date on it: paying 40% costs the label £0.1436 more per copy than the standard term,
i.e. **5.22% of its gross receipts** (2.7536 → 2.6100). That is the price of winning shelf space,
and it is small enough to be a real temptation and large enough to matter across a pressing run.

### The dealer price, split (design — F53's gap)

Inside £2.61 the sources stop. Proposed tunables, chosen so the block sums exactly:

| Line | Rule | £ per copy | Share of £2.61 |
|---|---|---|---|
| Distributor | 12% of dealer price | 0.3132 | 12.00% |
| Manufacturing | fixed per unit | 0.8500 | 32.57% |
| Mechanical (publisher pool) | 6.25% of **retail** | 0.3125 | 11.97% |
| Artist royalty | rate × **dealer price** | 0.1566 | 6.00% |
| Label retains | residual | 0.9777 | 37.46% |
| **Total** | | **2.6100** | **100.00%** |

Two rules are load-bearing here and both are documented:

- **Mechanicals are 6.25% of retail, not of dealer price** (F48). That is why the mechanical is
  11.97% of the dealer price — the publisher's pool is computed on the sticker the fan pays. Using
  the retail denominator is the whole point of F48's worked fractions of a penny.
- **The royalty convention must be stated, not implied.** 6% of dealer price = £0.1566 = 3.13% of
  retail. Royalties in this era were argued over exactly this: "percentage of retail" versus
  "percentage of price to dealer". F16's 6% for The Jam is only meaningful with its denominator.
  **Design decision: royalties are a percentage of dealer price (PPD)** — the artist-hostile,
  label-friendly convention, which gives the negotiation table something real to fight about. A
  "percentage of retail" clause is then a **concession worth 52.2% more money** to the artist for
  the same headline number. That single chooser is the most era-true lever in the game.

### The calibration test that proves the chain

F49 says a writer's share on one hit album side was ~£900–£1,500. Run the chain backwards on a
10-track LP: mechanical pool = £0.3125/unit, split 50/50 → writer share **£0.015625 per work per
copy**. To reach £900 the album must sell 57,600 copies; £1,500 needs 96,000.

**The documented figures reproduce each other.** F51 + F48 + F47 generate F49 at a plausible
mid-success pressing run, with no fudge factor. That is the strongest validation available that
these constants belong together, and it should become a unit test (`writer_share_lands_in_f49_band`).

### Retail prices are a 1970s curve, not a constant

`list_price` should be a per-release choice constrained by a year curve: LP average £2.85 in Nov
1975 (F54), top-price £5.00 by 1979 (F51) — a 75% rise. Singles hit a psychological ceiling: £1.01
cut to 99p in Aug 1979 "to stay under the £1 mark" (F54). **Design:** a single priced at £1.00 or
above loses reach with a `below_one_pound` bonus — the 99p ceiling is a ready-made pricing decision
(F54's own proposed use).

### Where it plugs into the code

Replace the constant in `src/game/economy.rs:548` and `:969` with a per-release
`Release::list_price_pence` and a computed `dealer_price`. `LABEL_INCOME_PER_COPY` becomes the
legacy fallback for saves whose releases have no price (`#[serde(default)]` → 0 → fall back), which
keeps old saves earning exactly what they earn today. The label margin at
`src/game/economy.rs:429-433` then loses its `0.30` mechanical (which now flows to a publisher) and
gains the distributor line — one function, one place.

---

## Proposed: managers and agents

### Who takes what

| Role | Base | Rate | Evidence |
|---|---|---|---|
| Manager | **all earnings, gross** | 20% (band with a hit); 10–25% band | F41; `FUTURE.md:641` |
| Agent | **gig fee only** | 10% established; 15–20% first agency | F44 |
| Publisher | song revenue | 50% (then a second 50/50 on US income) | F19, F47 |

F41 is unusually precise and unusually brutal: 20% of **ALL** earnings, "gross, covering records,
live and other income", with production thrown in free. The caveat matters for design: the manager
in that case also owned the publisher and the label, so the 20% sat *on top of* other cuts (F43).
That is the real hazard — **ownership, not percentage** — and it is the negotiation red line the
game should expose. F42 independently requires manager and agent to be **two contracts**, even
when one company holds both; a single "representation" slider is historically wrong.

### The deduction order (the core answer)

**Recommended order — evidence-default (F41):**

```
gross contractual earning
  → manager 20% of gross          (before expenses, before recoupment, before agent)
  → agent 10–20% of the gig fee   (live only; not on royalties)
  → label recoupment              (records only; apply_recoupment, economy.rs:263)
  → artist receives the remainder
```

The manager's cut is taken **first and on gross**, so it does not shrink when the artist is
unrecouped; the agent's cut is taken on the fee, so the manager's 20% also lands on the agent's
commission. That is the documented trap and it must be the default, or the game teaches a softer
industry than existed.

**The alternative order, sold as a concession:** `gross → recoupment → manager 20% of received`.
It is worth exactly **20% of the advance**, which is the clean number that makes the lever legible.

### Worked example A — a royalty (100,000 copies, 6% PPD, £6,000 advance)

Per-copy royalty £0.1566 → gross contractual royalty **£15,660**.

| Order | Manager | Recoupment | Artist receives |
|---|---|---|---|
| Gross-first (default) | 0.20 × 15,660 = **£3,132** | 6,000 | 15,660 − 6,000 − 3,132 = **£6,528** |
| Recoup-first (concession) | 0.20 × 9,660 = **£1,932** | 6,000 | 9,660 − 1,932 = **£7,728** |

Difference: **£1,200 = 20% of the £6,000 advance** exactly. ✓ The manager is paid while the artist
is still recouping — the defining cash-flow misery of the era, and now a number the player can see.

Recoupment length also moves with the order: £6,000 / £0.1566 = **38,314 copies** at 6% PPD; at
F16's post-option 13%, **17,683 copies**. A 13% deal recoups in under half the copies — which is
the real reason the option ladder is worth fighting for.

### Worked example B — a £500 gig fee

| Order | Agent | Manager | Artist |
|---|---|---|---|
| Both on gross (default) | 15% = £75 | 20% = £100 | **£325** |
| Manager net of agent | 15% = £75 | 20% × 425 = £85 | **£340** |

Difference **£15 = 20% of the agent's £75**. Commission-on-commission is invisible until you write
the two lines next to each other; that is the teaching moment, and it costs 3% of gross
(0.20 × 0.15) whenever both contracts exist.

### Mechanics

- Manager offers reuse the deal-offer stream (`FUTURE.md:645-646`), drawing from the existing
  seeded RNG; `cut`, `skill`, and the hidden `honesty` skim are FUTURE §9.4's design and need no
  reinvention (`FUTURE.md:637-654`).
- The manager's cut must be **one function** applied at every `player.earn_money` site, not four
  scattered edits: `src/game/actions/business.rs:253` (advance), `:319`,
  `src/game/actions/live.rs:327` (gig), `:713` (tour week). Route all four through
  `Player::earn_money` (`src/game/player.rs:157-159`) gaining a `gross`/`net` distinction, or a
  manager cut will silently miss the advance — which is where the biggest single cheque is.
- **Design:** while unrecouped, a gross-first manager cut becomes **arrears** (a debt), not cash
  from nothing; settled from the first post-recoupment income. Without this the player can go
  negative with no explanation.
- Firing settlement ≈ 8 weeks of average take and a 12-week offer cooldown, reusing
  `Band::deal_cooldown` (`src/game/band.rs:36`) — the field is already reserved for exactly this.

---

## Proposed: publishing

### What exists today: nothing

There is no publishing model (finding 4 above). `LABEL_MECHANICAL_PER_COPY` is a floating cost.
The proposal is to give that £0.30 (→ £0.3125 on a £5.00 LP) a recipient.

### The pool and its two splits

```
per unit:  mechanical = 6.25% × list_price            (F48 — retail denominator)
per work:  mechanical ÷ track_count                    (design — why track count was fought over)
writer:    50% of the work's mechanical                (F19)
on US income: a further 50/50 → writer keeps 25%       (F47)
```

F48's per-work figures (1.73p, 1.85p, 0.943p, 0.925p) are exactly this division: £0.3125 ÷ 1.73p
≈ 18 works per copy. **The per-work rate is a function of track count, not a fixed penny figure** —
so a 14-track album pays each writer 22% less than a 10-track one for the same sales. That is the
mechanical reason co-writing credit and track placement were litigated, and it is a free mechanic:
the player's track-count choice now has a publishing consequence.

### Worked example C — 50,000 copies, 10 tracks, £5.00 LP

Pool = 50,000 × £0.3125 = **£15,625**. Per work = £1,562.50; writer's 50% = **£781.25** per work.
If 40% of sales are US: US pool £6,250 → second split → writer gets **£1,562.50** on that slice
instead of £3,125 — a **£1,562.50** loss to the same songs. The player sees "50:50" on the
contract and receives 25% abroad (F47: "a checkable era-correct trap").

### The advance, and the onerous-deal trap

F20 (`Schroeder v Macaulay`, 1974) voided a 5-year, £50-advance, life-of-copyright assignment with
auto-extension as an unreasonable restraint of trade; F19 says full-term assignment with **no
reversion** was the norm. F49's scale check — £900–£1,500 per album side — is what makes a £50
advance genuinely exploitative rather than merely stingy: it is **3–6% of one album side's writer
income** (£50 / £1,500 = 3.3%; £50 / £900 = 5.6%).

**Design:** a publishing contract is `{advance, term_years, split, us_split, reversion}`.
Recoupment runs against the writer's share only (not the publisher's). Full-term assignment with
`reversion: None` is the cheap default; it should be **challengeable in court** on an F20-style
event card when the terms cross a hardship threshold (long term + tiny advance + no reversion),
with the F20 outcome available — the contract is voided, the advance is not repaid, and the
publisher relationship ends badly. That turns F19/F20 from trivia into a risk the player can take
or refuse.

### Where it plugs in

Publisher becomes a world entity like `Label` (`src/game/world/labels.rs:230`), seeded
deterministically from `data/record_labels.json` or a sibling file; `LabelStatus` already has the
lifecycle states a publisher needs (`src/game/world/labels.rs:198-207`). The label's
`LABEL_MECHANICAL_PER_COPY` deduction at `src/game/economy.rs:431` becomes a transfer to the
publisher's balance sheet, closing the destroyed-money hole.

---

## Proposed: labels behaving differently by year

The existing personality layer is already the right hook: `LabelStrategy` has `Balanced`,
`ArtistFriendly`, `HitFactory`, `GenrePurist`, derived from `financial_power` /
`creative_freedom` / `artist_development` (`src/game/world/labels.rs:86-120`), and
`LabelStatus` carries `Active → Distressed → Folding/Acquired` (`src/game/world/labels.rs:198-207`).
Year behaviour should be a **modifier layer over strategy**, not a replacement: each year supplies
multipliers, so a `HitFactory` in 1977 is a different animal from a `HitFactory` in 1971.

| Year | Behaviour | Mechanism | Evidence |
|---|---|---|---|
| 1970–73 | Conservative, long-term artist development; publishing is the quiet profit centre | `ArtistFriendly` weight up; publishing advance offers frequent but small | F19, F43 |
| 1974 | Legal shock: onerous contracts become voidable | enable the F20 court-challenge card | F20 |
| 1975–76 | Price correction; dealer-margin squeeze begins | LP index £2.85 (Nov 75); margin-squeeze event; `HitFactory` weight up | F54, F55 |
| 1976 | **Punk signing spree** — offers detach from sales history | advance multiplier for hype-driven acts ~16× spread; `drop_weeks` short | F22 |
| 1976–78 | **Indie infrastructure arrives** | unlock `DistributionChannel` tier: DIY → indie national reach | F30 |
| 1977 | Mechanicals and track-placement fights; option ladders standard | F48/F49 publishing pressure; `DEAL_EXTENSION_*` live | F16, F48, F49 |
| 1978 | Cartel-era indie distribution matures | indie releases gain national shop reach; indie chart eligibility | F30 |
| 1979 | **Discounting war and the 99p ceiling**; five named indie distributors | singles price floor event; distributor choice becomes real | F53, F54 |

Two of these deserve to be *fast* and *cruel*, because the evidence is:

- **A&R scouting and fast dropping (F22).** The Sex Pistols were signed for ~£40,000 in Oct 1976
  and dropped after ~3 months; A&M paid £75,000+ in Mar 1977 and dropped them in six days, and the
  band kept the money. The Clash got £100,000 in Jan 1977 — a ~16× spread over The Jam's £6,000 in
  the same year. **Design:** a `hype` scalar (fame × press heat ÷ sales history) that multiplies
  offers far beyond sales and sets `drop_weeks` to 13–26. Severance stays with the artist. This is
  the one place where a player can rationally prefer being dropped.
- **The margin squeeze (F54, F55).** Nearly 200 dealers protested a margin squeeze; Polydor cut
  singles £1.01 → 99p; stores selling only best-sellers at reduced prices already held ~30% of the
  album market. **Design:** a market-wide pressure track that raises the retailer margin demanded
  (36.67% → 40%) and shrinks the label's residual — the same £5.00 LP earns the label £0.1436 less
  per copy when it must pay Island's 40%.

---

## Proposed: the negotiation surface

Each lever must have a price the player can compute *after* being burned once. Six levers, each
mapped to a real cost:

| Lever | Range | What it costs the other side | What the player learns |
|---|---|---|---|
| **Advance** | £5k–£100k by tier/hype (F16, F22) | straight cash off the label's `capital`; adds to `unrecouped` | cash now is debt later — it must be recouped at £0.1566/copy |
| **Royalty rate** | 6% → 13% via option (F16); cap 0.35 (`constants.rs:1093`) | reduces label residual pound-for-pound | 1pp ≈ £0.0261/copy at £2.61 PPD |
| **Royalty base** | PPD (default) vs retail | retail basis is 52.2% more expensive to the label | the headline % is meaningless without its denominator |
| **Option ladder** | albums + term + bump (`constants.rs:763-765`) | label locks the act; artist trades freedom for the rate | the 13% at year 4 is bought with years 1–3 |
| **Publishing share** | 50/50, US 25% (F19, F47) | publisher's income; writer's reversion | "50:50" is not 50% abroad |
| **Manager %** | 10–25%, gross (F41; `FUTURE.md:641`) | 20% of *everything*, before expenses | gross vs net-of-recoup is worth 20% of the advance |
| **Term** | 52–156 weeks by tier (`constants.rs:725-727`) | longer term = more unrecouped risk carried | a bad deal outlives the hit |

**Presentation rule:** every lever shows its **per-copy and per-advance cash equivalent** at the
moment of choice ("+1% royalty ≈ +£261 per 10,000 copies; the label's share falls to £0.9516").
The player should never have to guess a percentage's worth — the historical trap is that artists
could not do this arithmetic, and the game's job is to let them learn it, not to hide it.

**The refusal lever matters too.** F22's severance rule means walking away from a hype offer is
sometimes correct; and F43's warning (a manager who also owns the publisher) is the red line the
player should be able to *see* in an offer's terms.

---

## Determinism, serde and the sim harness

**Determinism.** Every new roll takes `&mut impl Rng` from the existing seeded stream — no
`thread_rng`, no `SystemTime`. Publishing splits, manager offers, hype and drop timers all draw
from the deal-offer stream or a named sibling, so `world_seed` alone reproduces a career. The
existing harness already asserts this (`seeded_worlds_are_reproducible_in_the_harness`,
`src/game/sim.rs:990-993`).

**Serde.** Every new field is `#[serde(default)]`; no existing field is renamed. `Release` gains
`list_price_pence: u32` (0 = legacy → fall back to `LABEL_INCOME_PER_COPY`, so an old save earns
exactly what it earns today). `RecordDeal` gains `royalty_basis: RoyaltyBasis` with `#[default]`
= `PriceToDealer`, `manager_cut: f32`, `publisher_id: Option<u32>`. New enums get a `#[default]`
variant so `#[serde(default)]` compiles. `PotentialDealOffer` gains the same levers as `Option`
fields so a pending pre-change offer still deserializes. Extend `src/game/tests/save_compat.rs`.

**Sim harness.** `LABEL_INCOME_PER_COPY` is the *only* per-copy income constant and it is currently
worth $3 — already at era scale after L-SCALE retired `SALES_INCOME_DIVISOR` to 1
(`src/game/constants.rs:40-56`). The proposed chain pays **£2.61/copy to the dealer tier** and
£0.1566 to the artist at 6% — i.e. an artist's per-copy income drops **2.3×** (£0.36 → £0.1566)
versus today at 12%. That is a real balance break, not a rounding change, and it must land with a
system-wide retune (advance sizes, pressing costs, gig income) refereed by `run_career` /
`print_report` medians (`src/game/sim.rs:683-830`). Two guardrails:

- Keep the **label's** total take per copy roughly constant (£2.61 − distributor ≈ £2.30 vs today's
  £4.00 − 1.00 − 0.30 = £2.70); the squeeze is felt as *artist* income falling, which is the
  historically correct direction, but it must not make `label_loyalist` bankrupt.
- Add a `publishing_income` bot so the balance lab measures the second stream, and a
  `hype_signing` bot for F22 (accept the biggest advance, get dropped, repeat) — it must be
  survivable but not dominant, or F22 becomes an exploit.

---

## Grounding

**Documented (F-ids).** F16 £6,000 + 6%, 13% after option · F19 full-term assignment, 50:50, no
reversion · F20 *Schroeder v Macaulay* voiding an onerous deal · F22 punk spree, ~16× spread, fast
drops, severance kept · F30 indie distribution 1976–78, £400 loan · F41 manager 20% of all earnings
gross · F42 management and agency as two contracts · F43 manager-owned publisher/label · F44 agent
10% established / 15–20% first agency · F47 double 50/50, writer 25% of US income · F48 mechanicals
6.25% of retail · F49 £900–£1,500 per hit album side · F51 £5.00 → £2.61, 40% dealer margin ·
F52 standard 36.67% · F53 label/distributor split undocumented · F54 £2.85 LP, £1.01 → 99p single,
margin protest · F55 ~30% of album market discount-driven.

**Code.** `economy.rs:548` flat royalty base · `economy.rs:263-274` recoupment · `economy.rs:429-433`
label margin · `economy.rs:817-823` pay-out order · `constants.rs:57-58, 892-894` per-copy
constants · `constants.rs:725-727, 763-765, 1081-1093` deal terms and caps ·
`world/deals.rs:48-71` offer shape · `band.rs:33-37` reserved cooldown · `band.rs:60-97` RecordDeal ·
`world/labels.rs:59-120, 198-236` tier/strategy/status/Label · `sim.rs:683-830, 990-993` harness ·
`player.rs:157-159` single money entry point · `FUTURE.md:632-660` the deferred manager.

**Invented (design), flagged as such above:** 12% distributor share; £0.85 manufacturing per unit;
royalties on PPD as the default basis; track-count division of the mechanical pool (inferred from
F48's per-work pennies); the hardship threshold that opens a court challenge to a publishing
contract; the
`hype` scalar and 13–26 week drop window; manager arrears while unrecouped; 8-week firing
settlement; the year-by-year multiplier table; per-copy cash-equivalent display in negotiation.

**Not verified:** whether the 15–20% first-agency rate attaches to the agency sentence in F44's
source (flagged "indicative" in the CSV); whether 6.25% mechanicals were computed pre- or post-VAT
in 1977 (this document assumes the retail price as printed, incl. VAT, matching F48's "6.25% of
retail"). Both are worth one source check before the numbers are hard-coded.
