# The survival layer: design proposal

**Scope.** The early game: no money, no deal, a day job. This document prices that layer against
`docs/research/uk-1970-1979/findings.csv` and proposes mechanics. It depends on **none** of
`FUTURE.md` §2–4 (Musician, relationships) — those are deferred, and every mechanic below is
expressible with the fields and actions that exist today.

**Conversion.** £1 = $1.65 throughout, the same assumption the gap analysis used
(`evidence/design-gap-analysis.md:15`). Marked *(arith)* wherever it is applied. That file also
labels the rate an *assumption*, not a finding, and this document inherits that caveat: every
`$` figure below is only as good as the 1.65.

**Evidence labels.** `[Fnn]` = a row of `findings.csv`. `[DERIVED]` = arithmetic on F-rows, shown.
`[INVENTION]` = a design constant with no documentary basis, stated as such. Nothing in this
document is presented as a period fact unless it carries an F-id.

---

## What exists now

A week in the current build costs the player **$0** and can pay **$77**.

**Cost side.** The default lifestyle is `LifestyleTier::Squat`
(`src/game/player.rs:48-56`, `Default` derived), whose upkeep is `LIFESTYLE_UPKEEP_PER_WEEK[0] = 0`
(`src/game/constants.rs:430`). Upkeep is the *only* recurring money sink in
`update_lifestyle` (`src/game/lifestyle.rs:85-90`), and it is zero at that tier. `action_practice`
(`src/game/actions/studio.rs:345-363`) charges nothing — it adds `PRACTICE_STRESS_COST = 4`
(`constants.rs:304`) and grants `+2` band skill. Travel to a gig is not charged anywhere:
`action_play_gig` calls `self.player.earn_money(earnings)` with no deduction before it
(`src/game/actions/live.rs:327`). So a squat-dwelling band that practices and gigs has **no
weekly outgoings at all**.

**Income side.** `Player::default()` sets `money: 0` (`src/game/player.rs:127`). A fame-0 band can
only reach venue 0 (`prestige 10 <= fame 0 + 20`), whose `base_payment` is `$300`
(`src/game/world/venues.rs:34`) at capacity 50. Working the formula at `live.rs:303-304`:

```
base_ratio        = (0 + 10) / (10 + 10)                    = 0.50
attendance_ratio  ≈ 0.50 × 1.00 (mid reception factor)       = 0.50
market_modifier   = 0.50 (demand) × 0.725 (saturation) × 1.1 (Growing)
                                                            = 0.39875
era_modifier      = 1.3 in 1975 (data/timeline.json:113)    = 1.3
earnings          = 300 × 0.50 × 0.39875 × 1.3              = $77.76 → $77
```

$77 = **£46.7** *(arith: 77 / 1.65)*. In 1970 (`gig_pay_modifier` 1.0, `timeline.json:40`) the
same night pays $59.8 → £36.2.

**The gap, in one line.** The code's first gig pays **£36–47** and costs **£0**; the evidence says
an unknown band's night is worth **£10–20** (below) and costs **£21–28** to live through. The
early game is missing roughly **£50–64/week of cost** and overpays the gig by **£25–35**.

**Three contradictions to record before designing.**

1. **The lifestyle ladder is a status sink, not a survival floor.** The comment at
   `constants.rs:427-429` says the ladder was scaled ×3 "to the era-real scale". It was scaled in
   the wrong direction at the bottom: tier 1 is `$120` = **£72.7/wk** *(arith: 120 / 1.65)*, which
   is **109%** of the £66.97 average manual weekly wage [F61]. A shared flat that costs more than a
   manual week is not a shared flat. The ladder should be left alone as the *aspirational* axis
   (its upper rungs are fine); what is missing is a separate **subsistence** floor beneath tier 0.
2. **`check_game_over` contradicts the eviction clock.** `turn.rs:426` ends the run the moment
   `money < 0 && fame < 10`. But `LIFESTYLE_EVICTION_WEEKS = 2` (`constants.rs:448`) assumes a
   player can be in the red for two weeks before the landlord acts. Two rules, one of which
   never fires. The instant-death rule is what makes a survival layer impossible; it must become
   a clock.
