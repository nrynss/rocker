## A living scene: design proposal

The scene is 120–260 bands that release, chart and occasionally vanish. This
document proposes what would make it feel *inhabited* — and what it would cost
in determinism, save compatibility and turn time.

### What the scene is now, and why it feels dead

A `SceneBand` (`world/scene.rs:19-33`) is a name, a `fame: u8`, a `peak_fame`,
a `latest_release` string, a `genre`, an `Option<String>` label and a
`momentum: i8`. That is a set of statistics, not a band. Three consequences:

1. **Nothing arrives or leaves visibly.** Bands die by a silent coin-flip:
   `fame < 8 && momentum <= 0 && rng.gen_bool(0.05)` (`scene.rs:192`), and only
   if `peak_fame >= 40` does the death produce a news line (`scene.rs:193-195`).
   A band nobody has heard of simply ceases to exist, unremarked. There is no
   formation event either — replacements are conjured by worldgen.
2. **There is no geography.** Every band exists everywhere at once. The
   evidence says the opposite: London rehearsal space cost five times the
   provincial rate (F59), and the pub circuit was a set of *specific rooms* in
   *specific places* — the Hope and Anchor, the Nashville, the Tally Ho (F05).
3. **The player has no relationship to any of it.** `label: Option<String>` is
   the only link, and it is one-way. No band has ever taken the player's
   support slot, poached a member, or shared a bill.

The player's band therefore moves through a crowd of strangers that occasionally
changes shape.

### Proposed: band lifecycle

Give each band a **phase** and make every transition emit a news line. The arc
follows the evidence: most acts formed, gigged the pub tier at zero margin, and
never broke at all (F69, F72); a few got signed and dropped fast (F22).

| Phase | Meaning | Enters when | Exits to |
|---|---|---|---|
| `Forming` | exists, no releases, playing pubs | worldgen, or a `Split` band later reforms | `Hunting` after N weeks |
| `Hunting` | gigging, no deal, building local draw | `Forming` + time | `Climbing` on a deal, or `Split` on burnout |
| `Climbing` | signed, releasing, charting | a label signs them | `Peaking` on momentum, `Slipping` on a flop |
| `Peaking` | at or near their ceiling | peak momentum reached | `Slipping` |
| `Slipping` | declining fame | momentum turns negative | `Split` or a late `Climbing` |
| `Split` | gone | decline, death, or sacking | `Forming` again (reformation) |

**New fields** (all `#[serde(default)]`; nothing renamed):

| Field | Type | Default | Purpose |
|---|---|---|---|
| `home_town` | `SceneTown` | `London` | the geography axis (six towns) |
| `local_draw` | `u8` | 0, read as `fame / 2` | pub following, distinct from national fame |
| `phase` | `BandPhase` | `Unknown` | the lifecycle state |
| `formed_week` | `u32` | 0 | how long they have been at it |
| `split_count` | `u8` | 0 | reformations tell a story |

`local_draw` defaults to *being read as* `fame / 2` rather than being written,
so an old save needs no migration pass — the field materialises on first write.
`phase` defaults to `Unknown` and is inferred once from fame/momentum **without
an RNG draw**, so loading an old save cannot shift the random stream.

**The rule that makes it feel alive:** every phase transition that a player
could plausibly have heard of writes one news line — *"The Rezillos have
split"*, *"New band on the circuit: The Undertones, from Derry"*. Today only
`peak_fame >= 40` splits are announced (`scene.rs:193`); the change is to
announce by *local* significance — a band the player has shared a bill with, or
one on the same town circuit — not only by national fame.

### Proposed: the local circuit

Six towns (London, Manchester, Liverpool, Birmingham, Glasgow, Leeds), each
with its own venue ladder and its own genre temperature. A band's `local_draw`
grows by playing its home town; `fame` grows by breaking out of it.

- **A gig is a local event.** Playing your home town builds `local_draw`; playing
  another town builds almost nothing until you have fame there.
