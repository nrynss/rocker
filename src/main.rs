// Several data-driven systems (markets, tours, genre tracking) are scaffolded
// ahead of use — see "Planned Features" in the README.
#![allow(dead_code)]

mod data;
mod data_loader;
mod game;
mod ui;

use game::Game;
use ui::app::App;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Validate data files before touching the terminal so any error
    // message is printed to a normal, visible screen.
    let game = match Game::new() {
        Ok(game) => game,
        Err(e) => {
            eprintln!("❌ Error starting game: {}", e);
            eprintln!(
                "\n📁 Please ensure you have created the data/ directory with all required files:"
            );
            for file in [
                "song_adjectives.txt",
                "song_nouns.txt",
                "song_verbs.txt",
                "song_emotions.txt",
                "song_places.txt",
                "album_titles.txt",
                "band_names.txt",
                "band_member_names.txt",
                "venue_names.txt",
                "city_names.txt",
                "timeline.json",
                "record_labels.json",
                "markets.json",
            ] {
                eprintln!("   - data/{}", file);
            }
            eprintln!("\n📖 See the documentation for file format examples.");
            return Err(e);
        }
    };

    let mut terminal = ratatui::init();
    let result = App::new(game).run(&mut terminal);
    ratatui::restore();
    result
}