3. **Free practice makes money the only non-scarce resource.** Because rehearsal costs nothing
   (`studio.rs:345-363`) and skill drives reception, the early game's scarce resource is *time*,
   not cash. Once rehearsal is priced [F59], time and money compete — which is the whole point of
   a survival layer.

---

## A worked week, 1975

Four-piece, provincial city, single man, one room in a shared flat. All figures **per musician
per week**, in 1975 pounds.

### Costs

| Line | £/wk | Basis |
|---|---|---|
| Rent, one room, shared flat | 8.00 | [INVENTION] 11.9% of the £66.97 wage [F61] |
| Food | 7.50 | [INVENTION] 11.2% |
| Local transport (bus/tube) | 1.50 | [INVENTION] 2.2% |
| Rehearsal, 4-hour block at 75p/hr | 3.00 | [F59], documented provincial rate, 1973 |
| Strings, leads, breakage | 1.00 | [INVENTION] 1.5% |
| **Subsistence basket** | **21.00** | = **$34.65** *(arith)* |

London variant: rehearsal is £4.00/hr [F59], so the 4-hour block is **£16.00** and the basket is
**£34.00** — 51% of the average manual wage. The London premium is documented; its size is
striking and is a design lever in its own right.

### Income

**The fee.** [F69] gives £25–45 **per band per night** in pubs, but the fieldwork window is
**c.1980-84** and must not be backdated silently. Average manual weekly earnings were £66.97 in
1975-76 [F61]; by 1982-83 they were roughly £150 *(external knowledge — NOT in findings.csv; this
is the weakest link in the document and is flagged again in §"Determinism, serde and grounding")*.
Deflator ≈ **2.2×**. Two independent checks agree: an RPI deflator 1975→1982 is ≈ 2.2-2.4.

```
£25 / 2.2 = £11.4     £45 / 2.2 = £20.5     →  £10-20 per band per night, 1975 money  [DERIVED-ESTIMATE]
```

Split four ways: **£2.50–£5.00 per musician per night.** This is the document's load-bearing
number and it is an estimate, not a finding.

Corroboration that it is the right order of magnitude: [F70] records a solo club residency at 25s
(£1.25) a night in c.1969-71, and [F71] records support slots as unpaid. A £2.50-£5.00 share for
an unknown four-piece sits correctly between those two poles.

**The dole.** £9.80/week from April 1975 for a single man [F61] = **46.7%** of the basket. It is a
lifeline that does not reach the shore.

### Four configurations

| Week | Income | Cost | Net |
|---|---|---|---|
| **A.** No gig, on the dole | £9.80 | £21.00 | **−£11.20** |
| **B.** One local gig, on the dole | £9.80 + £2.50-5.00 | £21.00 | **−£6.20 to −£8.70** |
| **C.** Two local gigs, on the dole | £9.80 + £5.00-10.00 | £21.00 | **−£1.20 to −£6.20** |
| **D.** Full-time day job + one Saturday gig | £58.26 + £2.50-5.00 | £21.00 + job costs | **≈ +£38** |

Configuration D is the only one that accumulates, and it is the historical norm that [F72]
describes: pub-rock bands surviving on pub work with members "getting on each others pockets".
**In 1975 the band is a hobby financed by a job.** That is not a failure of the design — it is the
period, and it is the most interesting thing the evidence says.

### The away gig, and why the van decides everything

A 200-mile round trip, 20 mpg [INVENTION for a loaded Transit], 71.5p/gal [F60]:

```
71.5p ÷ 20 mi = 3.575p/mile   × 200 mi = 715p = £7.15
```

Against a £15 band fee (mid of the deflated range), fuel is **47.7%** of the fee. After fuel the
band clears £7.85, or **£1.96 each** — 9.3% of the weekly basket for a night's work and 200 miles.

To cover the £21 basket on away gigs alone you would need `21 / 1.96 = 10.7` away dates a week.
A four-piece can play four. **Away dates are structurally loss-making at this tier**, which is
exactly why [F71] records support slots as unpaid and why the sector ran on local pub work.

### Could a band survive on gigs alone?

Marginal case: three local nights a week at the **top** of the range, £33.60/band/night
(= £8.40 each), gives £25.20 against £21.00 → **+£4.20**. That is subsistence with zero margin for
gear failure, illness, van repairs or a single away date — and two away dates put it under water
(£25.20 − 2 × £8.56 of fuel = +£8.08 for the band, i.e. £2.02 each — still underwater once
anything else breaks). So: **not reliably.**
Only at the ceiling of the fee band, only locally, only while nothing breaks.

