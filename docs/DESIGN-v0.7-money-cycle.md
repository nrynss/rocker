# Design — The Money Cycle (v0.7)

> Tours that quote, a roof over your head, charts that breathe, records
> that certify, pressing and distribution that cost real money. This
> document is the **decided design** for the v0.7 feature cycle, agreed in
> discussion on 2026-07-15. The cycle's task board is `HANDOFF.md` at the
> repo root. FUTURE.md (Musician, relationships, solo/band) stays the
> north star for a later cycle; this cycle touches no quality formulas and
> none of the Musician data model.
>
> **Deferred explicitly:** addiction (FUTURE §9.1), vacations picker
> (§9.3), and the Manager (§9.4) remain unbuilt and dormant. Do not
> implement them here; do not remove the dormant fields.

Numbers marked **[tune]** are starting values to be validated in the
`sim.rs` balance lab (task M7), not contract. Everything else is decided
design.

Baseline at cycle start: **0.6.0**, `cargo test` → **102 passed,
4 ignored**, clippy clean, fmt clean. Cycle closes as **0.7.0**.

---

## §A — Tour economics: the quote, not the ambush

### The problem in the current code

`action_go_on_tour` (`src/game/actions/live.rs`) picks a cost tier from
the **player's fame bracket** — `<35` local, `<60` regional, `<80`
national, `≥80` international — and charges that tier's
`base_cost_per_show` (500 / 1,500 / 5,000 / 15,000 from `markets.json`)
for the *same region you toured last month*. Crossing fame 60 makes an
identical tour cost 3.3× more overnight, with no explanation and no
choice. Three more lies on top:

- `base_cost_per_show` is charged **once per tour**, not per show.
- `travel_cost_modifier` and `equipment_cost_modifier` are loaded
  (`src/data_loader.rs:141`) and **never read**.
- The player sees the cost only in the error message when they can't
  afford it.

### The fix: the rig is a choice, the cost is a quote

**Fame never selects a cost tier again.** Same region + same rig = same
cost, at any fame. Fame gates which rigs you *may* book and how many
seats you fill — that's it.

**Tour rigs** (picker, venue-picker pattern):

| Rig | Fame gate | Cost / tour week [tune] | Capacity mult [tune] | Wear / week [tune] |
|-----|-----------|------------------------|----------------------|--------------------|
| Van tour | — | $150 | 0.8× | health −5, stress +9 |
| Tour bus | 25 | $600 | 1.0× | health −4, stress +8 |
| Truck & crew | 55 | $2,500 | 1.3× | health −3, stress +6 |
| Full production | 75 | $8,000 | 1.7× | health −2, stress +5 |

- Capacity mult scales `synth_tour_venue_capacity` — a bigger rig books
  bigger rooms, raising the gross ceiling.
- Wear replaces the flat `TOUR_HEALTH_COST_PER_WEEK` /
  `TOUR_STRESS_COST_PER_WEEK`: the van grinds you down, the production
  rig has roadies.
- Total cost = `rig cost × tour weeks × country travel mult ×
  region travel_cost_modifier` — the dead `markets.json` modifiers
  finally do their job (repurposed as per-tier travel scaling on the
  **rig**, not the fame bracket; the old `touring_costs` fame-tier keys
  are re-keyed to rigs in `markets.json`).
- **Tour length is a choice too.** The picker offers 1–4 weeks [tune:
  3 weeks gated at fame 40, 4 at fame 60]; today's fame-derived length
  is deleted with the fame-derived cost tier. Cost scales linearly
  with weeks; fame and regional-fame gains scale sublinearly [tune] —
  a long tour is a bigger investment, not a strictly better one.
  Region × rig × length is the full booking matrix: many more ways to
  tour, each a quoted decision.
- **The quote comes first.** The tour picker shows, before booking:
  itemized cost, weeks, shows, and a projected gross range (computed
  from the same formula the tour uses, at momentum 1.0, ±the reception
  spread). Booking a money-losing tour is allowed — it buys fame and
  regional fame — but it is a *decision*, never a surprise.
- **Tours feed the charts.** Regional fame earned on the road is the
  same number that scales your presence on that territory's chart
  (§C) — touring America is how a record starts selling, and
  charting, in America.

