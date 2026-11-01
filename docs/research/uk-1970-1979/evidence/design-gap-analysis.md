# Design gap analysis: evidence vs code

Scope: `findings.csv` (F01–F72, UK 1970–1979) against the shipped simulation in
the repo root. Every claim below cites `file:line` or an `F-id`.

## Method and units (read this before the table)

**The code is dollar-denominated.** `format_money` prints `$` unconditionally
(`src/data/mod.rs:46-52`), and every cost/income constant is a bare integer with no
currency tag. The entire evidence base is **£ sterling**. No exchange rate, no
`£`, no `GBP`, and no `currency` field exists anywhere in `src/` (exhaustive grep:
zero hits). This single fact sits underneath every comparison in this document and is
mismatch **#1**.

**Conversion used below:** £1 = $1.65 (1977 average). This is an *assumption*, not a
finding — `findings.csv` carries no FX row. All conversions are marked *(arith)* and
should be re-run if the project picks a different rate or declares an abstract
"game dollar". Percentages, ratios and per-head figures are FX-independent and are
the load-bearing part of each finding.

**Scale caveat that will bite any fix:** v0.8 deliberately multiplied the whole
economy by 3 ("one honest money scale", L-SCALE) — see the rationale at
`src/game/constants.rs:44-56` (the divisor), `src/game/constants.rs:99`
("pressing setup ×3"), `src/game/constants.rs:481-483` (tour rig ×3) and
`src/game/world/venues.rs:29-33` (gig base pay ×3). Constant *levels*
are therefore 3× their pre-v0.8 values while *ratios* were preserved. A fix that
moves a level must re-run that balance sweep; a fix that moves a ratio does not.

---

## Verdict summary

| Verdict | Count |
|---|---|
| **WRONG** — evidence contradicts the code | **14** |
| **UNSUPPORTED** — code invents it, or the layer is absent entirely | **14** |
| **FINE** — evidence supports the current value | **8** |
| **Total examined** | **37** |

The 14 UNSUPPORTED split 1 in the mismatch table (row 14, dead JSON fields) and
13 in the invented-numbers section below (manager, agent, dole, day jobs, lifestyle
floor, tour rigs, retail price, dealer margin, gear, indie chart, distribution
dating, VAT, market volume).

Highest-severity clusters, in order: (a) the currency/unit question, which blocks
everything else; (b) gig economics — pay shape inverted, no promoter/VAT/agent
deduction, entry rung 4–7× too rich; (c) certification thresholds, which are
wrong on three independent axes at once; (d) the absent survival layer (dole, day
jobs, rehearsal rent, gear).

---

## Mismatches that matter