---

## A worked week, 1978

Same band, three years on. The documented shocks are fuel [F60] and the dole's falling real value
[F64]; the basket uprating below is **[INVENTION]**, applied at ~+30% and labelled as such.

### Costs

| Line | 1975 | 1978 | Basis |
|---|---|---|---|
| Rent | 8.00 | 10.40 | [INVENTION] +30% |
| Food | 7.50 | 9.75 | [INVENTION] +30% |
| Transport | 1.50 | 2.00 | [INVENTION] |
| Rehearsal | 3.00 | 3.90 | [INVENTION] 75p → 90p/hr; no 1978 rate in the base — see gap |
| Strings | 1.00 | 1.50 | [INVENTION] |
| **Basket** | **21.00** | **27.55** | = **$45.46** *(arith)* |

### Income

- **Dole:** £12.90/week [F61, from 15 Nov 1976]. This is the *last documented single rate* in the
  base; the Nov 1977 and Nov 1978 upratings are **not** in `findings.csv`. Gap, flagged below.
- **Fee:** [F69]'s £25–45 deflated to 1978 by ~1.35 (not 2.2) → **£18.50–33.30/band/night**
  → **£4.63–£8.33 each** [DERIVED-ESTIMATE].

| Week | Income | Cost | Net |
|---|---|---|---|
| Two local gigs + dole | £12.90 + £9.26-16.66 | £27.55 | **−£5.39 to +£2.01** |
| One local gig + dole | £12.90 + £4.63-8.33 | £27.55 | **−£10.02 to −£6.32** |
| Two gigs, one of them away (200 mi) | above − £8.56 fuel | £27.55 | **−£13.95 to −£6.55** |

### The 1978 story is the road, not the rent

Nominal fuel [F60] and the same 20 mpg:

| Year | p/gal | p/mile | 200-mi round trip | % of a £15 band fee |
|---|---|---|---|---|
| 1970 | 32.5 | 1.625 | £3.25 | 21.7% |
| 1975 | 71.5 | 3.575 | £7.15 | 47.7% |
| 1977 | 79.3 | 3.965 | £7.93 | 52.9% |
| 1978 | ~85.6 *[INVENTION: +8% on 1977]* | 4.280 | £8.56 | 57.1% |

**Fuel cost per mile more than doubled across the decade, and it did it in one step** — the
February 1974 shock is visible in [F60]'s series. A band that bought its van in 1971 and kept it
saw the cost of reaching the same gig rise by 163% while the fee roughly doubled. That wedge is
the single most period-true pressure available, and the current build models none of it.

Meanwhile the dole's real value fell from 39.5% to 29.8% of average earnings between 1971 and 1979
[F64]. In 1975 the dole covered 46.7% of the provincial basket; by 1978, £12.90 against £27.55 is
**46.8%** — nominally identical, and *worse* in real terms because the upratings lagged the wage.
Modelling the dole as an accurate **nominal table** with a **basket that tracks the wage** produces
[F64]'s squeeze for free. No separate "decay" mechanic is needed, and none should be added.

---

## Proposed: the day job

**Shape.** A standing state on the player, like `LifestyleTier`, not an action:

```rust
#[serde(default)] pub day_job: DayJob,          // None | PartTime | FullTime
#[serde(default)] pub job_tenure_weeks: u16,    // feeds the dole's contribution test
#[serde(default)] pub job_stress_debt: u8,      // unused if you keep it simple
```

**Why a state and not an action.** The engine gives one action per week (`turn.rs:360-410`). A job
that consumed the action would make the game unplayable as a music sim. A job is a *background
condition* that changes what the week costs and what it forbids. This is the smallest possible
surgery.

### Income

| Tier | £/wk (1975) | Derivation |
|---|---|---|
| Part-time (≈20 h) | 30.00 | [INVENTION] 45% of the manual wage |
| Full-time (≈40 h) | 58.26 | [DERIVED] 87% of £66.97 [F61] — a young manual worker below the average |

Full-time at 87% is a deliberate design choice: the average manual wage is *everyone's* average,
and a 19-year-old in 1975 is below it. Raise to 100% if the sim shows the job is too weak.

