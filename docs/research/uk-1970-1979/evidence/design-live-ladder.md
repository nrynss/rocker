# The live ladder: design proposal

Scope: how a band gets from a pub back room to an arena in the 1970–79 UK, and what
the player chooses at each rung. Every historical number carries an F-id from
`docs/research/uk-1970-1979/findings.csv`; every invented mechanic is marked
**[design]**. No code was edited.

---

### Conclusion in brief

The five-rung ladder should **not** be re-typed — it should be **re-framed**. The
defect is not the capacity spine, which is close to right, and not the payment
array as such, which is badly mis-scaled at exactly one rung (the theatre,
+508%); it is that a single `base_payment` scalar is
used as *four different kinds of money* — a landlord's fee at the pub, a
promoter's percentage split at the club and theatre, an institutional guarantee
at the SU, and a net headline fee at the arena — and is then multiplied by
`attendance_ratio` in every case (`venues.rs:34`, `live.rs:303-305`).
That is why the per-head take *falls* with size while F01 says the ticket price
*rises*: the money is not measuring the same thing at both ends of the table.

So the highest-value change is a **`DealKind` on `Venue`** (`FlatFee`,
`FeePlusPercentage`, `Guarantee`, `Percentage`) with a **ticket price per rung**,
so the gate is computed and then split, instead of a fee being scaled. Under it,
net per head across the three gate-share rungs rises **$4.87 → $8.11 (1.67×)**
in step with F01's **7×** ticket ladder, and the ladder grows by **exactly one
rung** — a 4,000 hall filling the 2,000 → 20,000 gap — with the arena demoted to
a one-off.