Existing per-show engine, momentum, regional fame, and the whole-tour
pot formula are untouched apart from the capacity multiplier, the wear
table, and the length choice.

---

## §B — Lifestyle: a roof over your head

The original 1989 game charged you rent every week and made where you
lived part of who you were. Restored.

### The tiers

`LifestyleTier` on `Player` (`#[serde(default)]` → `Squat`):

| Tier | Upkeep / week [tune] | Stress release bonus [tune] | Happiness floor [tune] | Rest healing bonus [tune] |
|------|---------------------|------------------------------|------------------------|---------------------------|
| Squat | $0 | +0 | 0 | +0 |
| Shared flat | $40 | +1 | 5 | +1 |
| City apartment | $180 | +2 | 10 | +2 |
| Townhouse | $700 | +3 | 15 | +3 |
| Mansion | $2,800 | +4 | 20 | +4 |

- **Upkeep** is deducted in the weekly tick (`lifestyle.rs` — the module
  finally earns its name).
- **Stress release bonus** adds to `STRESS_PASSIVE_RELEASE`.
- **Happiness floor**: the weekly stress drain cannot pull happiness
  below the floor (event/incident losses still can).
- **Rest healing bonus** adds to the health/stress recovery of rest-type
  actions (`LazeAround`, `TakeBreak`).

### Image

- Fame ≥ 60 while living at Squat or Shared flat: the tabloids run
  photos. Happiness −2/week [tune], one news line the first week.
- Low fame in a Mansion: no penalty. Rock'n'roll excess is allowed;
  the rent is the penalty.

### Moving — always the player's call

- Tier changes happen **only** through `GameAction::ChangeLifestyle(tier)`
  — instant (no week consumed), via a picker modal. The game never
  moves you up, and never moves you down except the one forced case
  below.
- Moving **up**: deposit of 4 weeks' upkeep [tune] on top of the first
  week — and a one-shot happiness **+10** [tune]. New digs feel like
  the career is going somewhere.
- Moving **down**: free, but a one-shot happiness **−15** [tune].
  Giving up the house hurts, even when it's the smart move.
- **Broke eviction** — the only involuntary move: money < 0 for 2
  consecutive weeks → down one tier, happiness **−20** [tune], news
  line. Volunteering the downgrade before the landlord decides is
  strictly gentler.

---

## §C — Charts that breathe

### The problems in the current code (`src/game/world/charts.rs`, `world/scene.rs`)

1. `truncate(CHART_SIZE)` **hard-deletes** anything pushed below #10 —
   a record scoring 3× the floor vanishes because one hot week produced
   eleven better scores. No re-entry, no slide down the chart.
2. The scene floods the board: ~180 bands rolling 1-in-16 (signed) /
   1-in-28 (unsigned) release odds ≈ **8–10 submissions per week** for
   10 slots. The whole chart can turn over weekly.
3. Flat 0.85 decay: even an untouched #1 falls below the floor in ≤8
   weeks. And nothing ever *climbs* — every record enters at its peak.

### The fix: regional Top 100s

- **Four sales territories + a scene board + an aggregate.** The home
  scene is a UK city; the charts are:
  - **Local** — the home scene's board. A *subset view* of the UK
    market, not a territory: every scene band competes here, the
    player always enters, and its sales are **already UK sales** —
    Local never adds into Worldwide and never adds demand of its own.
    The grassroots prestige board where careers start.
  - **UK, Europe, America, Japan** — the four **sales territories**,
    each stored, decayed, and competed independently at **depth 100**.
    Japan earns its slot: the second-largest record market of the era,
    already a tour destination in `markets.json`. Australia stays
    tour-only (fame and gross, no board).
  - **Worldwide** — *derived*: the same release's effective scores
    summed across **the four territories only**, re-ranked, top 100.
    Never stored, never decayed on its own — pure aggregation,
    recomputed after the weekly decay pass.
- **Data model.** New `ChartRegion` enum and
  `regional_charts: BTreeMap<ChartRegion, Vec<ChartEntry>>` on
  `GameWorld` (`#[serde(default)]`; BTreeMap for deterministic
  iteration). The legacy `charts` field stays serialized — on first
  load of an old save it seeds the Local board, then stays empty.
  New module `src/game/world/regions.rs` owns the enum and presence
  computation.