| # | Severity | Verdict | Where (file:line + constant) | Code says | Evidence says (F-id) | Recommended direction | Risk |
|---|---|---|---|---|---|---|---|
| 1 | **Critical** | WRONG | `src/data/mod.rs:46-52` `format_money`; whole economy | Money is `$` | Entire base is `£` (F01, F11, F16, F37, F51, F61, F69) | Decide the unit **before** renumbering: either re-denominate to `£`, or declare an abstract unit and add one documented FX constant used only when comparing to history. Do not silently treat `$300` as `£300`. | Blocks every other row. Touches every constant, all save files, and all UI strings. |
| 2 | **High** | WRONG | `src/game/world/deals.rs:569-575` `priced_advance` → `src/game/constants.rs:1060` `ADVANCE_STATURE_PER_FP_CUBED = 1.1`, `:1066` `ADVANCE_FAME_FLOOR = 0.15` | An unknown act's *floor* advance from Atlantic (fp 90) is `90³ × 1.1 × 0.15` = **$120,285** *(arith)*. Stature ceiling `90³ × 1.1` = $801,900. | F16: The Jam signed to Polydor in Feb 1977 for **£6,000** ≈ $9,900 *(arith @1.65)*. Code floor is **12.2×** that *(arith: 120285/9900)*. | Move the beginner anchor toward **£6,000–£10,000**. The cleanest lever is `ADVANCE_FAME_FLOOR` (0.15 → ~0.012) plus a hard `ADVANCE_MINOR_TIER_CAP`; do **not** cut `ADVANCE_STATURE_PER_FP_CUBED`, which correctly sizes the *star* tier (F22: £40k–100k ≈ $66k–165k, which the current ranges already contain). | Breaks the recoup sweep and 15-yr label solvency (`constants.rs:1040-1042`). Must re-run L-SIM. |
| 3 | **High** | WRONG | `data/record_labels.json` `advance_range` on all 6 majors | Lowest major advance anywhere is **$75,000** (Elektra); Columbia reaches $2,250,000 | F16's £6,000 beginner major deal ≈ $9,900. F22's *famous-act* punk deals are £40k–100k. | Add a genuine beginner tier or rescale the major floors by ~10×. As shipped there is **no rung on the ladder between $0 and $75,000**, so the single best-documented transaction in the base (F16) is unreachable. | Same as #2. Also note these ranges are largely **dead data** — see #14. |
| 4 | **High** | WRONG | `src/game/constants.rs:1093` `ROYALTY_HARD_CAP = 0.35`; `:1081-1082` `ROYALTY_REACH_MAX = 0.32` / `MIN = 0.10`; derived at `src/game/world/deals.rs:580-583` | Rate runs 32% (reach 0) → 10% (reach 100). Computed *(arith)*: Dischord reach 15 → **28.7%**, SST reach 25 → **26.5%**, Factory reach 50 → **21.0%**. Cap 35%. | F17: period norm **5–15%** ("typical … between 5 and 15%"), UK 8–15%, new acts at the **bottom**. F16: The Jam's beginner major rate was **6%**, rising to 13% only on option. | Compress the band to roughly **0.06–0.15**: e.g. `ROYALTY_REACH_MAX → 0.15`, `ROYALTY_REACH_MIN → 0.06`, `ROYALTY_HARD_CAP → 0.15`. Keep the reach→rate *slope* (indies paying more is defensible commercial logic) but inside F17's envelope. | Low mechanical risk, high balance risk: royalty is the main label-lever and feeds recoupment (`economy.rs:548`). A 32%→15% cut halves signed income for indie-label acts. |
| 5 | Medium | WRONG | `src/game/constants.rs:1082` `ROYALTY_REACH_MIN = 0.10` | A reach-98 major (Columbia) prices at **10.4%** *(arith)* | F16: the documented beginner major rate was **6%**. F17 puts new acts at the *bottom* of 5–15%. | With #4: lower the floor to ~0.06 so a cold act on a major lands on F16's 6%. Currently the code's *cheapest possible* major deal is already 73% above the one hard data point. | Minor on its own; do it in the same pass as #4. |
| 6 | **High** | WRONG | `src/game/actions/live.rs:303-304` (`earnings`), credited gross at `:327` `player.earn_money(earnings)` | Band receives `base_payment × attendance_ratio × market_modifier × era_modifier` — **no deduction of any kind**. | F02: of each ticket about **one third to the promoter**, plus **8% VAT** and box-office commission, *before* the band's share. F44: booking agency takes **10%** (established) / **15–20%** (first agency). F41: manager takes **20% of gross**. | Add an explicit deduction chain between gate and player: promoter ~33%, VAT 8% (see #23), agent 10–20%, manager 20%. At F02+F44+F41 the band's net is roughly `gate × 0.67 × 0.92 × 0.85 × 0.80` ≈ **42% of gate** *(arith)*. The game currently pays **100%**. | Large: instantly cuts gig income ~58%. Must be sequenced with #7 and #8 or the early game becomes unplayable. |
| 7 | **High** | WRONG | `src/game/world/venues.rs:27` `capacities = [50, 200, 500, 2000, 20000]`, `:34` `payments = [300, 900, 2400, 9000, 45000]` | Per-head take at a full house *(arith)*: 50-cap **$6.00**, 200-cap **$4.50**, 500-cap **$4.80**, 2000-cap **$4.50**, 20000-cap **$2.25**. Take **falls 2.67×** from smallest to largest. | F01: ticket price **rises** with venue tier — 50p cheap gig, ~£2 average for a big band, £3.50 Wembley. That is a **7× rise** *(arith: 3.50/0.50)*, not a 2.67× fall. | Invert the slope. Anchor the payments table to F01's face values × capacity: 50-cap ≈ £25 gate ≈ **$41**, 200-cap ≈ £400 ≈ **$660**, 500-cap ≈ £1,000 ≈ $1,650, 2000-cap ≈ £4,000 ≈ $6,600, 20000-cap ≈ £70,000 ≈ $115,000 *(all arith @1.65, face value = full house)*. Then apply #6's deductions. | **Largest single change in the document.** Gig income is the early-game lifeline; dropping the entry rung from $300 to ~$41 (a 7.3× cut) with no dole or day job (#17, #19, #20) makes the opening unplayable. Land #6/#7 only together with the survival layer. |
| 8 | **High** | WRONG | `src/game/world/venues.rs:34` `payments[0] = 300` for a 50-cap room | A 50-cap pub gig pays **$300** ≈ **£182** *(arith)* | F69: a small local band earned **£25–45 per band per night** in pubs — roughly **£6–11 per musician**. Code is **4–7× the top of that range**. | Move the entry rung to **£25–45 per band** ≈ **$41–74** *(arith)*. Note F69's window is c.1980–84, **not** the 1970s — do not silently backdate; the true 1970s figure is likely **lower**, so treat £25–45 as a ceiling for 1970–79. | Compounds #7. See the sequencing note there. |
| 9 | **High** | WRONG | `src/game/economy.rs:568-581` `compute_certification_level` (takes only `copies_sold`); `:565` "era-agnostic — the copy thresholds never move"; call sites `:617`, `:844`, `:1008` | One threshold set for both formats: silver **50,000**, gold **150,000**, platinum **400,000** (`constants.rs:137-139`) | F38: **singles were unit-based throughout** at **250,000 silver / 500,000 gold / 1,000,000 platinum**. | Make certification format-aware: `ReleaseType` already exists (`src/game/music.rs:17-20`) but is not passed in. Add a `(ReleaseType, units)` signature. Singles: 250k/500k/1M. | Medium code change, large balance change: a self-pressed 50k run currently reaches "Silver" on a single (`constants.rs:86-87` says so deliberately), which F38 makes impossible. The National pressing run (50k) can no longer certify a single at all. |
| 10 | **High** | WRONG | `src/game/constants.rs:137-139` `CERT_SILVER_THRESHOLD = 50_000`, `CERT_GOLD_THRESHOLD = 150_000`, `CERT_PLATINUM_THRESHOLD = 400_000` | 50k / 150k / 400k | F38, album unit thresholds "as of **Jan 1979**": **60,000 / 100,000 / 300,000**. | Set album thresholds to **60k / 100k / 300k**. Deltas *(arith)*: silver **−17%**, gold **+50%**, platinum **+33%**. | Gold and platinum get *easier*; silver gets harder. The `constants.rs:69-75` note already flags that ~71 cert events per career is under review — this row is the evidence for that review. |
| 11 | **High** | WRONG | `src/game/economy.rs:596-605` `era_max_certification_level`; `src/game/constants.rs:153-155` `CERT_YEAR_SCHEME_START = 1973`, `CERT_YEAR_PLATINUM = 1976`, `CERT_YEAR_MULTIPLATINUM = 1984` | Awards are **unit-based from 1973**; platinum "does not exist yet" before 1976 | F38: BPI **album** awards were **£-value based from Apr 1973** — £1m platinum / £250k gold / £75k silver (1974-75 keys); silver £100k from 1 Jan 1976; gold £300k / silver £150k for releases on/after 1 Jan 1977 — switching to units only **late 1978 / Jan 1979**. Platinum therefore exists from **1973**, not 1976. | Two fixes. (a) `CERT_YEAR_PLATINUM` 1976 → **1973**; F38's 1974-75 keys already carry a £1m platinum. (b) Model 1973–78 as **value-based** (thresholds above, against retail × units) and flip to units at Jan 1979 — which is what #10's thresholds then apply to. | The value→unit switch needs a retail price, which the game does not have (#21). Sequencing: land #21 first or approximate value as `units × a per-format price constant`. |
| 12 | Medium | WRONG | `src/game/constants.rs:154` `CERT_YEAR_PLATINUM = 1976` | No platinum before 1976 | F38: £1m platinum appears in the **1974-75** keys. | Move to **1973**, matching `CERT_YEAR_SCHEME_START`. | Trivial mechanically; but it is downstream of #11, so do them together. |
| 13 | Medium | WRONG | `src/game/actions/studio.rs:345-363` `action_practice` | Rehearsal costs **nothing** — only `PRACTICE_STRESS_COST = 4` (`constants.rs:304`) and a +2 skill gain. No money leaves the player. | F59: **£4.00/hour** in London EC1 for a 10-musician room (1971); 75p/hour at a provincial studio (1973). An 8-hour week in London is **£32/week ≈ $53** *(arith)*. | Charge per rehearsal, tiered by scene: London ~£4/hr, provincial ~75p/hr. An 8-hr week → **$53 London / $10 provincial** *(arith)*. Two regional rates also give `markets.json`'s London/Manchester/Scotland split something to do. | Low, and it *adds* a money sink the early game currently lacks — which is exactly the headroom #7 needs. |
| 14 | Medium | Unsupported (dead data) | `data/record_labels.json` `royalty_rate`, `advance_range` | 13 labels carry hand-authored rates (majors 10-15, indies 25-50) and ranges | F16/F17 describe the real 5–15% band; the JSON's 40% and 50% (Touch and Go, Dischord) are far outside it | The live offer path **ignores both fields**: royalty is derived from reach (`deals.rs:588-592`) and advance from capital/fame (`deals.rs:569-575`). Only tests read them (`src/game/tests/deal_lifecycle.rs:20-21`). | **Delete or clearly mark these fields.** They are the most visible "royalty rate" in the repo and they are wrong *and* inert — a future reader will trust them. Low risk, high confusion-removal value. |
| 15 | Medium | WRONG | `data/timeline.json` (keys 1970/1972/1975/1977/1980/1983/1985/1988/1990/1993) + `src/game/timeline.rs:95-107` fallback | Missing years silently inherit the **previous** era, defaulting to 1970. **1978 and 1979 both run the 1977 "Punk Revolution" block**: `gig_pay_modifier 0.9`, `recording_cost_modifier 0.8` | F32: 1979 was the **all-time UK singles peak** (89.1m, never beaten pre-digital). F37: LP list price **£3.49 (Feb 1977) → £4.99 (Jan 1980)**, **+43%** *(arith)*. F40: VAT moves to **15% in 1979**. | Add at minimum a **1979** era (and ideally 1978). 1979 should not be a 0.9 gig-pay year: F32 and F37 both describe a market at its peak, not a punk trough. | The fallback at `timeline.rs:95-107` is silent — nothing warns that 1979 is running 1977 numbers. Cheap to fix, easy to miss. |

---

## Numbers the code invents (no evidence either way)

These are not contradicted — they are simply unanchored. Several can now be anchored.

- **No manager skim (F41).** No manager entity, no 20% gross cut. Grep for
  `manager|commission` over `src/game/` returns only `EventManager`
  (`src/game/events.rs:38`) and a forward-looking comment (`src/game/band.rs:36`).
  F41 documents a real 20%-of-gross agreement (Gilbert O'Sullivan / Gordon Mills,
  1970). **Anchor available:** 20% of gross, across *all* streams including live.
- **No booking-agent skim (F44).** Gig fees are credited gross (`live.rs:327`).
  F44 gives ~10% established, 15–20% for a young band's first agency — i.e. early
  representation costs *more*. **Anchor available**, and it is a nice difficulty
  curve: the beginner pays a higher rate.
- **No dole (F61–F63).** Nothing in `src/game/` models unemployment benefit;
  `player.money` starts at **0** (`src/game/player.rs:127`). F61: £9.80/wk (Apr
  1975) → £11.10 → £12.90/wk (Nov 1976) against **£66.97 average manual weekly
  earnings**. F62: contribution-gated (25×/50× the £15 weekly LEL) — so
  school-leavers qualified for nothing, which is a *gameplay-relevant* distinction
  the code could reproduce. F63: availability-for-work test + 3 waiting days per
  spell. **Anchor available**, and it is the natural counterweight to #7.
- **No day jobs.** No employment action exists. F61's £66.97 manual weekly wage is
  the natural ceiling for a part-time option.
- **Lifestyle upkeep floor is implausible against F61.** `LIFESTYLE_UPKEEP_PER_WEEK
  = [0, 120, 540, 2_100, 8_400]` (`src/game/constants.rs:430`). Tier 1 = **$120/wk
  ≈ £73/wk** *(arith)* — **109% of the £66.97 average manual weekly wage**
  (F61) for the *cheapest non-zero* tier. No rent series exists in the base to
  confirm or refute a specific figure, so this is UNSUPPORTED, but it is
  arithmetically hard to defend: the entry lifestyle consumes an entire manual
  wage before food. Worth a research pass; until then treat tier 1 as suspect.
- **Van/tour rig costs.** `TOUR_RIG_COST_PER_WEEK = [360, 1_200, 2_800, 6_500]`
  (`src/game/constants.rs:492`). A van week is **$360 ≈ £218** *(arith)*.
  F60 anchors only **fuel**: 79.3p/gal in 1977; a 500-mile week at 20 mpg is
  ~25 gal ≈ **£19.8/wk ≈ $33** *(arith)* — about **9%** of the van figure. So the
  fuel component can be anchored now; the other 91% (hire, crew, hotels) is
  unanchored. F03 is the closest structural analogue: 10cc took £1,000 a gig and
  spent **£750 before reaching the stage**, i.e. a **75%** pre-stage cost ratio.
- **No retail price layer (F37).** The game never models what a record costs the
  consumer, so the F37 series (singles 50p → 70p; LPs £2.42 → £4.99) has nothing to
  attach to. This is the missing rung that blocks #11's value-based certification.
- **No dealer/retailer margin (F51/F52).** F51's chain is arithmetically closed
  (£5.00 retail / £2.61 dealer ex-VAT / 40%) and F52 gives the standard term as
  **36.67%** on sale-or-return. `LABEL_RECOUP_PRESSING_PER_COPY = 0.15`
  (`constants.rs:701`) is a recoup rate, not a margin — unrelated.
- **No gear purchase (F56–F58).** Grep for `gear` in `src/` returns nothing. F56/F57
  give the real entry ladder (guitar £36.57–45; 4-drum kit £122.94; basic PA
  £71.30) and F58 the counter-intuitive finding that in 1975 a **new-budget rig
  (~£512) was cheaper than used-professional (~£688)**. That inversion is a
  ready-made progression gate: poor bands buy new copies, gigging bands buy old
  Fenders.
- **No indie-chart era gate (F31).** `peak_chart_position` is a single main-chart
  position (`src/game/music.rs:255`, set at `src/game/economy.rs:804`). There is no
  `era_max` equivalent for charts, unlike certification (`economy.rs:596`). F31:
  **no separate indie chart existed inside the 1970s**; the first official one is
  **19 Jan 1980**, so independents had to crack the main chart. An unsigned indie
  charting in 1974 is therefore anachronistic — the code has no gate to stop it.
- **Distribution is date-blind (F30).** `DistributionChannel` is
  `MailOrder | Regional | National` (`src/game/music.rs:92-96`) with
  `DISTRIBUTION_CHANNEL_FEE = [0, 1_200, 4_500]` (`constants.rs:797`), purchasable
  at any year. F30: Stiff (1976, from a **£400 loan**), Rough Trade's distribution
  arm, and the regional **Cartel** network coalesced around **1978**. "National"
  reach before 1978 should be unavailable or heavily penalised.
- **No VAT layer (F40).** Grep for `VAT|vat` over `src/`: zero hits. F40: records at
  10% (from 1 Apr 1973) → 8% (29 Jul 1974) → 15% (1979). This is the missing input
  to #6's net-gig arithmetic and to F51's ex-VAT dealer price.
- **No market-total volume model (F32).** F32's 88.8m (1978) / 89.1m (1979) singles
  have nothing to compare against; `data/markets.json` carries regions with
  `record_sales_per_capita`, not a national unit total. Not a mismatch — an absent
  aggregate. Low priority for a single-band sim.
- **`ADVANCE_STATURE_PER_FP_CUBED`, `ADVANCE_CAPITAL_FRACTION`,
  `ROYALTY_BID_PREMIUM`, `BID_ESCALATION_FRACTION`** (`constants.rs:1052-1100`) are
  pure feel-tuning with no historical referent. Correctly labelled `[tune]`. Leave
  them; the evidence base has nothing to say about bidding wars.

---

## Evidence that supports current code

Do **not** "fix" these.

- **`PRESSING_SETUP_SINGLE = 75.0`** (`src/game/constants.rs:100`) ≈ **£45**
  *(arith)*. F12 itemises the non-per-copy share of a 1978 pressing as mastering
  **£40** + plating **£27** + labels **£8** = **£75**. The code's flat setup sits
  inside that bundle. FINE.
- **DIY single, all-in, is close.** Setup 75 + 500 × $0.10 pressing
  (`constants.rs:100,112`) = $125, plus the cheapest studio tier $60/song
  (`constants.rs:620`) = **$185 ≈ £112** *(arith)*. F11: Desperate Bicycles made
  500 copies for **£153** all-in. Code is **27% under** — comfortably the same
  order of magnitude, and the gap is explained by F11 including sleeves, which the
  code does not model. FINE.
- **`STUDIO_RATE_PER_SONG[0] = 60`** (`constants.rs:620`). F12: £98 for 14 hours
  including master tape ≈ **£7/hr ≈ $11.5/hr** *(arith)*. A 3-hour song at the
  code's cheapest tier is ~$20/hr — same order. FINE.
- **`LABEL_INCOME_PER_COPY = 3`** (`constants.rs:58`). F51's dealer price is
  **£2.61 ex-VAT ≈ $4.31** *(arith)*. $3 is 30% below a *top-price* 1979 LP dealer
  price and is a wholesale-side number being compared to an artist-side constant —
  close enough that changing it would be false precision. FINE.
- **Venue capacity ladder** `[50, 200, 500, 2000, 20000]` (`venues.rs:27`). F05
  confirms the pub circuit (Hope and Anchor, Nashville, Tally Ho) as the entry rung
  through the punk era; F01's £3.50 "Wembley supergig" bounds the top. The
  *capacities* are defensible — it is the **payments** attached to them (#7) that
  are wrong. FINE.
- **`PRESSING_PER_COPY_SINGLE = 0.10` / `ALBUM = 0.50`**
  (`constants.rs:112-113`). The *album* figure has no contradicting evidence and is
  the right shape (5× the single). Only the *single* rate is contradicted (#9's
  neighbour below). Marking the album rate FINE.
- **`CERT_YEAR_SCHEME_START = 1973`** (`constants.rs:153`). F38: BPI album awards
  began **Apr 1973**. Exact match. FINE.
- **`album_vs_singles` trend** (`data/timeline.json`, e.g. 0.6 at 1977 rising to
  0.7 at 1980). F32 confirms the album side boomed in value (£163m vinyl album
  trade value peak, 1978) even while singles units held. Directionally right. FINE.

---

## What cannot be reconciled

Where the game's needs and the history genuinely diverge. The game should win — but
the divergence should be **documented and labelled**, not left to look like an error.

1. **One currency, one decade, no inflation index.** The evidence is a 1970–79 £
   series with double-digit inflation. The code is one flat `$` scale with era
   *modifiers* (0.8–1.5) standing in for price change. A faithful implementation
   needs a price index; a playable one needs the modifier. Recommend keeping the
   modifier, documenting it as a deliberate simplification, and **not** claiming
   era-realism for absolute levels. (This is also the honest resolution of #1.)

2. **The ×3 L-SCALE rescale versus every F-figure.** `constants.rs:44-56` shows the
   whole economy was deliberately tripled to reach "era-real" magnitude, validated
   by a sim sweep (`docs/DESIGN-v0.8-label-cycle.md:335-336`). Every comparison in
   this document therefore fights a decision that was already made and measured.
   The ratios flagged here (per-head slope #7, royalty band #4, advance multiple
   #2) survive the rescale untouched, which is why they are the trustworthy
   findings; the *levels* (#8, #13) must be re-validated by that same sweep.

3. **Certification is a progression reward, not a trade body.** F38's £-value
   scheme is genuinely awkward to simulate (it needs retail price and shipment
   value). The code's unit thresholds are simpler and more legible to a player.
   Recommend a documented, deliberate simplification here — but it should be
   *stated* in `constants.rs` next to `CERT_*`, because the current comment
   (`economy.rs:565`, "era-agnostic — the copy thresholds never move") reads as a
   historical claim, and F38 contradicts it.

4. **The dole is a floor the game may not want.** F61–F63 describe a survival
   guarantee that would remove most early-game pressure. A middle path the evidence
   supports: model it as a **low, contribution-gated** floor (F62 — school-leavers
   get nothing, which is both accurate and good design), available only while the
   band is not gigging full-time (F63's availability test). That preserves
   pressure and gains authenticity.

5. **F69 is out of period.** £25–45/band/night is c.1980–84, and the source note
   explicitly warns against silently backdating it. It is the *only* documented
   pub-fee figure, so #8 uses it as a ceiling — but the honest position is that the
   1970s pub fee is **unresearched**, and this is the single most valuable open
   research question, because it sets the entry rung of the whole game.

---

## Suggested landing order

1. **#1 currency decision** — everything else is provisional until this is settled.
2. **#14 dead JSON fields** — cheapest fix, removes a wrong-and-visible number.
3. **#4/#5 royalty band**, **#10/#11/#12 certification** — self-contained, testable,
   no dependency on the retail layer except #11(b).
4. **#13 rehearsal cost** — adds a money sink, which buys headroom for step 5.
5. **#6/#7/#8 gig economics** — land as one change, and only after the survival
   layer exists (dole F61–F63, day jobs, rehearsal rent #13) to absorb a 5–7× cut in
   entry-rung income.
6. **#2/#3 advance floors** — last, because they need the L-SIM recoup sweep re-run.