Two things must be fixed at the same time or the re-framing will not pay off:
the **support slot must stop paying** (it currently banks $3,000 from fame 5,
`turn.rs:158-161`, against F71's unpaid reality), and **tours need a per-show
cost stack** — without one, rig choice collapses to "buy the biggest you can
afford" ($207,736 vs $131,542); with one, the **bus** is the optimum and the
truck and full-production rigs lose $6,508 and $29,476 respectively. That is
F03's £750-on-£1,000 inversion turned into a decision.

Overall confidence: **high** on the diagnosis and on the direction of every
correction (each rests on F01/F02/F03/F71, all high or medium confidence); **low
on the exact 1974 £/$ scale**, which is a labelled design anchor (`GIG_SCALE =
16`), and on the intermediate ticket prices, which no source in `findings.csv`
supplies.

---

### What exists now

Five venues, generated once per world, one per district
(`src/game/world/venues.rs:27-34`):

```rust
let capacities = [50, 200, 500, 2000, 20000];
let prestiges  = [10, 25, 40, 70, 95];
let payments   = [300, 900, 2400, 9000, 45000];   // L-SCALE ×3
```

One gig action, one formula (`src/game/actions/live.rs:288-305`):

```rust
let base_ratio = ((self.band.fame as f32 + 10.0) / (venue.prestige as f32 + 10.0)).min(1.0);
let attendance = (venue.capacity as f32 * attendance_ratio) as u32;
let earnings = (venue.base_payment as f32 * attendance_ratio * market_modifier * era_modifier) as u32;
```

and it is credited as cash with nothing deducted (`live.rs:327`,
`self.player.earn_money(earnings)`).

Four structural consequences, all verifiable in the current code:

1. **Per-head take falls as the room grows.** At a full house:
   300/50 = **$6.00**, 900/200 = **$4.50**, 2400/500 = **$4.80**,
   9000/2000 = **$4.50**, 45000/20000 = **$2.25**. Smallest to largest is a
   **2.67× fall** — the opposite of F01, where a cheap gig is 50p and Wembley is
   £3.50, a **7× rise**.
2. **The number in `base_payment` is a different *kind* of money at every rung.**
   This is the actual defect, and it is why (1) cannot be fixed by re-typing the
   array. At 1974 prices (F01) and a 50.67% band share (F02, below), a full pub
   gate is 50 × 50p = £25 and the band's $300 is **75% of the gate** — i.e. a
   landlord's fee, which is what the pub circuit actually paid (F69, F72). At
   the 20,000 arena the gate is 20,000 × £3.50 = £70,000, and $45,000 is
   **≈4% of it** — a net, not a gross. The same scalar is used as a fee at the
   bottom and a net at the top, and then multiplied by `attendance_ratio` in
   both cases.
3. **No gate exists.** Earnings never mention `capacity × ticket price`, so a
   full house pays the same as an empty one scaled by the same ratio and the
   ticket ladder of F01 is unrepresentable.
4. **Fame is the only door policy.** `venue.prestige > fame + 20`
   (`live.rs:275`) is the whole booking system. There is no relationship, no
   promoter, no agent, no standing.

Two further current-state facts matter for the targets below.

**The support slot is paid, and paid well.** `src/game/turn.rs:158-161`:

```rust
let base_pay = weeks * (150 + u32::from(host_fame) * 15);
let pay = (base_pay as f32 * self.timeline.get_gig_pay_modifier()) as u32;
```

A 4-week slot behind a fame-40 act banks 4 × (150 + 600) = **$3,000** — more
than the 500-cap SU show's $2,400 base at fame 70, and it is offered from fame 5
at 6%/week (`constants.rs:163-166`). F71 says a support slot was worth
effectively nothing, with expenses at best. The current mechanic is both
ahistorical and the strongest early cash path in the game.

**Tours already quote honestly, and already lose money on purpose.**
`docs/DESIGN-v0.7-money-cycle.md:44` ("Fame never selects a cost tier again"),
rig cost `360/1200/2800/6500` per week (`constants.rs:492`), and the picker
shows a projected net range before booking (`live.rs:532`). What is missing is
any *per-show* cost line: `tour_pot` computes one whole-tour cost from weeks ×
rig (`live.rs:458-465`) and the gross is scaled by the rig's capacity multiplier
(`live.rs:496-503`), so once a rig is affordable a bigger rig is almost always
strictly better.

---

### Proposed: the venue ladder

#### Does five rungs become six?

Yes — **one** new rung, and no capacity changes to the existing five. The
current ratio gaps are **4× / 2.5× / 4× / 10×**: the widest jump by far is
2,000 → 20,000, which is exactly where the evidence puts a tier of large halls
and ballrooms. So insert a **4,000 hall** between the theatre and the arena, and
take the 20,000 arena **off the weekly menu**, making it a bookable one-off.
Both moves are arithmetic, not taste (see *Balance*).

The college/student-union circuit is **not** a capacity — modelling it as a rung
would be a category error. It is a *deal type* (a fixed institutional
guarantee), which is what gives the middle of the ladder its economic character.

#### The corrected ladder

Money in 1974 £ (the evidence unit), converted at **GIG_SCALE = 16** [design:
anchored so a pub fee of £18.75 reads as today's $300]. Band share of face value
= **50.67%**: 1 − 1/3 promoter (F02) − 8% VAT (F02) − 8% box office **[design;
F02 names the deduction without giving a rate]**.

| # | Rung | Cap | Ticket | Gate at full house | Deal kind | Band gross | Show cost stack | Band net | Net/head † |
|---|------|-----|--------|--------------------|-----------|-----------|-----------------|----------|-----------|
| 1 | Pub back room | 50 | 50p (F01) | £25 = $400 | **Landlord's fee** (F69, F72) | $300 | $0 (walk in, house PA) | **$300** | $6.00 |
| 2 | Club / ballroom | 200 | 75p [design] | £150 = $2,400 | **50.67% of the gate** (F02) | $1,216 | $243 (20%) | **$973** | $4.87 |
| 3 | College SU | 500 | £1.00 [design] | £500 = $8,000 *(guarantee quoted off a 60% house)* | **Guaranteed fee** [design] | $1,950 | $195 (10%) | **$1,755** | $3.51 |
| 4 | Theatre / civic | 2,000 | £1.50 [design] | £3,000 = $48,000 | **50.67% of the gate** (F02) | $24,320 | $9,728 (40%) | **$14,592** | $7.30 |
| 5 | Hall / ballroom | 4,000 | £2.00 (F01 "average big band") | £8,000 = $128,000 | **50.67% of the gate** (F02) | $64,853 | $32,427 (50%) | **$32,427** | $8.11 |
| 6 | Arena (one-off) | 20,000 | £3.50 (F01 Wembley) | £70,000 = $1,120,000 | **Headline fee vs 30% margin** (F03) | $150,000 [design cap] | $105,000 (70%) | **$45,000** | $2.25 |

† Net/head = band net ÷ capacity: per seat *offered*, not per seat sold — which is why
the SU, priced off a 60% house, reads low.

The arithmetic, shown once per deal kind:

- **Gate share** (club, theatre, hall): `cap × ticket × 16 × 0.5067`.
  Theatre: 2,000 × $24 = $48,000, × 0.5067 = **$24,320**; × (1 − 0.40) = **$14,592**.
  Hall: 4,000 × $32 = $128,000, × 0.5067 = **$64,853**; × (1 − 0.50) = **$32,427**.
- **Guarantee** (SU): priced at **80% of the risk-adjusted gate share** [design] —
  a 60% house is 300 × $16 = $4,800, × 0.5067 = $2,432, × 0.80 = **$1,950**.
  The 20% discount *is* the price of certainty.
- **Fee** (pub, arena): a number the band is paid, not a fraction. Pub $300 is
  **75% of a full £25 gate** — a landlord's fee out of beer money. Arena
  $150,000 × 0.30 = **$45,000**, exactly today's arena payment: today's $45,000
  is a correct *net* that has been paid as a *gross*.

**The slope, corrected.** Across the three gate-share rungs, net per head runs
**$4.87 → $7.30 → $8.11**, a **1.67× rise**; the whole ticket ladder runs
**50p → £3.50**, a **7× rise**, which is F01 exactly. Today's ladder runs
$6.00 → $2.25 per head, a **2.67× fall** — the slope flips sign.

Two dips are deliberate and are the point:

- **SU** dips to $3.51/head because a guarantee buys certainty instead of
  upside. A band that expects to sell out should *not* take the SU; a band that
  cannot yet fill a room should, and does not lose money when it doesn't.
- **Arena** dips to $2.25/head because F03's headline economics are a 30%
  business. A big number, a small margin.

**Why each rung has a different economic character** — this is the answer to
target 1, and it is the reason a single `base_payment` column cannot work:

- **Pub** — a landlord's fee out of beer sales, not a door split. Zero margin for
  a full band (F72); the documented band fee is £25–45/night (F69, c.1980–84 —
  deflating at roughly 2× **[labelled estimate]** gives £12–22, and $300/16 =
  £18.75 sits inside that). What it buys: a room, a repeat slot, and nothing
  else.
- **Club** — the first rung where a promoter exists and therefore the first rung
  where a percentage means anything.
- **SU** — a cheque from an institution, signed weeks in advance, with the
  student union carrying the attendance risk. The band trades money for
  certainty and a young audience.
- **Theatre** — real money and the first real cost stack (PA, crew, hall hire).
  F02's split now applies literally.
- **Hall** — the same deal at 2.67× the theatre's gate, and the cost stack reaches
  50%: this is where F03's "£750 spent before the stage" starts to bite, and the
  per-head return only rises 11% for doubling the room.
- **Arena** — a headline fee against a 70% cost base. A big number that is a
  small margin.

---

### Proposed: earning a booking

Replace the fame-only door policy with **two** inputs, so a relationship can
substitute for fame:

```
can_book = venue.prestige <= fame + 20            // today, unchanged
        || venue.standing(you) >= STANDING_MIN    // new
```

`Venue` gains `#[serde(default)] standing: i8` (0 = unknown, −100..100), moved
by: playing the room (+), selling it out (++), cancelling inside a week (−−),
and a bad night in front of a thin house (−). A residency (F05: the Hope and
Anchor, the Nashville, the Tally Ho) is a `standing` contract: N weeks, a
fixed fee, and a hard penalty for a no-show — worth it to a band whose fame
cannot open the next rung yet, and the reason a landlord will keep a Tuesday
free for you.

**Who books you, what they risk, what they take** **[design, shaped by F02/F44/F45]**:

| Rung | Booker | Their risk | Their take |
|------|--------|-----------|------------|
| Pub | Landlord | unsold beer | none on the door; the fee is the cost |
| Club | Local promoter | the hall hire | ~1/3 of the door (F02) |
| SU | Ents secretary | the union's budget | none; the guarantee *is* the deal |
| Theatre | Impresario | the week's rent | ~1/3 + box office (F02) |
| Hall | Regional promoter | the guarantee he paid you | ~1/3 + box office |
| Arena | National promoter | everything | ~1/3 + box office, and he takes it first |

An **agent** (F44: ~10% for an established act, 15–20% for a young band's first
agency — early representation costs *more*) is the mechanism that turns
`standing` into bookings you cannot see: with an agent, a share of the
next-rung venues become offerable at a fame deficit, at 10–20% of the fee. F45's
contradiction is worth preserving rather than resolving: a 1977 handbook says
the agent's 10% is "added on top" and paid by the promoter (F45, low-medium
confidence) while F44 treats it as a commission on the fee. Model it as
**subtracted by default, with a per-agent `buyer_paid: bool`** — the two
historical readings then coexist as different agents, and the player can prefer
one.

---

### Proposed: the support slot

F71 is unambiguous — a support slot was unpaid, "wouldn't be much money", and no
1970s UK support fee, expenses rate or petrol money could be found. The current
$3,000 pay (`turn.rs:158-161`) must go to near zero. But the slot should not
become a dead option, because it is the cheapest way to reach another act's
audience.

Redefine it as **a trade of cash for reach**, with a real cost:

- **Pay: $0.** Petrol money at most: `expenses = weeks × RIG_FUEL_PER_WEEK`
  **[design]**, reimbursed, not profit.
- **Cost:** the weeks are consumed (today they are, via `OngoingActivity`), the
  tour wear applies (`business.rs:352-354`), and the band cannot gig or record in
  those weeks. That is the real price.
- **Payoff, three parts** **[design]**:
  1. `fame_gain` as today (2 + gap/8, clamped 3–12) — but only on completion
     (`business.rs:369-380`), which the code already does correctly.
  2. **A borrowed audience**: a `regional_fame` credit in the host's territory,
     not the band's home scene. This is the mechanic that makes the slot worth
     taking and it does not exist today.
  3. **A standing credit with the host's promoter**, so the slot is a route into
     the next rung he books.
- **The 25% encore roll** (`business.rs:392-397`) stays — it is the right shape
  for "their crowd adopted you", and it is already keyed to the booking week via
  `activity_rng`, so it is deterministic under save/load.
- **The refusal is a choice too**: declining a 4-week unpaid run to keep gigging
  your own circuit should be a defensible line, not a mistake.

Net effect on balance: removes a $3,000 early-game windfall that currently
out-earns a 500-cap show, and returns it as a slower, locational asset.

---

### Proposed: touring economics

v0.7's rigs and length choice are the right frame. Five changes follow from
F03/F04/F59/F60.

**1. A per-show cost stack, not just a per-week rig cost.** F03: 10cc took
~£1,000 a gig and spent ~£750 before the stage — crew wages, hotels, petrol,
vehicles, publicity. That is a **75% cost ratio at headline level**. The stack
should be a rate on gross that rises with the rung, because the fixed costs
(crew, PA, hotels) are what grow:

| Rig | Stack on gross [design] | Rationale |
|-----|------------------------|-----------|
| Van | 30% | four people, one van, borrowed PA |
| Bus | 40% | crew of 3, hotels start |
| Truck & crew | 55% | crew of 8, hall PA, freight |
| Full production | 70% | F03's ratio, approached |

**2. The inversion becomes the choice.** This is the worked tour, and it is the
reason to build the stack.

*2-week theatre tour, 10 shows (5/week, `constants.rs:337`), 55% houses, UK
travel mult 0.8 (`live.rs:440`), stop capacity 2,000 × the rig's mult. This is a
**re-derivation**, and the honest caveat matters: today's `tour_pot` builds the
whole-tour gross from the region's *population* (`live.rs:472-476`), not from a
stop's capacity × ticket, so the gross column below replaces that basis rather
than adjusting it:*

```
per-show gross (van) = 2,000 × 0.8 × 0.55 × $24 × 0.5067 = $10,701
tour gross          = $10,701 × 10                        = $107,008
```

| Rig | Seats/show | Gross | Stack | Rig cost | **Net** | Net if no stack |
|-----|-----------|-------|-------|----------|---------|-----------------|
| Van | 880 | $107,008 | $32,102 | $576 | **$74,330** | $106,432 |
| Bus | 1,100 | $133,760 | $53,504 | $2,218 | **$78,038** | $131,542 |
| Truck & crew | 1,430 | $173,888 | $95,638 | $6,720 | **$71,530** | $167,168 |
| Full production | 1,870 | $227,392 | $159,174 | $19,656 | **$48,562** | $207,736 |

Rig costs: `cost_per_week × weeks × 0.8 × travel_mod × equipment_mod`
(`live.rs:458-465`) = van 360×2×0.8 = $576; bus 1,200×2×0.8×1.1×1.05 = $2,218;
truck 2,800×2×0.8×1.25×1.20 = $6,720; full 6,500×2×0.8×1.4×1.35 = $19,656
(`constants.rs:492`, `data/markets.json` touring_costs).

**The shape of the result is the whole argument.** The optimum is the **bus**,
not the biggest rig: truck already nets **$6,508 less than the bus**, and full
production nets **$29,476 less**, while playing to 1.7× the audience.
Read the last two columns together — *without* a cost stack, full production
beats the van by **$101,304** (and the bus by $76,194) and rig choice collapses
to "buy the biggest you can afford". With the stack it becomes a genuine optimisation with an interior
maximum. That is F03 (£750 of £1,000 spent before the stage) turned into a
decision, and it also preserves F04: the money the big rig fails to earn is
still paid out as `regional_fame` and chart presence, so the loss is a purchase,
not a blunder.

**3. F04 says a losing tour is a rational purchase.** Gallagher & Lyle spent
£15,000 on a 29-person tour and recovered ~£3,000 — 20% — and called it
worthwhile because album sales doubled. In game $ at GIG_SCALE: **$240,000 out,
$48,000 back**. Mechanically this is already possible (the quote shows a
negative net and booking is allowed, `DESIGN-v0.7:44`), but nothing connects the
loss to record sales. Proposal: a completed tour writes `regional_fame` in each
visited territory on the same key that scales chart presence
(`DESIGN-v0.7:73-74` already intends this), so the loss converts into sales
weeks later. Without that link the loss is simply a mistake, which is the
ahistorical part.

**4. F60 is a dated table, not a modifier.** Fuel rose 32.5p/gal (June 1970) →
71.5p (1975 avg) → 79.3p (1977), with a step in **February 1974**. Model the van
rig's cost as `weeks × fuel_price(year) × gallons_per_week` **[design]**, so the
oil shock lands mid-tour as a real squeeze rather than as a smooth era scalar.
The game's era `gig_pay_modifier` already rises 1.0 (1970) → 1.3 (1975)
(`data/timeline.json:40,113`); fuel should rise *faster* through 1974–75 than
the fee does, which is precisely the squeeze the sources describe.

**5. Geography (F59)**: London rehearsal at £4/hr for a 10-piece vs 75p/hr
provincial — a **5.3×** premium. A band based in the capital should pay more per
week to rehearse and have better access to the club/SU rungs; the provincial
band should pay less and travel further. This is the one place where the
existing "start city" choice can be given a live-economics consequence.

---

### Proposed: what goes wrong

Each failure mode is a *choice the player made*, or a risk they accepted, not a
random tax:

| Failure | Trigger | Consequence | Basis |
|---------|---------|-------------|-------|
| **Festival declared free** | mega-festival scale; promoter sees no profit | gate collapses after costs are sunk; fee unpaid or slashed | F06 (Isle of Wight 1970, 600–700k, declared free) |
| **Financier absconds** | festival or big hall, single-backer deal | acts unpaid; the band eats travel and the guarantee never lands | F06 (Bickershaw 1972, collapsed financier, acts unpaid) |
| **Passout resale** | festival gate | sold tickets ≠ gate receipts; £60,000 gate lost money at £2.25 | F06 |
| **Van breakdown** | van rig, long tour, 1974+ | one or two shows lost; the guarantee still has to be honoured or standing falls | F60 (fuel/vehicle cost era) + **[design]** |
| **Venue won't rebook you** | thin house, no-show, or a rough night in a small room | `standing` drops below the gate; the rung closes for a season | F05/F72 + **[design]** |
| **Promoter takes his third first** | any percentage deal | the band's share is what is left after promoter, VAT and box office — never a fraction of a number the band was shown | F02 |

The design rule behind all six: **the band's guarantee is a promise someone else
has to keep.** Festivals are high-variance *because* the counterparty is weakest
there (F06) — which makes the festival a genuine gamble rather than a big gig.

---

### Proposed: reading a gig

A gig currently produces one number the player can act on (`earnings`) plus a
verdict label (`live.rs:299-318`). The evidence says a night had several
separable readings, and the player should learn different things from each:

| Reading | Source now | What it should teach |
|---------|-----------|----------------------|
| **Attendance** | `attendance_ratio`, driven by `fame vs prestige` and reception (`live.rs:288-302`) | whether the *booking* was right — a half-full room at your own rung is a signal about the room, not the band |
| **Reception / verdict** | `compute_reception`, thresholds 40/70/85 (`shows.rs:23-35`, `constants.rs:360-362`) | whether the *performance* was right; rough → solid → great → transcendent |
| **Live reputation** | `apply_show_verdict_rewards` (`live.rs:210-217`) | the slow asset; every night but a rough one nudges it |
| **Standing with the room** | **missing** | whether you can come back |

Add one learning output on a **rough** night, and only on a rough night: name the
weakest member (`average_member_skill`, `band.rs:287`) so the player knows who to
rehearse. That is the cheapest possible "what you learn from a bad night" and it
needs no new simulation.

Two calibration notes on the existing engine, since they bear on "what makes a
gig good":

- `reception_attendance_factor` spans only **0.85–1.15**
  (`constants.rs:377-378`) — i.e. ±15% around a solid night, so a transcendent
  night sells **1.35×** what a rough one does (1.15/0.85). F01's whole premise — that the room you can fill is the ladder —
  suggests this lever is far too weak; a great band in a 500-room and a bad band
  in the same room should not be 15% apart. **[design: widen to 0.55–1.25, so a
  bad night visibly empties a room]**
- At fame 0 against a prestige-10 pub, `base_ratio` = 10/20 = **0.5**, so the
  first gig plays to a **42–58%** house (0.5 × the 0.85–1.15 reception factor)
  and pays `$300 × attendance_ratio × market_modifier × era_modifier`. A fresh
  world opens at demand 50, saturation 55, `Growing` (`world/mod.rs:105-108`),
  which by `world/mod.rs:228-238` is 0.5 × (1 − 55/200) × 1.1 = **0.40** — so
  the first night pays **≈$50–70**, i.e. **£3–4.50 for the whole band**.
  That is a good opening beat (F72's zero margin, F70's round-of-drinks floor)
  and should be preserved, not fixed.

---

### Determinism, serde and balance

**Determinism.** No new `thread_rng`. Failure rolls (van breakdown, promoter
abscond, festival declared free) use `activity_rng(booking_week, step)`
(`src/game/rng.rs:94-96`) — the same keyed stream that already makes a
mid-tour save reload identically, so a risk taken in week *n* resolves the same
way whether or not the player reloads. All table lookups are index-aligned
arrays or `BTreeMap`, never hash iteration.

**Serde.** No renames. `base_payment` stays and keeps its meaning for old saves;
new fields carry defaults so a pre-change save deserializes and behaves exactly
as today until the migration runs:

```rust
#[derive(Serialize, Deserialize)]
pub struct Venue {
    pub name: String,          // unchanged
    pub capacity: u32,         // unchanged
    pub prestige: u8,          // unchanged
    pub base_payment: u32,     // unchanged — the flat fee / legacy fallback
    pub location: String,      // unchanged
    #[serde(default)] pub ticket_pence: u16,   // 0 = legacy, use base_payment
    #[serde(default)] pub deal: DealKind,      // #[default] FlatFee
    #[serde(default)] pub standing: i8,        // 0 = unknown
}

#[derive(Serialize, Deserialize, Default, Clone, Copy)]
pub enum DealKind {
    #[default] FlatFee,        // legacy behaviour: pay base_payment × ratio
    FeePlusPercentage,
    Guarantee,
    Percentage,
}
```

`#[serde(rename = ...)]` must not be used anywhere in this change; the variant
names above are new so no `alias` is needed.

**Balance — and the honest risk.** Live income at the top of the weekly ladder
moves a lot. Against today:

| Rung | Today (gross) | Proposed (net) | Δ |
|------|--------------|----------------|---|
| Pub 50 | $300 | $300 | **0%** |
| Club 200 | $900 | $973 | +8% |
| SU 500 | $2,400 | $1,755 | −27% |
| Theatre 2,000 | $2,400 | $14,592 | **+508%** |
| Hall 4,000 | *new rung* | $32,427 | — |
| Arena (one-off) | $45,000 | $45,000 | 0% |

Four of the six rungs land within ±27% of today (pub, club, SU, arena). The SU falls 27% on purpose —
today's $2,400 at a 500-cap is already a generous guarantee, and the guarantee is
now explicitly a discount on upside.

The theatre's +508% is not a tuning slip; it is the size of the error. A
sold-out 2,000-seat theatre at £1.50 is a £3,000 gate, and $2,400 is **5%** of it — while the *stack-based* net is $14,592, 30% of it.
That gap cannot be absorbed by a cost stack — a 40% stack still leaves 6× — so
**this is a re-baseline, not a patch**, and it must be staged:

1. **Stage 1 (net-neutral):** `DealKind`, `ticket_pence`, `standing`, the
   per-show cost stack, the support-slot pay cut, and the widened attendance
   factor — all of them applied to the *existing* five rungs without
   re-pricing them. Each makes the ladder more decision-dense while leaving
   every rung's net roughly where it is (the stack trims a few percent; the
   support cut removes one early windfall). This is the stage that does not
   require re-deriving any bot baseline.
