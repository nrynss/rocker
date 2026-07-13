# 🎸 Rocker - Rock Star Management Simulator

A Rust-based clone of the classic 1989 DOS game "Rockstar" by Wizard Games, enhanced with historical accuracy and customizable content. Start your musical journey in 1970, right after the Beatles' breakup, and navigate the changing music industry through the decades!

## 🎮 About the Game

Rocker is a text-based management simulation where you play as an aspiring rock musician starting in 1970. Experience the authentic evolution of the music industry while managing:

- **Historical Timeline** - Live through real music eras from 1970s post-Beatles to 1990s grunge
- **Band dynamics** - Keep your bandmates happy and skilled with procedurally named characters
- **Health & energy** - Balance work with rest to avoid burnout and the excesses of rock life
- **Finances** - Earn money through gigs while managing era-appropriate recording costs
- **Fame & reputation** - Build your following from local pubs to international stadiums
- **Creative output** - Write songs and record albums with generated titles
- **Industry challenges** - Deal with record labels, competition, and random events
- **Market evolution** - Adapt to changing musical trends and economic conditions

## 🕒 Historical Features

### Era-Based Gameplay (1970-1990+)
- **1970**: Post-Beatles Revolution - High innovation, emerging FM radio
- **1972**: Glam and Progressive Era - David Bowie influence, concept albums
- **1975**: Arena Rock Peak - Stadium tours, disco emergence
- **1977**: Punk Revolution - DIY culture, independent labels surge
- **1980**: New Wave Emergence - MTV preparation, synthesizer dominance
- **1983**: MTV Generation - Music videos essential, corporate rock boom
- **1985**: Corporate Rock Era - Live Aid, hair metal peak
- **1988**: Underground Brewing - Hip hop rises, alternative builds
- **1990**: Alternative Revolution - Grunge emerges, industry disruption

### Dynamic Market Conditions
- **Recording costs** change based on studio technology evolution
- **Genre popularity** shifts according to historical trends
- **Industry focus** moves between singles and albums across eras
- **Innovation climate** affects how receptive audiences are to new sounds

## 🚀 Getting Started

### Prerequisites

