mod data;
mod data_loader;
mod game;
mod ui;

use crossterm::{
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use std::io;

use game::Game;
use ui::terminal::TerminalUI;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Setup terminal
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen)?;

    // Create and run the game
    let result = run_game();

    // Cleanup terminal
    disable_raw_mode()?;
    execute!(stdout, LeaveAlternateScreen)?;

    result
}

fn run_game() -> Result<(), Box<dyn std::error::Error>> {
    // Try to create the game - this will validate data files
    let mut game = match Game::new() {
        Ok(game) => game,
        Err(e) => {
            println!("❌ Error starting game: {}", e);
            println!(
                "\n📁 Please ensure you have created the data/ directory with all required files:"
            );
            println!("   - data/song_adjectives.txt");
            println!("   - data/song_nouns.txt");
            println!("   - data/song_verbs.txt");
            println!("   - data/song_emotions.txt");
            println!("   - data/song_places.txt");
            println!("   - data/album_titles.txt");
            println!("   - data/band_names.txt");
            println!("   - data/band_member_names.txt");
            println!("   - data/venue_names.txt");
            println!("   - data/city_names.txt");
            println!("   - data/timeline.json");
            println!("   - data/record_labels.json");
            println!("   - data/markets.json");
            println!("\n📖 See the documentation for file format examples.");
            std::thread::sleep(std::time::Duration::from_secs(5));
            return Err(e);
        }
    };

    let mut ui = TerminalUI::new()?;

    // Show welcome screen
    ui.show_welcome()?;

    // Get player name and band name
    let player_name = ui.get_input("Enter your name: ")?;
    let band_name = ui.get_input("Enter your band name: ")?;

    game.initialize_player(&player_name, &band_name);

    // Main game loop
    loop {
        ui.clear_screen()?;
        ui.display_game_state(&game)?;

        let choice = ui.show_main_menu(&game)?;

        match game.process_turn(choice) {
            Ok(continue_game) => {
                if !continue_game {
                    ui.show_game_over(&game)?;
                    break;
                }
            }
            Err(e) => {
                ui.show_error(&format!("Error: {}", e))?;
            }
        }

        // Check win/lose conditions
        if game.is_game_over() {
            ui.show_game_over(&game)?;
            break;
        }
    }

    Ok(())
}