2. **Stage 2 (re-baseline):** the corrected ticket ladder. This *will* move the
   `balanced-indie` and `road-dog` baselines, and `docs/DESIGN-v0.7:414`
   ("median signed act recoups before the term's halfway mark") plus
   `src/game/tests/fame.rs` ceilings must be re-derived, not merely re-run.
3. **Guard:** `sim.rs:272-277`'s `biggest_open_venue` picks the largest
   affordable room; once the theatre pays 6× the club, every bot will sprint to
   prestige 70 and skip the middle rungs. Bots should be given a
   **risk-adjusted** chooser (expected net after stack and after the booking
   gate), or the ladder will be observed only at its top in every sim run.

The one number I would **not** change is the pub's $300: it is the only rung
whose payment is already the right *kind* of money, and it anchors F69's
£25–45 band fee inside its deflated 1974 range.

---

### Grounding

**Historical (F-ids, `findings.csv`).** F01 ticket tiers 50p / £2 / £3.50 (1974);
F02 promoter ~1/3 + 8% VAT + box office before the band's share; F03 10cc
£1,000 fee against £750 pre-stage costs; F04 £15,000 tour returning £3,000,
worthwhile on doubled album sales; F05 London pub circuit from 1973 and the
residency; F06 festival fragility — declared free, collapsed financier, acts
unpaid, £60,000 gate at £2.25 losing money; F41 manager 20% of gross; F43
manager-owned publisher/label; F44 agent 10% established vs 15–20% first agency;
F45 agent's 10% "added on top", buyer-paid; F59 London rehearsal £4/hr vs 75p/hr
provincial (5.3×); F60 fuel 32.5p → 71.5p → 79.3p/gal with a February 1974 step;
F69 £25–45 per band per night, **c.1980–84 only, deflated here as a labelled
estimate**; F70 25s solo club floor; F71 support slots unpaid, no term found;
F72 pub tier zero-margin.

**Inherited and not re-derived.** The per-head figures ($6.00 / $4.50 / $4.80 /
$4.50 / $2.25), the gross-credit finding (`live.rs:303-327`) and the 2.67× fall
are the sibling gap analysis's results, cited here from `venues.rs:27,34` and
`live.rs:303-327`.

**Invented (labelled [design]).** GIG_SCALE = 16; the 8% box-office rate; the
75p / £1.00 / £1.50 / £2.00 intermediate ticket prices; all five cost-stack
percentages (30/40/55/70 and the per-rung 0/20/10/40/50); the $150,000 arena
gross cap and its 30% margin; the hall rung at 4,000 and the arena's move to a
one-off; `standing` and its thresholds; `DealKind` and `FeePlusPercentage`; the
agent's `buyer_paid` flag; fuel `gallons_per_week`; the 0.55–1.25 attendance
band; the ensemble of failure modes and their triggers.

**Contradictions left standing, not reconciled.** F02 names box-office
commission without a rate (hence a design dial). F44 and F45 disagree on *who
pays* the agent (hence a per-agent flag, both modelled). F69's window is
c.1980–84 and is used only after an explicit deflation, never backdated
silently.