- Rust 1.88+ installed ([Install Rust](https://rustup.rs/)) — the project uses the 2024 edition
- A terminal (the game runs full-screen)

### Installation & Running

```bash
git clone <this repository>
cd rocker
cargo run
```

## 🎯 Game Mechanics

### Core Gameplay Loop
Each turn represents one week in your rock career:

1. **Choose an action** from the main menu
2. **Process consequences** - energy, money, health changes
3. **Handle random events** - opportunities and challenges
4. **Check progress** - are you closer to rockstar status?

### Actions Available

- **Laze Around** (1) - Recover energy and reduce stress
- **Write Songs** (2) - Create material for future releases
- **Practice** (3) - Improve band skill
- **Record Single** (4) - Release a single (requires songs, ~$100 studio time, plus your pressing run when unsigned)
- **Record Album** (5) - Release an album (requires 8+ songs, ~$1000 studio time, plus your pressing run when unsigned)
- **Play a Gig** (6) - Opens the venue picker to select a venue from local pubs to stadiums
- **Go on Tour** (7) - Opens the region picker to select a tour destination across global markets
- **Support Slot** (T) - Open for a bigger act when they come calling: modest pay, serious exposure
- **Take a Break** (8) - Four weeks off: full recovery, but the spotlight moves on without you
- **Visit Doctor** (9) - Restore health ($50)
- **Marketing** (M) - Run press, radio, and promo-film campaigns for your releases (independents only — a label runs its own promo)
- **Deal Offers** (V) - Review, accept, or reject record label offers
- **Save / Load** (S / L) - Persist your career to a JSON save file

Navigate with ↑/↓ and Enter, or press an action's hotkey directly.
Recording a release opens a 4-week sales window — market it before it drops
to boost its first-run sales.

Live shows only carry you so far: a venue can't make you more famous than
the crowd it holds, and without records word of mouth stalls — gig and tour
fame caps out until you put out new music. Records, and the labels that
distribute them, are what turn a hot local act into a star. Support slots
are the exception: opening for a bigger act reaches their audience, not
yours.

Fame also fades. After a quiet week with no shows and nothing in its sales
window, the public starts to forget you — stay on stage or on the shelves.

### Independent vs. Signed

Distribution is everything. Without a label your records only reach as far
as your fame carries them, and you press them yourself: choose a run from a
500-copy garage pressing to a 50,000-copy national one, paying setup and
per-copy costs up front. Press too few and a hit sells out with money left
on the table; press too many and you've burned cash on boxes in the garage.
A label presses and promotes every release itself — run size and promo push
scale with its network — but only pays you a royalty slice. Unknown bands
earn more signed; superstars can afford to go independent and keep
everything.

### The Scene & Record Deals

The scene lives its own life: 180+ scene bands release records, chart, sign with labels, break up, and new bands arrive to take their place. When an act much bigger than you likes what they hear, they may offer you the opening slot on their tour — support slots expire fast, so decide quickly.

Record labels will scout you as your fame grows. If you reject a record deal offer, the largest unsigned act on the scene may swoop in and poach the contract, which will be reported in the weekly news logs.

### Reproducible Seeding

The entire game world can be seeded. Launching the game with `ROCKER_SEED=42 cargo run` ensures that the starting world, names of competing acts, and week-by-week updates evolve deterministically, making runs reproducible and shareable.


### Win Conditions
- **Fame ≥ 90** AND **Albums ≥ 5** = YOU'RE A ROCKSTAR! 🌟

### Lose Conditions
- **Health ≤ 0** = Game Over (poor health)
- **Money < 0** AND **Fame < 10** = Game Over (broke and unknown)

## 🛠️ Technical Details

### Architecture
- **Rust 2024 edition** (MSRV 1.88)
- **Modular design** with separate concerns (game logic knows nothing about the UI)
- **Full-screen TUI** built with `ratatui` - panels, gauges, modals, and an event log
- **Serializable state** - save/load to JSON
- **Random events system** for dynamic gameplay
- **Market simulation** with economic cycles

### Key Dependencies
- `ratatui` - Terminal UI framework (crossterm backend)
- `serde` + `serde_json` - Serialization for save/load
- `rand` - Random number generation

### Platform Support
✅ **Windows** - Native terminal support
✅ **macOS** - Native terminal support
✅ **Linux** - Native terminal support

## 🎨 Features

### Current Features (v0.4.0)
- ✅ **Full-screen TUI** - ratatui interface with stat gauges, modals, and a live event log
- ✅ **Historical timeline** - Accurate music industry evolution 1970-1990+
- ✅ **Reproducible Seeding** - Fully seeded world generation and deterministic week-by-week updates via `ROCKER_SEED` env var
- ✅ **Venue-based Gigs** - Gig at 5 distinct venues with prestige fame gates, ticket sales attendance, and base payouts
- ✅ **Regional Markets & Tours** - Tour regions in the US, UK, Europe, Japan, and Australia, with population-tier fame gates, travel multipliers, and regional fame progression
- ✅ **Deal Poaching** - Scene bands dynamically poach record deals that you reject
- ✅ **Pressing runs** - Choose your own run size as an independent (and live with sell-outs); labels press and promote to the size of their network
- ✅ **Distribution economics** - Indie reach capped by your fame; labels bring reach but take their cut
- ✅ **Fame decay** - Disappear from view and the public starts to forget you
- ✅ **Support tours** - Bigger acts offer you opening slots: modest pay, major exposure
- ✅ **A living scene** - 180+ bands rise and fall with the trends, split up, and new bands debut
- ✅ **External data files** - Fully customizable names and content
- ✅ **Era-based mechanics** - Recording costs, trends, and market conditions
- ✅ **Generated content** - Procedural song titles, band names, venues
- ✅ **Record deals** - Label offers with advances, royalties, and album obligations
- ✅ **Marketing campaigns** - Press, radio, and promo films that boost release sales
- ✅ **Save/Load** - JSON save files
- ✅ Song writing, singles, and albums with a windowed sales model
- ✅ Random events system and market simulation
- ✅ Health/energy/stress management

### Planned Features
- 🔄 **Player choices in events** - Interactive decision making
- 🔄 **Multiple difficulty levels** - Easy to Rockstar mode
- 🔄 **Band member relationships** - Hiring, firing, and deeper social dynamics
- 🔄 **Music genres** - Style specialization (genre field is currently a placeholder)
- 🔄 **Chart tracking** - Billboard-style success metrics


## 🎵 Development

### Building from Source
```bash
# Development build
cargo build

# Release build (optimized)
cargo build --release

# Run tests
cargo test

# Run with debug output
RUST_LOG=debug cargo run
```

### Project Structure
```
src/
├── main.rs              # Entry point: data validation + terminal setup
├── data_loader.rs       # Loads the customizable data/ files
├── game/
│   ├── mod.rs           # Core game state, turn processing, sales model
│   ├── player.rs        # Player stats and attributes
│   ├── band.rs          # Band members, deals, and dynamics
│   ├── music.rs         # Songs, releases, marketing campaigns
│   ├── events.rs        # Random event triggering
│   ├── timeline.rs      # Historical eras (1970 onward)
│   └── world.rs         # Market conditions and competing bands
├── ui/
│   ├── mod.rs           # UI module declaration
│   ├── app.rs           # App state machine and input handling
│   └── render.rs        # ratatui layout and widgets
└── data/
    └── mod.rs           # Game constants and helpers
```

## 🤝 Contributing

This is a learning project, but contributions are welcome! Ideas for improvement:

- **Balance tweaking** - Game difficulty and progression curves
- **New events** - More variety in random events
- **UI improvements** - Better terminal graphics and layout
- **Additional platforms** - Web version, mobile adaptation
- **Localization** - Multiple language support

## 📜 License

GNU Affero General Public License v3.0 (AGPL-3.0-or-later) — see [LICENSE](LICENSE).

## 🙏 Acknowledgments

- Original **Rockstar (1989)** by Wizard Games - The inspiration for this project
- **Crossterm** library maintainers - For excellent cross-platform terminal support
- **Rust community** - For making systems programming accessible and fun

---

**Ready to rock? Start your journey from garage band to stadium legends!** 🎸🤘

*"It's better to burn out than to fade away..." - but try to avoid both in this game!*

## 📁 Customizable Content

### External Data Files
The game automatically creates editable text files in a `data/` directory:

- **Song titles**: `song_adjectives.txt`, `song_nouns.txt`, `song_verbs.txt`, `song_emotions.txt`, `song_places.txt`
- **Music industry**: `album_titles.txt`, `band_names.txt`, `record_labels.txt`
- **Characters & places**: `band_member_names.txt`, `venue_names.txt`, `city_names.txt`

### Easy Customization
- **Edit any file** to add your own content (one entry per line)
- **Historical accuracy** - Add period-appropriate names for different eras
- **Infinite variety** - More entries = more unique combinations
- **Comments supported** - Lines starting with `#` are ignored

Example song generation: "Electric" + "Dreams" = "Electric Dreams"