- **Presence gates entry.** A release submits to each board at
  `score × presence(region)` [tune], dropping entries below the floor:
  - **Player:** Local is always home turf (presence 1.0). **UK gets a
    home floor** — 0.1 [tune] even for a mail-order act, because local
    sales *are* UK sales. Beyond that, presence comes from the
    distribution channel (§E-3: regional distributor 0.3, national
    0.5) or the label's `market_reach` when signed — multiplied by
    country-aggregated `regional_fame`. Tours literally carry your
    records abroad (§A).
  - **Scene bands:** unsigned acts chart Local (UK spillover at fame
    ≥ 60); signed acts spread by label tier — Boutique UK, Independent
    UK + 1 territory, Major all four [tune].
- **Territory filler.** Four Top-100 boards can't be fed by one city's
  scene. Each territory gains 4–6 ambient releases per week [tune] —
  name-generated foreign acts as **chart-only entries** (no band
  state, no scene simulation), scored on the scene-release scale and
  seeded from the world RNG. America's chart is full of American
  bands you'll never meet; the scene stays 180 bands.
- **Ramp-in — records climb.** `ChartEntry` gains `base_score: u32`
  and `peak_position: u8` (`#[serde(default)]`). Effective score =
  `base_score × ramp × decay`, where ramp is ×0.6 entry week, ×0.85
  week 1, ×1.0 from week 2 [tune]; decay (`0.92` [tune], floor 25)
  applies from week 2. A strong release debuts mid-chart, climbs for
  two weeks, peaks, then slides. Lifecycle stays **pure score** per
  region — eviction only below #100, no special cases, no `is_player`
  favoritism.
- **Calmer scene:** release odds 1-in-26 signed / 1-in-44 unsigned
  [tune] → ~4–5 releases/week. Across four boards at depth 100 that
  fills charts without churning them; scene fame/momentum rewards
  unchanged.
- **UI:** the charts modal gets region tabs (`←/→` to switch, Local →
  UK → Europe → America → Japan → Worldwide), top 10 at a glance,
  scroll to 100. Movement arrows, peak, weeks-on-chart. Player news
  lines name the region ("📈 '{title}' ↑ #12 → #7 UK").

The determinism tests must pass unmodified.

### Regional sales — copies scale with presence

Demand follows the same presence model: `calculate_release_outcome`'s
single global multiplication becomes a **sum over the four sales
territories** (UK, Europe, America, Japan — Local is inside UK and
adds nothing) — `Σ score × presence(territory) ×
UNITS_PER_SCORE_POINT` — capped by the pressing as today.
`copies_sold` stays one global number; only how it accumulates
changes. An act present on all four territories moves roughly 3–4×
the copies of a home-market act at the same score — which is why the
certification thresholds in §D scale up with it.

---

## §D — Certifications: silver, gold, platinum

`Release` already tracks cumulative `copies_sold` (first run + long
tail). Certifications derive from it — **units only**, no other input.

Thresholds are sized for the regional sales model (§C — an act on all
four boards moves 3–4× a Local-only act, so the bar sits accordingly
higher):

| Award | Copies sold [tune] |
|-------|--------------------|
| Silver | 50,000 |
| Gold | 150,000 |
| Platinum | 400,000 |
| Multi-platinum | each further 400,000 (×2, ×3, …) |

- `certified: u8` on `Release` (`#[serde(default)]`): 0 none, 1 silver,
  2 gold, 3 platinum, 4+ multi-platinum count.