- **London is a different market.** The evidence is explicit: rehearsal costs
  5x (F59), and London was where the A&R men actually went (F05). London gives
  better access to labels and worse margins. That is the trade the player
  makes by moving.
- **The circuit is shared.** Bands and the player draw from the same pool of
  venue slots, so a rival's rise can cost the player a booking.

### Proposed: genre waves

The wave model already half-exists (`timeline.get_trending_genres()` drives
`on_trend` in `scene.rs`). Make the wave do three things it does not do now:

1. **Shape a band's odds.** A band whose genre is on the wave gains momentum
   faster and can break out of its local ceiling; off-wave, momentum decays.
   This is what makes 1976 feel different from 1973 without changing any rule.
2. **Create and kill bands.** Bands form *because* a wave is rising (pub rock
   bands appear 1972-74; punk bands 1976-77), and a dying wave strands its
   latecomers in `Slipping` — the authentic "signed a punk band in 1979" trap.
3. **Stay bounded.** Punk's DIY turn was real but limited: there was no
   independent chart until January 1980 (F31), and singles never collapsed by
   units (F32). The wave changes *who gets heard*, not the size of the market.

### Proposed: visible drama each week

Concrete, cheap news lines the player will see:

- **A band splits** — named, with its peak fame, and whether the player shared a bill with it.
- **A band forms** — a new name on the player's town circuit.
- **A rival signs** — "X have signed to Y" (the label is on the roster already).
- **A rival charts** — first entry, and highest position.
- **A rival takes your slot** — if the player declined a booking, or lost a venue relationship.
- **A wave breaking** — "Punk is taking over the London pubs" when the trend crosses a threshold.
- **A local scene note** — the Hope and Anchor has changed hands; a new club has opened.
- **A reformation** — a split band returns, usually worse.

Each is one string appended to the existing `news` vector; cost is nil.

### Proposed: the player inside the scene

The scene becomes a set of *relationships*, not a backdrop:

- **Shared bills.** When the player plays a venue, one or two scene bands are on
  the same bill. This is how the player learns names.
- **Poaching.** A rising band with money may approach a player's member — the
  mechanism already has the dormant relationship fields to hang this on.
- **Support acts.** A scene band bigger than the player can offer them a support
  slot; a scene band smaller can take one (unpaid — F71).
- **Rivalry.** Repeatedly losing a venue slot to the same band builds a rivalry
  that colours news text and negotiation.

### Determinism, serde and cost

- **Determinism.** Every new roll draws from the injected `rng` in the same
  order it does today. The phase inference on load is *deterministic and
  RNG-free*, so an old save does not shift the stream. The three determinism
  tests must stay unmodified; the risk is any new `rng` call inside a `retain`
  closure (order-dependent) — avoid it, or use index loops.
- **Serde.** Every field `#[serde(default)]` with a sensible default; no renames;
  `local_draw` materialises lazily so no migration is needed.
- **Cost.** The weekly scene loop is already O(n). Phase transitions are O(1)
  per band. The only O(n²) risk is "which bands share a bill" — do that by
  iterating a *filtered* candidate list (same town, same week), not
  all-pairs. At 260 bands this is negligible.

### Grounding

| Mechanic | Evidence |
|---|---|
| Pub-tier zero margin; most acts never broke | F69, F72 |
| London vs provincial cost gap; specific London rooms | F59, F05 |
| Labels scout and drop fast; hype spikes | F22 |
| No indie chart until Jan 1980; DIY bounded | F30, F31 |
| Singles market did not collapse | F32 |
| Support slots unpaid | F71 |

**Labelled invention:** the six-town set, the six-phase state machine, the
reformation mechanic, the rivalry system and the specific news templates are
design choices, not historical findings. The evidence constrains the *shape*
(movement was local-first, most acts failed, the circuit was specific rooms) but
not these exact rules.