### What it costs

1. **Stress.** Full-time `+4/wk`, part-time `+2/wk`, applied in `update_lifestyle`. Against
   `STRESS_PASSIVE_RELEASE = 3` (`constants.rs:273`) a full-time job plus one gig (`GIG_STRESS_COST
   = 8`, `constants.rs:387`) nets **+9 stress/week** — the player cannot gig every week and hold
   the job without hitting `GIG_STRESS_GUARD = 85`. That is the trade, and it needs no new system.
2. **Access.** Full-time forbids **touring** and **recording**; part-time forbids touring only.
   Implementation is one guard in `action_go_on_tour` and one in `book_recording`, both of which
   already return `Err(String)`. The cleanest rule: booking either **auto-quits the job** with a
   log line ("You didn't come back on Monday"), which is more period-true than a refusal and gives
   a hard, legible decision.
3. **No practice penalty.** Tempting, but it double-counts the stress cost. Leave practice free.

### Why the job is not just income

[F62] is the decisive finding: contributions **buy** dole entitlement, and a band of school-leavers
qualifies for **nothing**. Turn that into the job's second payoff:

```
25 × £15 LEL = £375   ÷ £66.97/wk = 5.60 weeks  →  6 weeks of full-time work buys the basic rate
50 × £15 LEL = £750   ÷ £66.97/wk = 11.20 weeks →  12 weeks buys the full rate
```

[DERIVED from F62 × F61]. The contribution test is a test on **earnings**, not on weeks; the
week-equivalent above is the game's translation and is labelled as such. It produces a good
number: **six weeks of work and you can sign on; twelve and you can live on it.** A band that
never worked has no safety net at all, which is precisely [F62]'s point and the sharpest available
statement of why the dole is not free money.

### The quit decision

`GameAction::QuitJob` and `GameAction::TakeJob(DayJob)`, both **instant** — added to the
non-consuming match arm at `turn.rs:385-397`, alongside `ChangeLifestyle`. Quitting resets
`job_tenure_weeks` to 0 for *future* entitlement but **preserves the accrued contribution record**
(that is how Class 1 works: the record is history, not a streak). Store
`contribution_weeks: u16` separately and never reset it.

---

## Proposed: the dole

**Shape.**

```rust
#[serde(default)] pub claiming_dole: bool,
#[serde(default)] pub dole_weeks_claimed: u16,
#[serde(default)] pub contribution_weeks: u16,   // accumulates only while employed
#[serde(default)] pub dole_waiting_days_left: u8,
#[serde(default)] pub dole_disqualified_weeks: u8,
#[serde(default)] pub dole_strikes: u8,
```

### 1. Eligibility — a gate, not a default

Claimable only if `contribution_weeks >= 6` (basic) or `>= 12` (full rate) [DERIVED, above]. A
fresh band at week 0 has 0 and **cannot claim**. This alone changes the early game more than any
other line in this document: the first six weeks are unfunded.

### 2. The waiting days — [F63]

Every **new** spell loses 3 waiting days. Implement as: the first week of a claim pays `4/7` of the
rate.

```
£9.80 × 4/7 = £5.60 in 1975
```

Not zero — the player still eats — but the first week back on the dole is always the worst week.

### 3. The status risk — [F63], and the trap

[F63]'s test is **"capable of, and available for, work"** — a *status* test, not an earnings test.
A gigging musician on the dole is not obviously available. Model it as a per-week seeded roll,
drawn from `action_rng()`:

```
p_strike = 0.08 + 0.02 × gigs_this_week + fame/500 + 0.05 × dole_strikes
```

At fame 10 with two gigs: 8 + 4 + 2 = **14%/week**. Over a 20-week claim that is near-certain
detection — which is the point. **On a strike:** benefit stops for `DOLE_DISQUALIFY_WEEKS = 6`,
happiness −10, `dole_strikes += 1`. The player is pushed toward the honest choice (declare, and
lose the benefit) or the criminal one (gig and pray), and both are legible.

**Declaring** is the honest path and should exist: a `DeclareEarnings` sub-choice that reduces the
benefit pound-for-pound for that week but carries no risk. That is the real 1970s dilemma and it
costs one branch.

### 4. Decaying generosity — [F63] and [F64]

