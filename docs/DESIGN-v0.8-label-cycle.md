# Design — The Label Cycle (v0.8)

> Record companies and big bands made gangbuster money. The game never
> modelled the half of the industry where that money actually lived — the
> label's wholesale ledger — so records "barely sold 50k", advances never
> recouped, and a Gold album made a fortune for *nobody*. This cycle makes
> the labels **living economic entities**: a balance sheet that rises on
> hits and falls on flops, a roster of real acts, reach and tier that grow
> and decline, competition and bidding for talent, mergers and failures,
> and — underneath all of it — a money scale where a superstar and their
> label both earn what the era actually paid.
>
> This is the **decided design** for the v0.8 feature cycle, opened
> 2026-07-24. The two flagged forks are settled (coordinator, 2026-07-24):
> **§B — one honest money scale** (not dual-scale), and **§F — label
> founding starts off** (sim may flip it). Remaining [tune] numbers are
> validated in L-SIM, not contract. The cycle's task board is `HANDOFF.md`
> at the repo root. FUTURE.md (Musician, relationships, solo/band) stays
> the north star for a later cycle.
>
> **Builds on:** the record-sales fix (branch `fix/balance-major-label-
> international`, folded in as this cycle's foundation) that gave a label's
> `market_reach` real international distribution. That fix makes reach
> *matter*; this cycle makes reach *move*.
>
> **Deferred explicitly (unchanged):** addiction (FUTURE §9.1), vacations
> picker (§9.3), the Manager (§9.4), and the whole Musician plan
> (FUTURE §1–§8) stay dormant. Do not implement them here; do not remove
> the dormant fields.

Numbers marked **[tune]** are starting values to be validated in the
`sim.rs` balance lab (task L-SIM), not contract. Everything else is
decided design pending sign-off.

Baseline at cycle start: **0.7.1** + the sales fix, `cargo test` →
**188 passed, 5 ignored**, clippy `-D warnings` clean, fmt clean,
determinism trio unmodified. Cycle closes as **0.8.0**.

---

## The shape of the problem

Today a label is inert data in `data/record_labels.json` — a name, a
`market_reach`, a `royalty_rate`, an `advance_range`, some specialty
genres. Signing snapshots a few of those onto a `RecordDeal` and that is
the end of the label's existence. It has no money, no memory, no roster,
no future. Three consequences:

1. **The gangbuster money goes nowhere.** When a record sells, cash
   reaches the player (royalty) or vanishes into the recoupment ledger
   (a debt counter that decrements and pays *no one*). The label's actual
   business — the ~$2–3/copy wholesale margin it clears on **every unit,
   recouped or not** — is not modelled. On a Gold record (150k copies) the
   label historically cleared ~$300–430k; in-game it books $0.
2. **Reach is frozen.** `market_reach` never changes, so a boutique can
   never grow into a major and a major can never fall — the single number
   that (post sales-fix) decides where your records sell is a constant.
3. **The world is dead around you.** Scene bands never sign, labels never
   compete, nothing rises or fails. The industry the player is trying to
   climb has no life of its own.

This cycle fixes all three by giving labels a balance sheet, a roster,
and a lifecycle — and by lifting the money scale so the numbers on that
balance sheet are the era's real numbers.

---

## §A — The label balance sheet (capital that goes up and down)

The core new entity. `Label` becomes **world state**, instantiated at
worldgen from `record_labels.json` (the JSON becomes seed data, not a
runtime lookup table).

```rust
// src/game/world/labels.rs (new module)
pub struct Label {
    pub id: u32,
    pub name: String,
    pub tier: LabelTier,              // Boutique | Independent | Major (now dynamic, §C)
    pub genres: Vec<MusicGenre>,      // specialty (from JSON)
    pub founded_year: u16,
    pub capital: i64,                 // the balance sheet — the "up and down"
    pub market_reach: u8,             // 0–100, now DYNAMIC (§C)
    pub financial_power: u8,          // 0–100, capacity to front advances
    pub artist_development: u8,       // 0–100, A&R patience/quality (§G)
    pub creative_freedom: u8,         // 0–100, personality (§G)
    pub strategy: LabelStrategy,      // §G
    pub status: LabelStatus,          // Active | Distressed | Folding | Acquired
    pub roster: Vec<RosterSlot>,      // player + scene bands (§D)
    pub hit_window: Vec<u32>,         // release ids that charted, rolling (§C)
    pub ledger_ytd: i64,              // profit/loss this game-year, for news + tier drift
}
```

`GameWorld` gains `labels: BTreeMap<u32, Label>` (`#[serde(default)]`;
BTreeMap for deterministic iteration). A new `world/labels.rs` owns the
struct, worldgen seeding, and the weekly label tick.

### The wholesale ledger — where the money is made

Every weekly sales pass, for **every copy sold by any roster act**
(player *and* scene bands, §D), the label books:

```
label_margin_per_copy = wholesale_price
                      − manufacturing_cost      (the run it already paid for)
                      − artist_royalty_per_copy  (what it owes the act)
                      − mechanical_royalty        (songwriters/publishers)
```

`capital += copies_sold × label_margin_per_copy`. Advances (§E),
pressing runs, and promo push (the existing `apply_label_promo` outlay)
are debits to `capital` at the moment they are spent — the same events
that already accrue to the deal's `unrecouped` ledger now also move the
label's real money. **Recoupment stops being a void:** the player's
royalty, while unrecouped, is retained by the label and shows on its
ledger as retained margin, exactly the historical mechanic.

This is the section that makes "gangbuster money" literal and visible:
the player can watch a label clear a fortune off a record the player has
not yet recouped a cent on — the Albini asymmetry as a live number.

Per-copy constants (`manufacturing`, `wholesale`, mechanical) are
**[tune]**, sized against §B's scale so a Gold record nets the label a
low-six-to-seven-figure margin and a Platinum act's catalogue makes it
millions.

---

## §B — The scale: era-real money

The v0.7 economy is deliberately **compressed**: `UNITS_PER_SCORE_POINT`
(30) inflates copies for certification while `SALES_INCOME_DIVISOR` (3)
divides the dollars back down, so a superstar's royalty tops out in the
low thousands and a Gold record nets the band ~$18k pre-recoup. Labels
making gangbuster money requires the magnitude to be era-real, for the
band **and** the label.

**Decided approach:** decompress the money economy to a "$1 in-game ≈ $1
of the era" scale, moving every coupled dial *together* so the v0.7
balance the sim already validates is preserved in shape while the
magnitude lifts:

- Retire `SALES_INCOME_DIVISOR`; set `INDIE_INCOME_PER_COPY` /
  `LABEL_INCOME_PER_COPY` to era-real per-copy artist royalties [tune].
- Scale player-facing costs — lifestyle upkeep, tour rig costs, recording
  and pressing bills, advances in `record_labels.json` — up in lockstep
  [tune], so affordability ratios (the §F targets: matched lifestyle upkeep
  10–25% of income; van profitable at fame 15–35) are preserved even as
  the absolute numbers grow.
- Certification thresholds (§D of v0.7: 50k/150k/400k) are **copy** counts,
  not dollars — untouched by the dollar rescale. (Their *era-accuracy* is a
  separate matter tracked in §I / L-CERT below.)

This is the **riskiest single task** in the cycle: it touches every dollar
in the game and must be re-validated end-to-end in the sim lab (no bot
bankrupted by the rescale, no bot trivially rich, all §F ratios intact).
It is sequenced **first** (L-SCALE, below) so every later section books
its money at the final scale.

> **Decided 2026-07-24 (coordinator): one honest scale.** The dual-scale
> alternative (compressed player wallet + a separate "industry" scale for
> label capital) is rejected — the player's own earnings should reach
> era-real "gangbuster" magnitude too, not just the labels'. L-SCALE
> therefore decompresses the single shared economy end-to-end.
>
> **Method (measure, then rescale):** the copy counts are already at the
> real certification scale (50k Silver …) — only the *dollars per copy*
> are compressed by `SALES_INCOME_DIVISOR`. So L-SCALE (a) retires the
> divisor (income rises to meet the already-real copy counts) and rebalances
> the divisor-coupled cost literals, then (b) rescales the *independent*
> player-cost constants (lifestyle, tour, recording, pressing setup,
> marketing, advances, tour box-office) so every §F affordability ratio
> holds. Step (b) is driven by the sim's §F ratio readout after step (a),
> not guessed — the sim says which costs moved out of band, and by how much.

---

## §C — Reach and tier that rise and fall

`market_reach` and `tier` stop being constants and **drift toward a target
derived from the label's health**, so a boutique with a breakout act
climbs and a major that loses its stars declines. Because the sales fix
made reach drive international distribution, a label's rise *literally*
widens where its acts sell — and its fall contracts it.

- **Target reach** = f(`capital`, `hit_window`) — a label flush with cash
  and recent charting records trends up; one bleeding money with a cold
  roster trends down. [tune] curve, bounded 0–100.
- **Drift, not teleport:** reach moves at most `REACH_DRIFT_PER_YEAR`
  [tune] points toward target, with hysteresis, so it reads as a trajectory
  in the news ("📈 Stiff Records is on the rise") not a jitter.
- **Tier follows reach** across [tune] thresholds (Boutique < Independent
  < Major), gated so promotion also needs sustained capital — a one-hit
  boutique gets rich before it gets *big*.
- **The player's live deal tracks its label:** `RecordDeal.market_reach`
  (frozen at signing today) reads through to the label's current reach for
  *new* releases, while each release keeps freezing its own
  `label_market_reach` for its catalogue tail (the sales-fix invariant).
  Signing a rising indie that grows with you vs a safe major where you are
  one of hundreds becomes a genuine bet.

---

## §D — Rosters and the scene

Labels sign **scene bands**, not just the player, so the world has a
living industry the player competes inside.

- Each `Label` holds a `roster: Vec<RosterSlot>` — the player (when signed)
  plus scene bands. The ~180-band scene (`world/scene.rs`) gains a signing
  model: labels scout and sign scene acts by fit (genre specialty, the
  band's scene fame) and by the label's appetite (financial power, strategy).
- **Scene acts feed the label P&L (§A).** A signed scene band's chart
  releases (already generated by the scene sim) book wholesale margin to
  its label — this is what actually moves most labels' capital, so the
  world's fortunes are driven by the whole roster, not only the player.
- **Rival context.** The charts already show scene and ambient releases;
  now the player can see *which label* a rival act is on, and a label's
  fortunes visibly track its roster's chart performance. The player's
  success (or a labelmate's) moves the shared balance sheet.

Roster size and scouting cadence are [tune], calibrated so majors carry
deep rosters and boutiques a handful, and the scene stays 180 bands (no
new simulation load beyond the signing bookkeeping).

---

## §E — Competition and bidding

The deal-offer stream (`world/deals.rs`) becomes a **market**: labels
evaluate the player against their own means and appetite, and a hot act
draws competing offers.

- **Multiple concurrent offers.** More than one label may table an offer
  in the same window; each is priced from *that label's* live capital,
  reach, royalty norms, and strategy (§G) — not a static JSON row.
- **Bidding.** When two+ labels want the player, they compete: advance and
  royalty escalate toward the richer/hungrier label's ceiling [tune],
  bounded by `financial_power` and `capital` (a label cannot advance money
  it does not have — a key consequence of §A).
- **Appetite tracks the world.** A label deep in the red (§A/§F) signs
  cautiously or not at all; a flush major on a hot streak reaches for
  marquee names. The offer the player sees is a readout of the industry's
  state, not a die roll.

Reuses the existing offer lifetime / expiry beats (`DEAL_OFFER_LIFETIME_
WEEKS`, the renewal window) — this extends how offers are *priced and
sourced*, not the paperwork around them.

---

## §F — Mergers, acquisitions, and failure

Labels can rise and fall all the way — including out of existence. Real
1970s–80s industry history: the majors consolidated the independents.

- **Distress.** `capital` below a [tune] floor for N weeks →
  `LabelStatus::Distressed`: stops signing, leans on its roster's
  single-cuts (the existing `label_moves.rs` machinery), sheds unrecouped
  acts at term (the existing breach path).
- **Acquisition.** A flush major may acquire a distressed label: the
  roster and the player's contract **transfer** to the acquirer. Terms
  carry, but the new owner's reach/strategy now apply — your record can
  end up on a bigger label overnight, or a colder one. News line; the
  player's deal identity updates.
- **Folding.** A label that cannot be saved or sold **folds**: active
  deals void (the player becomes a free agent with a news line and any
  unrecouped balance written off, reusing the breach write-off path), its
  catalogue goes dormant.
- **Founding (optional, [tune] gate).** Across a 15-year timeline, new
  boutiques may appear so the low tier does not bleed empty as acquisitions
  thin it — off by default until L-SIM shows the population needs it.

All of this rides the existing weekly world tick and news log; every
industry event the player can be affected by is a news line, never silent.

---

## §G — Label character (A&R)

Labels are not interchangeable. Each carries a `LabelStrategy` and the
personality stats already in the JSON (`artist_development`,
`creative_freedom`), now load-bearing:

- **Strategy** [tune] — e.g. `ArtistFriendly` (higher royalty, more
  creative freedom, patient with unrecouped acts), `HitFactory`
  (big advances, aggressive single-cuts, short patience), `GenrePurist`
  (only signs its specialty, but pays a premium for fit).
- Strategy shapes offer pricing (§E), single-cut aggression and memo
  cadence (`label_moves.rs`, the recoup-pressure dials from v0.7 §E-5),
  and renewal behaviour (v0.7 §E-4). A cutthroat hit-factory in the red is
  a very different labelmate than a patient boutique.
- **Creative freedom** gains a real effect (currently cosmetic): low
  freedom raises single-cut aggression and can nudge the era-genre pressure
  the label applies; high freedom leaves the act alone. [tune]

---

## §H — UI

- **Label ledger / browser modal:** a label's tier, reach trajectory,
  capital band (qualitative — "flush" / "steady" / "in the red", not a spied
  exact figure unless signed to them), roster highlights, and recent
  industry moves.
- **Deal picker shows the label, not a row:** each offer annotated with the
  label's health and trajectory, so "rising indie vs safe major" is legible
  at the table (§C/§E).
- **Industry news:** mergers, foldings, a label going distressed, a rival
  act's breakout — the world's fortunes surfaced in the existing news log
  and, for the player's own label while signed, its ledger movement.

No new input paradigms — reuses the modal/picker patterns from v0.7.

---

## §I — Sim lab, bots, and tests (L-SIM)

The balance lab is the contract for every [tune] number. New and extended
coverage:

- **Label solvency over 15 years:** across all seeds, the label population
  neither hyper-inflates (everyone a flush major) nor collapses (mass
  foldings) — a stable industry with mobility. Majors stay dominant in
  aggregate; at least some boutiques rise a tier across a run.
- **The gangbuster target:** a Platinum-catalogue player's label clears
  era-real millions; the player as a recouped superstar earns era-real
  money (six-plus figures on a hit), not the compressed thousands. A
  matched-lifestyle homebody still survives (§B ratios intact).
- **Bidding works:** a fame-100 act draws competing offers and clears a
  higher advance/royalty than the same act drew as an unknown.
- **Failure is real but rare:** foldings/acquisitions occur across the seed
  set but are not the median label's fate; a player signed to a folding
  label is handled gracefully (free agent, no panic).
- **Scale re-validation (L-SCALE):** every v0.7 §F ratio target holds after
  the rescale (lifestyle upkeep 10–25% of income; van profitable 15–35,
  full production 75+; signed act nets ≥ indie over a full deal).
- **L-CERT (era accuracy, folded in from the sales-fix follow-up):**
  certifications gain a **year gate** — no scheme before 1973, no
  RIAA-style Platinum before 1976, no multi-platinum before 1984 — so the
  early timeline stops awarding certifications that did not historically
  exist. Thresholds themselves stay 50k/150k/400k (a UK-scaled abstraction,
  decided in v0.7); only their *availability* gates on the year.

Determinism trio passes **unmodified**; every new field `#[serde(default)]`;
the v0.4 fixture and `saves_from_v0_4_still_load` stay sacred; the world
RNG stays injected from `world_seed`, never ambient.

---

## Key design constraints — do not violate

- **A label's money is real and conserved.** Every dollar a label spends
  (advance, pressing, promo) leaves its capital; every copy its roster
  sells adds margin. Recoupment retains the player's royalty *to the
  label*, it does not delete money.
- **Reach and tier drift, never teleport.** A label's rise or fall is a
  trajectory the player can read and bet on, bounded per year.
- **A label cannot spend money it does not have.** Advances and pressing
  are gated by `capital`/`financial_power` — the constraint that makes
  distress and bidding meaningful.
- **One honest money scale (§B), moved in lockstep.** The rescale preserves
  every v0.7 affordability ratio; it changes magnitude, not balance shape.
- **Every industry event that touches the player is a news line.** Mergers,
  foldings, distress, a deal changing hands — never silent, same rule as
  v0.7's label memos.
- **The sales-fix invariants hold:** a release freezes its own
  `label_market_reach` for its catalogue tail; Local is a UK subset and
  never double-counts; charts stay pure-score lifecycle at depth 100.
- **Serde:** every new field `#[serde(default)]`; never rename serialized
  fields; addiction/energy dormant fields stay.
- **World RNG injected, not ambient.** Label scouting, bidding, and M&A
  rolls all derive from `world_seed`.
- **The Musician plan (FUTURE §1–§8) is untouched**; §9.1/§9.3/§9.4 stay
  deferred.

---

## Task map (→ `HANDOFF.md`)

| Task | Section | Gist |
|------|---------|------|
| L-SCALE | §B | Decompress to one era-real money scale; re-validate every v0.7 ratio. **Sequenced first.** |
| L-ENTITY | §A | `Label` world entity, worldgen seeding from JSON, the weekly wholesale ledger. |
| L-REACH | §C | Dynamic reach/tier drift; live deal reads through to the label. |
| L-ROSTER | §D | Scene-band signing; roster P&L feeds label capital. |
| L-BID | §E | Priced-from-state offers, multiple concurrent, bidding escalation. |
| L-MNA | §F | Distress, acquisition, folding (+ optional founding). |
| L-CHAR | §G | `LabelStrategy` + creative-freedom effects wired into offers/single-cuts/renewals. |
| L-CERT | §I | Year-gate certifications to the era (folded in from the sales-fix follow-up). |
| L-UI | §H | Label ledger/browser modal, deal-picker health annotations, industry news. |
| L-SIM | §I | Bots + assertions for solvency, gangbuster targets, bidding, graceful failure. |

Dependency spine: **L-SCALE → L-ENTITY → {L-REACH, L-ROSTER} → {L-BID,
L-MNA, L-CHAR} → L-UI**, with L-CERT independent and L-SIM continuous.