- Checked in the weekly catalog pass (`economy.rs`), one-shot per level.
- Award moment: news line ("🏆 'Neon Nights' is certified GOLD —
  40,000 copies."), fame +2 / +4 / +6 (capped by existing rules),
  happiness +5 / +8 / +12, `reputation.commercial_success` +3 / +5 / +8
  [tune] for silver/gold/platinum (multi repeats platinum's bump).
- UI: badge in the discography modal (`ui/render/modals/file.rs`):
  🥈 / 🥇 / 💠×N.

Thresholds are calibrated in the sim lab (M7) so a median 15-year career
lands 1–3 silvers, a genuine hit goes gold off its tail, and platinum is
legend material.

---

## §E — Pressing & distribution with real costs

Indie pressing already costs money (`PRESSING_TIERS`, `pressing_cost`).
Five gaps remain — three in the money pipeline, two in the contract
itself:

### E-1. Re-pressing (sold out is not a dead end)

Today a sold-out run logs "demand was there for more" and nothing can be
done. New: `GameAction::RePress(release, tier)` — pick any released
record with `copies_sold == copies_pressed` (or low stock), choose a
pressing tier, pay `pressing_cost`, `copies_pressed += run`. Instant
(no week). Signed acts don't choose: the **label auto-represses** when a
release sells out or certifies (news line; cost recouped, see E-2).

### E-2. Label recoupment (the machine bills you first)

The label's pressing and promo are currently free money — and so is the
**advance**: `action_sign_deal` (`business.rs:94`) banks it at signing
and nothing ever claws it back. New: `RecordDeal` gains
`unrecouped: i32` (`#[serde(default)]`). The **advance joins
`unrecouped` at signing**; at each release the label's outlay —
pressing (`label_pressing_size` × per-copy cost [tune: $0.30/copy])
plus promo (promo push × $15 [tune]) — is added on top. Royalty income
pays it down **before** reaching the player; while `unrecouped > 0` the
weekly log shows "⚖️ Label recouping: $X remaining." Recoupment
survives the deal: catalog released under the deal keeps paying at deal
terms, and keeps paying the balance down, until it's cleared — the
classic hit-record-still-broke story, and the honest price of the
advance and `market_reach`. Sim lab (M7) validates a signed mid-tier
act still nets more than indie on equivalent records over a full deal —
worth signing, just no longer free.

### E-3. Indie distribution tiers (reach you can buy)

Unsigned reach is currently `0.15 + fame-scaled` — implicit and
unpurchasable. New: releasing while unsigned offers a distribution
choice alongside the pressing picker:

| Channel | Fame gate | Fee / release [tune] | Reach floor [tune] |
|---------|-----------|----------------------|--------------------|
| Mail order & gigs | — | $0 | 0.15 (current formula) |
| Regional distributor | — | $400 | 0.30 |
| National distributor | 35 | $1,500 | 0.50 |

Effective reach = `max(channel floor, current indie formula)`. Fee due
at release. A label deal still beats all of it on reach — but the indie
path now has purchasable rungs.

### E-4. The contract has a clock

Today `fulfill_album_obligation` (`band.rs:240`) clears the deal the
instant `albums_delivered >= albums_required` — and Independent and
Boutique deals can require **one album**, so a band can sign, bank the
advance, ship a single album, and walk free the same week. No real
contract works like that.

New fields on `RecordDeal` (all `#[serde(default)]`): `signed_week: u32`
and `term_weeks: u16`, stamped at signing. Term by tier [tune]:

| Tier | Albums (unchanged) | Term |
|------|--------------------|------|
| Boutique | 1–2 | 52–78 weeks |
| Independent | 1–3 | 78–104 weeks |
| Major | 2–4 | 104–156 weeks |

- **Free agency comes at whichever is later**: all albums delivered
  **and** the term served. Deliver early and you stay on the roster —
  releases still go through the label at deal terms, single-cuts and
  recoupment continue — with a news line: "🤝 Obligation delivered —
  under contract with {label} until {date}."
- **Breach**: term expires with albums still owed → the label drops
  you. `reputation.commercial_success` −10 [tune], any `unrecouped`
  balance is written off with a second news line (they remember), and
  `deal_cooldown: u16` on `Band` (`#[serde(default)]`, 26 weeks [tune])
  blocks new offers — same field name FUTURE §3 plans around, so the
  Musician cycle inherits it.
- **The renewal window**: 26 weeks [tune] before expiry — all albums
  delivered — the label looks at its ledger and makes a move through
  the existing offer stream (normal expiry beat; declining is free,
  you play out the term to free agency). What it offers depends on
  whether it made its money back:
  - **Recouped, decent sales** → a **new contract**: fresh albums +
    term at the label's current terms with +2–4pp royalty [tune] and a
    new advance (which joins the new ledger, per E-2). Signing replaces
    the old deal; the remaining term dissolves into the new one.
  - **Not yet recouped** → an **extension**: +1 album, +52 weeks
    [tune], a small advance, royalty unchanged, and the unrecouped
    balance carries forward — the label protecting its investment,
    not rewarding you.
  - **Deep in the red with weak sales** → no offer. They let the
    clock run out and write you off.
  - Nothing re-signs automatically. Loyalty pays only if you take it;
    riding out the term to shop the open market is always allowed
    (recoupment still survives the deal, per E-2).

### E-5. The label's active hand

The label spent money on you (E-2) and holds your time (E-4) — it acts
like it. All rolls on the existing action stream; the v0.6 single-cut
machinery (`label_moves.rs`) is the enforcement arm.

- **Recoup pressure**: while `unrecouped > 0`, the single-cut chance
  doubles [tune] and its idle-weeks gate drops 3 → 2 — a label in the
  red gets antsy about product.
- **Label memos** — the label *asks* before it *takes*. Weekly checks
  while signed, each a news-log line, ~25% roll when its condition
  holds [tune], one memo max per week:
  - No unreleased songs and no album progress for 4+ weeks:
    "📠 {label}: 'We need songs on tape. Write.'"
  - Unreleased songs sitting idle 4+ weeks: "📠 {label}: 'Cut a single
    from that material — this week, ideally.'" Ignored for 4 more weeks
    with a cuttable album available → the existing single-cut fires at
    boosted odds (they stop asking).
  - Inside the final 12 weeks of the term with albums still owed:
    "📠 {label}: 'The contract says {n} more album{s}. The clock says
    {weeks} weeks.'" — and stress +3/week [tune] while this holds. The
    deadline is real pressure, not flavor.
- Memos are information, not compulsion: the player can ignore
  everything and eat the breach. The game never forces the action.

---

## §F — Sim lab, bots, and tests (M7)

New bots in `sim.rs`:

- **homebody** — never tours, matches lifestyle tier to income; must
  survive 15 years without bankruptcy (lifestyle upkeep can't be a
  death spiral).
- **road-dog** — tours constantly on the biggest affordable rig; must
  not trivially out-earn the release-focused grinders.
- **indie-lifer** — never signs, buys distribution tiers; must be
  viable, slower than signed peers, and ahead of the old 0.15-floor
  indie baseline.

Measured targets [tune until they hold]:

- Chart half-life of a 300-score entry: 6–10 weeks inside a regional
  top 10.
- A Worldwide top-10 requires presence in at least three territories;
  a home-market act (mail-order, no tours abroad) can top the Local
  board but never crack Worldwide's top 20.
- Certifications per median 15-year career: 1–3 silver, gold on hits —
  under the scaled thresholds and territory sales. A home-market act
  (UK floor only) must still be able to reach Silver.
- Van tour profitable at fame 15–35; full production profitable only
  at fame 75+.
- Matched lifestyle upkeep: 10–25% of weekly income.
- Recouped label act nets ≥ indie equivalent over a full deal term.
- A steadily-releasing bot never breaches; a bot that signs and then
  only tours breaches and eats the penalty (the clock must have teeth,
  but only for the negligent).
- Median signed act recoups the advance before the term's halfway mark.

Save-compat: every new field `#[serde(default)]`; the v0.4 fixture and
`saves_from_v0_4_still_load` stay sacred; the three determinism tests
pass **unmodified**.

---

## Key design constraints — do not violate

- **Same region + same rig = same cost, at any fame.** Fame gates rigs
  and fills seats; it never re-prices a tour.
- **The quote precedes the booking.** No cost the player first learns
  about in the outcome log.
- **Charts are pure score lifecycle, per board, at depth 100.** No
  special eviction, no player favoritism, determinism tests unmodified.
  Worldwide is derived from the four sales territories only — **Local
  is a UK subset and never double-counts** into Worldwide or demand.
- **Lifestyle moves are the player's call.** Broke eviction is the only
  forced move; up feels good, down hurts, always one-shot.
- **Certifications derive from `copies_sold` only.**
- **A contract binds both directions: albums owed AND a term served.**
  Free agency at whichever comes later, never on the release beat.
- **The label asks before it takes.** Memos precede single-cuts;
  nothing the label does is ever hidden from the news log.
- **Serde:** every new field `#[serde(default)]`; never rename
  serialized fields (`energy`, addiction fields stay dormant).
- **World RNG is injected, not ambient.**
- **The Musician plan (FUTURE §1–§8) is untouched**, and §9.1/§9.3/§9.4
  stay deferred.