- **Earnings-related supplement** ran from day 13, capped at 156 days [F63]. Model: weeks 1-2 pay
  basic only; weeks 3-22 pay basic + **25% [INVENTION — the amount is not in the base]**; after
  week 22, flat basic.
- **The rate table must be nominal by year** — £9.80 (1975), £11.10 (1976), £12.90 (from 15 Nov
  1976) [F61], with the 1977-78 upratings to be filled from the same Hansard series. The real
  squeeze [F64] then emerges from the basket rising faster than the rate. Do **not** add an
  inflation-drift multiplier; [F64]'s own guidance is "model a dated step function, never inflation
  drift", and the nominal table *is* the step function.

### 5. Why this is a trap and not free money

- You cannot claim at week 0 (contributions).
- The first week pays 4/7 (waiting days).
- Gigs are a 14%/week audit risk (status test).
- The rate covers 47% of a provincial basket at best.
- It ends the moment you take a job — and the job is what rebuilt the entitlement.

The dole is a **bridge between jobs**, which is what it was. It is not an income strategy.

---

## Proposed: gear as progression

### The ladder, from [F58]

The counter-intuitive finding: in 1975 a **new-budget** full rig cost ~£512 while a **used-pro**
rig cost ~£688. The poor band bought *new copies*; the band with gig money bought *old Fenders*.
The ladder runs **new-cheap → used-pro**, and the upward move is **quality, not novelty**.

| Slot | New budget | Used pro | Source |
|---|---|---|---|
| Guitar | £36.57–45 | £160 | [F56], [F57] |
| 4-drum kit | £122.94 | £170 | [F56], [F57] |
| Amp (100W) | ~£98 *(new equiv.)* | £61–88 | [F57] |
| PA | £71.30 | — | [F56, 1971 price] |
| **Full 4-piece rig** | **~£512** | **~£688** | [F58] |

### The wall

```
£512 new-budget rig ÷ £66.97 manual wage  = 7.6 weeks of full-time work
£512 ÷ £9.80 dole                          = 52 weeks on the dole
£512 ÷ £15 band fee                        = 34 gigs, whole fee, nothing taken home
```

**7.6 weeks of work buys the tools; 52 weeks of dole does not.** This is the strongest possible
argument for the day job and it is pure arithmetic on [F56] and [F61]. It should be the first thing
the player learns.

### What gear *does* — three levers, not one stat

A flat "gear quality → +N% earnings" is the wrong shape, because it makes the cheap kit simply
worse and destroys [F58]'s insight that cheap-and-new was the *rational* choice. Give gear three
independent jobs with three different decision shapes:

**1. Reliability — a variance lever, not an average lever.**
Each slot has `condition: u8`. Every gig draws a breakdown roll:

```
p_fail = (100 - condition)/500 + 0.06   for new-budget   → ≈ 12%/gig at condition 100
p_fail = (100 - condition)/500 + 0.00   for used-pro     → ≈ 4%/gig at condition 100
```

*[INVENTION — the rates; the mechanism is the point.]* On failure the night pays **half**, and a
repair bill of **15% of the item's price** is charged. Cheap gear is not worse *on average* — it is
worse *in the tail*, and at pub tier the tail is survivable. That is exactly why a broke band
bought the copy. Used-pro only earns its premium once a lost night costs more than the repair.

**2. Ceiling — a gate on the fee ladder.**
The top venue tiers require a PA grade the pub rig cannot supply. Rule: venue prestige ≥ 70
(2,000-cap) requires PA grade ≥ 3. *[The gate is a design choice; that a 2,000-capacity room needs
more PA than a pub rig is an inference, flagged as such.]* This ties gear to the **fee**, not to a
stat, and gives the player a concrete reason to spend £200 rather than a number going up.

**3. Sound — a small, capped reception bonus.**
`+0..+6` on the reception score, capped hard. Skill comes from free practice (`studio.rs:345-363`);
gear must never substitute for it, or the practice loop dies. Cap at 6 so gear is a tiebreaker on a
close night, never a substitute for a hundred weeks of rehearsal.

### Why this ladder is better than a flat stat

| Choice | New-budget | Used-pro |
|---|---|---|
| Cash now | £512 | £688 |
| Failure tail | 12%/gig | 4%/gig |
| Resale | ~30% | ~60% (it is already old) |
| Right when | fee < £20/night | fee > £40/night |

