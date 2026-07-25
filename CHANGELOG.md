# Changelog

Notable changes to Rocker, newest first. Format loosely follows
[Keep a Changelog](https://keepachangelog.com/); versions follow
[SemVer](https://semver.org/) in spirit — this is a game, the API is vibes.

## Unreleased

**The tour-economics pass** — clearing the touring follow-up left open at the
close of the Label Cycle, plus the length bug found underneath it.

### Fixed

- **Full production is a rig you can actually choose.** A rig upgrade pays for
  itself when `territory pot × Δcapacity > Δcost`, and the ladder's top step
  didn't: cost ran van→full 25× on the rig table (47× once `markets.json`
  travel/equipment modifiers compounded) against a capacity spread of only
  2.1×, so truck→full needed a pot around $81k where the region topped out near
  $45k. Full production wasn't merely unprofitable — it was **dominated**, never
  the best choice in any reachable game state. The top two capacity multipliers
  widen (1.3→1.4, 1.7→2.0), which puts every crossover back on the map: once
  the act has built a territory, each rig up the line out-earns the one below
  (at regional fame 100: van +$58k, bus +$70k, truck +$89k, full +$101k), while
  arriving cold with the big show still loses (−$16k) exactly as designed.
- **Touring longer earns more.** The whole-tour gross pot had **no `weeks` term
  at all**, while shows booked *and* cost both scaled with length — so twenty
  nights grossed precisely what five did, split twenty ways, for four times the
  money. Every rig lost by touring longer; full production went +$17,854 at one
  week to −$50,948 at four on an unchanged quoted gross. The pot now scales
  `weeks^0.65`, normalized at one week: a fourth week still earns less than the
  first (a territory saturates) but it earns something, which is the trade-off
  design §A actually asked for rather than a strictly dominated button.
- **Touring pays well without crowning itself.** Giving length a real gross
  term multiplied the longest bookings' box office ~2.5×, and the sim lab
  caught what that did to a whole career: the road-dog bot's 15-year median
  went $4.26M → $18.99M and *passed the signed superstar's $16.75M*, inverting
  the Label Cycle's thesis that a label's worldwide reach is the top of the
  economy. The bug had been suppressing an entire playstyle; un-suppressing it
  shouldn't crown it. The box-office coefficient comes down (0.45 → 0.30) with
  a matching trim to the top two rigs' weekly cost, which moves magnitude
  without disturbing any ordering the pass exists to fix. Touring lands at a
  road-dog median of **$12.23M** — a real career, roughly triple what the bugs
  allowed, still under the signed superstar. Every other bot's 15-year median
  is byte-identical to main: the headline tour is the only thing that moved.
- **The v0.8 "permanent money-loser" reading was measuring one point.** The
  balance sweep only ever toured *cold* territories (regional fame 0), and
  regional fame is the dominant term in the gross pot — a 0–100 range against a
  fame contribution capped at 33. The sweep now varies it, and pins the rig
  ladder's inversion, the cold-territory ordering, and length monotonicity so
  none of the three can silently drift back. It also pins the texture the
  retune produced: on a *moderately* built territory the truck & crew is the
  right rig and full production plateaus with length — a rig too big for the
  market it's dragged around is supposed to bleed.

### Changed

- **The tour quote shows the bottom line.** The picker adds a colour-coded
  projected net (green profit / yellow marginal / red loss) and the regional
  fame already banked in the territory — the number that explains why the same
  rig loses money on a first visit and prints money on a return. §A always
  required a money-losing tour to be a decision rather than a surprise; it was
  leaving the subtraction to the player.

## 0.8.0 — 2026-07-25

**The Label Cycle** — record labels become a living economy. Labels stop being
inert JSON and gain a balance sheet that rises on hits and falls on flops, a
roster of real acts, reach and tier that grow and decline, competing offers and
bidding wars, mergers and failures — on a money scale where a superstar and
their label both earn what the era actually paid. Design lives in
`docs/DESIGN-v0.8-label-cycle.md`. Closed green: **243 tests** (from a 188
baseline) + 8 balance sweeps, determinism trio unmodified, clippy `-D warnings`
clean, fmt clean, v0.4 saves still load.

### Added

- **Labels are world entities with a balance sheet.** Each seeds from
  `record_labels.json` with real `capital`; every copy its roster sells books
  the ~$2.70/copy wholesale margin to it, and advances/pressing/promo debit it.
  Recoupment stops vanishing into a void — the player's royalty is *retained to
  the label* while unrecouped, exactly the historical mechanic. A Gold record
  nets its label ~$405k; a platinum act's catalogue, millions.
- **Reach and tier that rise and fall.** `market_reach`/tier drift toward a
  target set by capital and recent hits, bounded per period with hysteresis — a
  boutique with a breakout can climb toward major, a major that loses its stars
  declines. A signed act's new releases read the label's live reach; catalogue
  tails keep the reach they were released with.
- **A living roster.** Labels scout and sign scene bands by genre fit and
  appetite; their chart activity moves label capital — the whole industry's
  fortunes turn, not just the player's.
- **Competition and bidding.** Deal offers are priced from a label's live
  capital/reach/financial-power (not a static row); a hot act draws multiple
  offers and a bidding war, each label bounded by what it can actually advance.
- **Label character (A&R).** Strategies have teeth — a hit-factory major pays
  big advances at low royalty and cuts singles aggressively; an artist-friendly
  indie pays higher royalty and is patient; a genre purist only signs its
  specialty. Creative freedom now shapes single-cut aggression.
- **Mergers, acquisitions, and failure.** A label bleeding money goes
  distressed, gets acquired (your contract transfers to the buyer), or folds
  (you're freed, the debt written off) — each surfaced in the news.
- **See it in-game.** A Discography browser (`d`) shows each record's sales —
  copies, gross earnings, chart peak, certification; a label-ledger browser
  (`b`) shows tiers, reach, and balance sheets (your own label's exact capital,
  rivals' as a qualitative band). Deal offers are annotated with the offering
  label's health; the charts show which label a rival act records for.

### Changed

- **One honest, era-real money scale.** The v0.7 income compression is retired;
  every dollar — record royalties, tours, gigs, advances, and all costs — moved
  ×3 in lockstep, so a signed superstar's career now clears era-real millions
  (sim lab: median ~$16.7M) while the affordability ratios hold.
- **Distribution reaches the world.** Foreign-territory sales (Europe, America,
  Japan) used to require touring each country; now a label's reach — and a
  bought indie distribution channel — puts records in foreign shops on release,
  with touring deepening a market on top. This is what makes records actually
  sell: a fame-100 act could previously hold the charts for a decade and never
  certify, and the international *independent* path never existed at all.

### Fixed

- **Certifications gate on the era.** No certification scheme before 1973, no
  RIAA-style Platinum before 1976, no multi-platinum before 1984 — the timeline
  stops handing out awards that did not yet exist.

### Known follow-ups

- ~~Full-production tours never turn a profit, even at fame 100 (−$26k) — a
  trap inherited from v0.7 tour economics, awaiting a cost/capacity retune.~~
  Resolved by the tour-economics pass; see Unreleased above.
- M&A's distress floor is $0, so it's a safety valve that doesn't fire in
  natural play (the solvent economy only grows labels); attrition is exercised
  by an injected-distress sweep.
- Certification frequency is now ~71 events over a relentless 15-year signed
  career — the intended "records sell worldwide" direction, an order above the
  retired v0.7 "1–3 silver" target (which described the pre-fix, UK-only economy
  in the 0.7.1 notes below).

## 0.7.1 — 2026-07-17

Money Cycle repairs: two reported bugs (#20, #21) plus everything a sweep
for their bug classes turned up.

### Fixed

- **Label recoupment is possible again** (#20). The M7 sales rescale
  tripled the copies every pressing charged to the recoupment ledger while
  royalty dollars stayed flat, so below a 30% royalty the advance could
  mathematically never be repaid — every major and most indies. The
  per-copy ledger charge is rescaled (and then tuned to $0.05/copy) so the
  median signed act recoups its advance before the term's halfway mark —
  now asserted in the sim lab so it can't silently regress again.
- **Indie pressing pays for itself again.** The same rescale miss on the
  player's side: per-copy pressing bills were still on the pre-M7 scale, so
  a sold-out album run *lost* money outright in every era with a recording
  cost modifier above 4/3 — five of the ten. Per-copy costs now divide by
  the income divisor, restoring the pre-M7 margins; a new test pins a
  fully-sold run out-earning its bill in every era.
- **Sell-out news reads in order** (#21). The sold-out line now reports
  the run that actually sold out (not the ledger-inflated post-restock
  count), and the label's fresh-run announcement follows the sell-out it
  reacts to instead of preceding it. First-run certification news got the
  same treatment: sales line, then the award, then the restock.
- **Instant actions no longer mint bonus sales weeks.** The weekly sales
  pass ran after every action — including instant ones (marketing,
  lifestyle moves, re-presses, deal responses) that don't advance the
  calendar — so each one re-sold a full catalog-tail week: free copies,
  income, recoupment paydown, and certification progress. The pass now
  resolves at most once per game week.
- **"Fame +N" tells the truth.** Gig, tour, support-slot, and record-sales
  lines all reported the pre-multiplier fame gain: during a comeback the
  band actually gained double what the log said, and at a cap it gained
  less. The log (and the tour report) now show the fame actually applied.
  Same for a tour's regional fame line when the 100-cap eats part of the
  rolled gain.
- **Label pressing runs carry the M7 rescale, and certifications are
  honestly reachable again.** Label run sizes were still sized for the
  old demand scale, so a signed act's first run nearly always sold out
  and the over-cap demand silently evaporated — runs now press 3× (the
  same coupling as the other per-copy fixes). And with the bonus-sales
  exploit closed, the certification balance previously propped up by that exploit is
  retuned through the catalog tail (gentler decay, higher weekly rate):
  a signed 15-year career medians 3 certifications in the balance sweep,
  inside the design's 1–3 target, where the exploit-free game previously
  allowed none — for anyone, ever.

## 0.7.0 — 2026-07-16

The Money Cycle: where the record business gets its hooks into you. Tours
quote before they bill, you finally have a home (with rent), the charts
span the world and hold still long enough to climb, records go gold, and a
label's money turns out to be a loan with a clock attached.

### Added

- **Tours you book, not tours that ambush you.** Touring is now two
  choices — a **rig** (van up to full production) and a **length** (one to
  four weeks) — with an itemized quote shown *before* you commit: cost,
  shows, and a projected gross. Fame no longer secretly re-prices a tour;
  the same region and rig cost the same at any fame. A bigger rig books
  bigger rooms (more reach, more wear), and the old dead travel/equipment
  numbers in the data finally do something.
- **A roof over your head.** Live where you can afford to — squat, shared
  flat, city apartment, townhouse, mansion — each with weekly rent and its
  own pull on stress, happiness, and how well rest restores you. Moving up
  costs a deposit and lifts the spirits; moving down stings; and if you
  stay in the red too long the landlord decides for you. Stay in a squat
  once you're famous and the tabloids will have opinions.
- **Charts that span the world and actually breathe.** Four sales
  territories — UK, Europe, America, Japan — each its own Top 100, plus a
  home-scene Local board and a Worldwide chart that sums the territories.
  Records now debut, **climb for a couple of weeks, peak, and slide** on a
  gentler decay instead of blinking out — and where a record charts abroad
  depends on your distribution reach and the regional fame you build by
  touring there. Home sales ride your reach directly; the world you have to
  go win.
- **Silver, gold, platinum.** Records certify as their sales pile up —
  a news moment, a fame and reputation bump, and a badge on the release —
  from silver through multi-platinum.
- **A label's money is a loan.** The advance, the pressing, the promo — a
  signed act pays it all back out of royalties before a cent reaches the
  band, with the running tab shown each week. The label keeps a hit in
  stores (re-pressing on its own), and an unsigned act can now re-press a
  sold-out record itself and buy into wider distribution tiers instead of
  being stuck at mail-order reach.
- **A contract with a clock.** Deals now carry a term as well as an album
  count — you're free at the later of the two, so delivering early keeps
  you on the roster, not out the door mid-release. Let the term lapse with
  albums still owed and the label drops you (and remembers). Six months
  before a healthy deal ends the label reads its ledger and makes its move:
  a richer new contract if you've paid off and sold well, a protective
  extension if you haven't, silence if you're deep in the red. And a label
  that's owed money leans on you — memos asking for songs and singles, and
  real deadline pressure as the clock runs down.

### Internal

- Every tunable value across the six systems is validated in the `sim.rs`
  balance lab — now with home-body, road-dog, and indie-lifer bot
  policies alongside the originals, and career metrics for certifications,
  breaches, and lifestyle. Sales volume, income, tour box office, and rig
  costs were calibrated against fifteen-year, sixty-seed sweeps; the
  three determinism tests pass unmodified.

## 0.6.0 — 2026-07-14

The Life Cycle: the four bars from the original 1989 game are back — with
stress as the pressure valve instead of a flat energy meter — every gig
and tour night resolves on its own, fame has real gravity instead of a
flat decay, and the game no longer ends when you make it.

### Added

- **Four bars: Health, Stress, Happiness, Creativity.** Energy is retired
  as a mechanic. Work raises stress; stress drains happiness and
  creativity; success — a great show, a hit record — refills them faster
  than the grind empties them. Writing only costs creativity when it's
  forced: three-plus consecutive weeks at the desk, or writing while
  stressed past 50. Happiness swings songwriting and recording quality
  by up to 20%.
- **Every show is its own night.** Gigs and tours now roll a reception
  (rough / solid / great / transcendent) and a box office for each
  performance, not one lump sum. Band skill is the dominant term — a
  tight, uninspired band is still reliably excellent — creativity only
  widens the chance of a transcendent night. Tours carry momentum from
  stop to stop: a hot streak sells out the back half, a bad night
  deflates it. Press `r` for the full per-show tour report.
  Great and transcendent nights feed creativity back; a tour that goes
  very well lifts happiness too.
- **Fame has gravity.** Idle decay now scales with how famous you are —
  a nobody fades after two quiet weeks, a stadium act gets a full year of
  grace — and decay ramps in gently (−1, −2, −3, −4, then a flat −5)
  rather than jumping straight to full speed. Every peak you've ever hit
  earns a permanent floor: fame can cool off, but a genuine legend never
  gets forgotten. Reclaiming lost ground is faster than the first climb.
  Charting, and — once you're established — a record released in the
  last year, both count as staying in the public eye.
- **Labels act on their own.** A signed act that goes quiet with
  un-singled album tracks may find its label has released one without
  asking — a real single, pressed, promoted, and chartable, whether the
  band likes it or not.
- **The story keeps going.** Reaching rockstar status is now a milestone,
  not an ending — a one-time celebration, then the career continues.
  Only death or going broke and unknown end a run.
- **More incidents, and richer ones.** Random incidents fire every week
  instead of every other, and now move all four bars — parties, gear
  disasters, press moments, fan encounters, and more — defined entirely
  in `data/incidents.json` rather than hardcoded.
- **The sales tail lives.** Post-launch marketing campaigns actually
  move catalog sales now (they used to be computed and ignored); a
  record's long-tail income also responds to the band's current fame,
  so a rising star's back catalog keeps earning.

### Internal

- Stat, fame-gravity, and per-show engines are validated against the
  `sim.rs` balance lab across 60-seed, 15-year sweeps; every tunable
  value in the design landed at its shipped number with no retuning
  needed. Fixed a fame-decay edge where a falling star's ramp could
  skip steps across a grace-tier boundary (new serialized
  `Game::decay_streak`).
- `energy` and the two addiction fields remain on `Player`, serialized
  but dormant — the drug/addiction system, vacations, and the manager
  (FUTURE.md §9) are deferred to a later cycle, as is the Musician
  identity rework (§1–§6).
- 100 tests passing (57 more than the 0.5.1 baseline), clippy and fmt
  clean, all determinism and save-compatibility contracts (including
  `saves_from_v0_4_still_load`) hold unmodified.

## 0.5.1 — 2026-07-14

Structure-hardening cycle: pure refactor, no gameplay changes — the big
files were split into owned packages so the next feature cycle lands in
small, reviewable diffs.

### Internal

- Structure-only refactor, no behavior change: the four monoliths were
  split into owned packages. `src/game/mod.rs` (~1,060 lines) → a 27-line
  module shell plus `core`, `constants`, `rng`, `genre`, `events_apply`,
  and a by-concern `tests/` package; `world.rs` (~1,090) →
  `world/{mod,scene,charts,deals,venues}`; `actions.rs` (~720) →
  `actions/{studio,live,business,rest}`; event outcomes out of `turn.rs`
  into `events_apply.rs`.
- UI split to match: `render.rs` (~1,050) →
  `render/{layout,panels,modals/*,setup,game_over}` and `app.rs` (~990) →
  `app` + `input/*`. Every production `.rs` is now under ~500 lines.
- Same 42 tests (2 ignored), relocated by concern under `src/game/tests/`;
  `cargo test` / `clippy -D warnings` / `fmt --check` all green. No
  save-format, RNG-stream, or gameplay changes.

## 0.5.0 — 2026-07-13

The cycle where the scene pushes back: records gate your rise, the charts
keep score, labels scout like labels, and every career can be replayed
from its seed.

### Added

- **Genre identity.** Pick your genre when founding the band; releases
  carry it, the era's tastes swing sales for and against it, and the
  press says so when your scene heats up or the times move on.
- **The charts are a shared scoreboard.** Player releases compete with
  the scene's on the weekly chart — `c` opens the top 10 with your
  entries highlighted, and the scene panel shows the current No. 1.
- **Pressing runs.** Recording independently now means choosing a run
  (500–50,000 copies) with setup plus per-copy costs. You can't sell
  copies that were never pressed; sell-outs are called out. Signed bands
  press on the label's dime, sized by its reach and your name.
- **Deal offers with a shelf life.** Offers expire after a few weeks if
  ignored — and expiry is not rejection: nobody poaches a lapsed deal,
  and scouting resumes once the slate is clear.
- **A real scouting arc.** Independent labels court working acts early,
  boutiques come mid-career, majors only chase genuinely big acts — the
  catalog and a charting record weigh in alongside fame.
- **Deterministic careers.** Every roll — worldgen, the weekly scene,
  and now every player action — derives from the world seed. Same
  `ROCKER_SEED`, same choices, same career, same log lines.
- **The balance lab.** Bot-driven career sims (gig-grinder, studio-rat,
  balanced-indie, label-loyalist) run whole careers headless; fast
  invariants run with the suite, full 15-year sweeps behind
  `cargo test -- --ignored --nocapture`.
- **Save compatibility, proven.** A real v0.4.0 save is committed as a
  fixture; a test keeps it loading and playable forever.
- **CI gates.** Tests on Linux/macOS/Windows, `clippy -D warnings`, and
  `rustfmt --check` on every push and PR.

### Changed

- **Fame is earned in layers.** Live fame is capped twice — by the
  venue (its prestige plus headroom) and by your catalog (a base
  ceiling that each single and album lifts). Gigging alone plateaus;
  records raise the roof.
- **The spotlight fades.** After one quiet week with no shows and
  nothing on sale, fame decays weekly until you're back in the
  public eye. Take a Break is now a real four-week disappearance with
  full recovery.
- **Unit economics.** Demand is sales score × distribution reach;
  indies earn $2 a copy against their own fame-limited reach, labels
  $3 × royalty through their network. The long tail draws down the
  pressing, not an infinite well.
- **Marketing belongs to the label when signed** — their promo machine
  auto-pushes every release; running your own campaign is an indie move.
- **First tuning pass from the balance lab:** slower fame climb so
  records, not gig-grinding, gate the win.

### Fixed

- Cross-track semantic conflicts from merging the cycle's parallel
  work (player chart entries now score with genre modifiers applied;
  setup seeding and genre pickers compose).

### Internal

- `src/game/mod.rs` (2,300 lines) split into `actions`, `economy`, and
  `turn` modules; the Game struct, constants, and tests stay put.
- Crate-wide `#![allow(dead_code)]` removed and the 24 methods it hid
  deleted; `thread_rng` no longer appears anywhere in the tree.
- Repo-wide `rustfmt`; this changelog.

## 0.4.0 — 2026-07-13

Baseline for the cycle above: the ratatui career sim — write songs,
record, gig, tour, sign deals, weather random and historical events,
save/load — with data-driven worldgen and a living scene of hundreds of
bands. Relicensed under AGPL-3.0-or-later.
