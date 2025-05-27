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

- Rust 1.70+ installed ([Install Rust](https://rustup.rs/))
- Terminal/Command prompt

### Installation & Running

1. **Clone or create the project:**
   ```bash
   mkdir rocker
   cd rocker
   ```

2. **Copy the provided source files** into the following structure:
   ```
   rocker/
   ├── Cargo.toml
   ├── src/
   │   ├── main.rs
   │   ├── game/
   │   │   ├── mod.rs
   │   │   ├── player.rs
   │   │   ├── band.rs
   │   │   ├── music.rs
   │   │   ├── events.rs
   │   │   └── world.rs
   │   ├── ui/
   │   │   ├── mod.rs
   │   │   └── terminal.rs
   │   └── data/
   │       └── mod.rs
   └── README.md
   ```

3. **Build and run:**
   ```bash
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

- **Laze Around** - Recover energy and reduce stress
- **Write Songs** - Create material for future releases
- **Practice** - Improve band skill
- **Record Single** - Release a single (requires songs and $100)
- **Record Album** - Release an album (requires 8+ songs and $1000)
- **Play Gigs** - Earn money and gain fame
- **Take a Break** - Full health/energy recovery
- **Visit Doctor** - Restore health ($50)

### Win Conditions
- **Fame ≥ 90** AND **Albums ≥ 5** = YOU'RE A ROCKSTAR! 🌟

### Lose Conditions
- **Health ≤ 0** = Game Over (poor health)
- **Money < 0** AND **Fame < 10** = Game Over (broke and unknown)

## 🛠️ Technical Details

### Architecture
- **Modular design** with separate concerns
- **Cross-platform** terminal UI using `crossterm`
- **Serializable state** for future save/load functionality
- **Random events system** for dynamic gameplay
- **Market simulation** with economic cycles

### Key Dependencies
- `crossterm` - Cross-platform terminal manipulation
- `serde` + `serde_json` - Serialization for save/load
- `rand` - Random number generation
- `chrono` - Date/time handling

### Platform Support
✅ **Windows** - Native terminal support
✅ **macOS** - Native terminal support
✅ **Linux** - Native terminal support

## 🎨 Features

### Current Features (v0.2.0)
- ✅ **Historical timeline** - Accurate music industry evolution 1970-1990+
- ✅ **External data files** - Fully customizable names and content
- ✅ **Era-based mechanics** - Recording costs, trends, and market conditions
- ✅ **Generated content** - Procedural song titles, band names, venues
- ✅ Basic game loop and mechanics
- ✅ Player and band management
- ✅ Song writing and recording
- ✅ Random events system
- ✅ Market simulation
- ✅ Terminal-based UI with colors
- ✅ Health/energy/stress management

### Planned Features
- 🔄 **Tours and venues** - Multi-city tour management
- 🔄 **Record deals** - Contract negotiations and obligations
- 🔄 **Player choices in events** - Interactive decision making
- 🔄 **Save/Load game** - Persistent progress
- 🔄 **Multiple difficulty levels** - Easy to Rockstar mode
- 🔄 **Band member relationships** - Deeper social dynamics
- 🔄 **Music genres** - Style specialization
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
├── main.rs              # Entry point and terminal setup
├── game/
│   ├── mod.rs           # Core game state and logic
│   ├── player.rs        # Player stats and attributes
│   ├── band.rs          # Band members and dynamics
│   ├── music.rs         # Songs, albums, and music generation
│   ├── events.rs        # Random events and consequences
│   └── world.rs         # Market conditions and competing bands
├── ui/
│   ├── mod.rs           # UI module declaration
│   └── terminal.rs      # Terminal-based interface
└── data/
    └── mod.rs           # Configuration and game data
```

## 🤝 Contributing

This is a learning project, but contributions are welcome! Ideas for improvement:

- **Balance tweaking** - Game difficulty and progression curves
- **New events** - More variety in random events
- **UI improvements** - Better terminal graphics and layout
- **Additional platforms** - Web version, mobile adaptation
- **Localization** - Multiple language support

## 📜 License

Apache License 2.0

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

Example song generation: "Electric" + "Dreams" = "Electric Dreams"# 🎸 Rocker - Rock Star Management Simulator