The decision inverts with success, and it inverts *because of the failure tail*, not because of a
bigger number. That is the mechanic [F58] is asking for.

---

## Proposed: the van

### Purchase

The only van price in the base is a **1965 new Transit at £542** [F60], and the note is explicit
that no 1970s used-van price was found after four search rounds. Derive rather than invent:

```
1965 new £542, ten years old by 1975, used market at 40-70% of new [F57] → £217-£271
                                                            → £240 [DERIVED-ESTIMATE]
```

£240 is **3.6 weeks of full-time work** *(arith)* or **16 gigs** at a £15 band fee. It is cheaper
than the rig, which is correct and makes it the natural *first* purchase.

### What the van gates

1. **Away gigs.** Without a van, gigs are home-city only.
2. **Touring.** `action_go_on_tour` requires a van; there is no bus rental at this tier.
3. **The PA.** A PA will not travel on a bus. Without a van you cannot use PA grade ≥ 1, which
   caps you below the venue tier that needs one. Gear and van are therefore coupled: the PA is
   useless without the van, and the van is pointless without the PA.

### Running cost — tied to [F60]

```
cost_per_mile(year) = fuel_p_per_gal(year) / MPG
MPG = 20 [INVENTION — a loaded Transit]
```

| Year | p/gal [F60] | p/mile | 200-mi round trip |
|---|---|---|---|
| 1970 | 32.5 | 1.625 | £3.25 |
| 1975 | 71.5 | 3.575 | £7.15 |
| 1977 | 79.3 | 3.965 | £7.93 |

Charged per away gig, deducted before the fee is split. The **1974 oil-shock step** is in [F60]'s
series and lands automatically because the table is dated — no event needed.

### Breakdown risk

```
p_breakdown = (100 - van_condition)/400 + mileage/100_000
```

*[INVENTION.]* On failure: the date is lost (fee 0), repair bill = £30 + 5% of van value, and
`van_condition` drops 20. Condition recovers +2/week with a service costing £8. A 1975 van at
condition 60 with 60k miles breaks down **16%/trip** — often enough to be felt, rare enough to
gamble on.

### The capacity limit — a fuel budget, not a range stat

```
max_round_trip_miles = tank_gal × MPG = 12 × 20 = 240 miles   [INVENTION on the 12 gal]
```

Beyond 240 miles a date needs a fuel stop, and the rule is: **the band refuses any date whose fuel
exceeds 50% of the fee.** At a £15 fee that is 200 miles — one round trip. At a £45 fee it is 600
miles. So **the van's touring range is a function of the band's fee tier**, which is the correct
coupling and needs no new stat. A band that cannot raise its fee cannot leave its county, however
good its van.

This also prices [F60] where it actually bites: not on the headline tour (`TOUR_RIG_COST_PER_WEEK`
is `[360, 1200, 2800, 6500]`, `constants.rs:492`, an order of magnitude above £7.15 of fuel) but
**at the bottom of the ladder, where the code currently charges nothing at all**.

---

## Proposed: poverty, burnout and quitting

### Replace the instant loss with a poverty clock

`turn.rs:426`'s `money < 0 && fame < 10` ends the run in one bad week. Replace with:

```rust
#[serde(default)] pub weeks_destitute: u8,
```

- Increments while `money < -£20` *[INVENTION: a small overdraft tolerance]*; resets at ≥ 0.
- **Week 4:** warning — "the band is eating into next month's rent".
- **Week 10:** the band dissolves. `game_over`, with a distinct ending text.
- Health −2/week while destitute. Poverty hurts the body before it kills the band.

This preserves the existing `LIFESTYLE_EVICTION_WEEKS = 2` rent rule (a *lifestyle* consequence)
while making destitution a *career* consequence on a longer clock. The two no longer contradict.

### Burnout

No new RNG. Uses existing stress:

- `stress >= 90` for two consecutive weeks → the following week's action is forced to
  `LazeAround`, and creativity −5. One log line. Fully deterministic.
- This is what makes the day job + gig combination bite: at +9 stress/week net, a full-time player
  who gigs every week reaches 90 in ten weeks and loses a week.

### Morale and the band walking

`BandMember::loyalty` exists and is currently unused for pressure (`src/game/band.rs:46`):

- While destitute, each member loses `3` loyalty/week *[INVENTION]*.
- At `loyalty < 20`, a seeded roll of `(20 - loyalty)%` per week → the member quits, taking their
  `skill` out of `average_member_skill()`, which feeds reception. The band gets audibly worse
  before it dies. That is the correct causal chain and it needs no new subsystem.

### The voluntary quit

`GameAction::Quit` already exists (`src/game/core.rs:86`). The survival layer should make it a
*real* ending rather than an escape hatch: "take the job, keep the flat, sell the amp" — with a
summary of what the band was. A trap is only a trap if walking out of it is legible.

---

## What this does to the early game

### Today

Fame 0, week 1: one gig = +$77, rent = $0, practice = $0. **The player can go full-time on turn
one.** There is no early game; there is a switch.

### Proposed target

| Milestone | Target | Why |
|---|---|---|
| First £100 saved | week 8-12 | 2 gigs/wk at £2.50-5.00 each, minus £21/wk subsistence |
| Van bought | week 25-35 | £240 = 3.6 weeks of wage, or 16 gigs |
| Full rig (new-budget) | week 40-60 | £512 = 7.6 weeks of wage |
| First paid support / club headline | week 30-50 | requires fame ~10-15 and the PA gate |
| **Full-time (no job, no dole)** | **week 60-90** | fee must reach £8.40/musician/night for 3 nights |

The full-time threshold is the design's real target. At the *top* of the unknown-band fee band
(£33.60/band/night), three nights clear £25.20 against a £21 basket: **+£4.20/week.** That is the
knife-edge. Full-time at this tier is possible and terrifying, which is exactly right — and it
means the mid-game becomes a hunt for the **fee tier**, not a grind for more gigs, because four
nights a week is the physical ceiling and the fifth night does not exist.

Above that, [F69]'s £45 ceiling and [F03]'s £1,000 for 10cc in 1974 leave a **documented chasm**
with nothing in it. That chasm is the correct place for the record deal to sit: the survival layer
ends when the deal arrives, not when the player out-grinds the pubs. **The exit from the survival
layer is a contract, not a wage.**

### The three mechanics that matter most

1. **Subsistence cost per week (£21 in 1975)** — without it nothing else is a decision.
2. **The contribution gate on the dole** — makes the day job structural rather than optional.
3. **Fuel per mile, dated** — turns the van from a purchase into a geographic cage.

---

## Determinism, serde and grounding

### Determinism

- **One stream.** Every new roll comes from `self.action_rng()` (`src/game/rng.rs:88`), already
  created once per turn at `turn.rs:366`. No `thread_rng`, no `SystemTime`, no hash iteration.
- **Draw order is the contract** (`src/game/rng.rs:26`: "Draw order on each stream is sacred").
  Every new draw must be appended **after** the existing draws for that week, or a fixed seed
  replays a different career than it did before the change. This is the single most likely way to
  break the determinism tests, and it is invisible until a replay diverges.
- **Tour steps** use `activity_rng(booking_week, step)` (`rng.rs:94`), keyed by step index. Van
  breakdowns inside a tour belong on that stream, not the action stream, for exactly the reason
  the comment at `rng.rs:70-78` gives.
- **No RNG for burnout** — the stress threshold is a deterministic latch.
- **Regression test:** a fixed seed, 200 weeks, assert the exact final `money`. Add it before the
  first mechanic lands, so every subsequent change is checked against it.

### Serde and save compatibility

- **Every new field carries `#[serde(default)]`** — matching the existing pattern at
  `player.rs:22-23` (`lifestyle`) and `core.rs:374-393`.
- **Never rename.** `LIFESTYLE_UPKEEP_PER_WEEK` keeps its name and its array; the subsistence floor
  is a **new** concept, not a re-purposing of tier 0.
- **New enums need a `Default`** so `#[serde(default)]` compiles: `DayJob::None` must be the
  default, so a pre-change save loads with no job, no van, no contributions — i.e. **exactly the
  current experience**, which is the correct migration.
- **Placement:** `day_job`, `contribution_weeks`, `weeks_destitute` on `Player`; `has_van`,
  `van_condition`, `van_mileage` on `Band` (a band asset, not a personal one); `claiming_dole` and
  the dole clocks on `Game` (they are turn state). Keeping them off `Game` where possible limits
  the blast radius of a schema mistake.
- **No deletions.** Removing `check_game_over`'s money clause is a behaviour change, not a schema
  change — old saves simply stop dying at week 2.

### Grounding ledger

| Constant | Value | Source |
|---|---|---|
| Subsistence basket 1975 | £21.00/wk | 2 lines [F59][F61], 3 lines [INVENTION] |
| Dole single, 1975 | £9.80 | [F61] |
| Dole single, Nov 1976 | £12.90 | [F61] |
| Waiting days | 3 of 7 | [F63] |
| Contribution gate | 6 / 12 weeks | [DERIVED] F62 × F61 |
| Full-time wage | £58.26 | [DERIVED] 87% of [F61] |
| Rehearsal | 75p/hr prov., £4/hr London | [F59] |
| Fuel 1970/1975/1977 | 32.5 / 71.5 / 79.3 p/gal | [F60] |
| MPG | 20 | [INVENTION] |
| Tank | 12 gal | [INVENTION] |
| Van purchase | £240 | [DERIVED] F60 × F57 |
| Rig new-budget / used-pro | £512 / £688 | [F58] |
| Guitar / kit / PA new | £40 / £122.94 / £71.30 | [F56] |
| Guitar / kit / amp used-pro | £160 / £170 / £61-88 | [F57] |
| Band fee, deflated to 1975 | £10-20 | [DERIVED-ESTIMATE] [F69] ÷ 2.2 |
| Failure rates, repair %, loyalty drain, destitution threshold, supplement % | — | **[INVENTION]** |

### Known weaknesses in this document

1. **The 2.2× deflator rests on an external earnings figure not in `findings.csv`.** It is the
   hinge of every fee number here. If it is wrong, the whole survival layer is mispriced. This is
   the first thing to fix — see below.
2. **Rent and food are invented.** No housing or food price appears anywhere in `findings.csv`
   (checked: no rent/groceries row). Three of the five basket lines are design choices wearing a
   percentage. They are anchored to [F61]'s wage rather than plucked from air, but they are not
   evidence.
3. **No 1978 dole rate and no 1978 rehearsal rate.** Both are inferred.
4. **The 12-week van range and 20 mpg are inventions.** The *shape* (range as a function of fee
   tier) is the design idea; the numbers are placeholders.
5. **`$` and `£` coexist throughout.** The codebase is dollar-denominated and the evidence is
   sterling. The 1.65 conversion is an assumption, and every `$` figure inherits it.

### What to research next

- **A 1975-78 average earnings series** (New Earnings Survey / DE Gazette). This single table
  converts [F69]'s fees from estimate to finding and fixes weakness 1.
- **Rent and food**: a 1975 Family Expenditure Survey row, or any period rent book. Three
  invented lines would become documented.
- **The 1977 and 1978 single-man dole rates** — the same Hansard uprating series [F61] already
  cites, one year further on.
- **A used-van price, any year 1970-79.** Four rounds have failed [F60 note]; a classified
  column from *Melody Maker* or *Beat Instrumental* would close it.


---

## Addendum — the deflator check (added after the fact)

Known-weakness #1 above says the 2.2× deflator is the hinge and rests on an
external earnings figure. It was checked against two independent anchors:

| Check | Ratio 1975→1982 | Note |
|---|---|---|
| External earnings (£66.97 → ~£150/wk) | **2.24×** | the document's original assumption |
| UK RPI, 1975 average → 1982 average | **≈2.38×** | independent price-index check |

Both land in the 2.2–2.4 band, so the deflator is sound to within ~8% and the
derived **£2.50–£5.00 per musician per night** for an unknown four-piece in 1975
stands as an order-of-magnitude estimate. It does **not** become a finding: it is
still `[DERIVED-ESTIMATE]`, and the honest statement remains that the 1970s pub
fee is unresearched. What the check buys is confidence that the survival layer is
not mispriced by a factor, only by a margin.

The other four weaknesses listed above stand unchanged: rent, food, the 1978 dole
rate, the 1978 rehearsal rate, the van range/mpg, and the `$`/`£` conversion are
all design choices or assumptions, not evidence.
