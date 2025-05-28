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

        match choice {
            game::GameAction::SaveGame => {
                let filename = ui.get_input("Enter filename to save (default: rocker.sav): ")?;
                let save_path = if filename.is_empty() { "rocker.sav" } else { &filename };
                match game.save_game(save_path) {
                    Ok(_) => ui.show_message(&format!("Game saved to {}", save_path))?,
                    Err(e) => ui.show_error(&format!("Failed to save game: {}", e))?,
                }
                // Does not consume a turn, continue loop
            }
            game::GameAction::LoadGame => {
                let filename = ui.get_input("Enter filename to load (default: rocker.sav): ")?;
                let load_path = if filename.is_empty() { "rocker.sav" } else { &filename };
                match Game::load_game(load_path) {
                    Ok(loaded_game) => {
                        game = loaded_game;
                        ui.show_message(&format!("Game loaded from {}", load_path))?;
                    }
                    Err(e) => ui.show_error(&format!("Failed to load game: {}", e))?,
                }
                // Does not consume a turn, continue loop to refresh screen
            }
            game::GameAction::ViewDealOffers => {
                if let Some(deal_choice_action) = ui.show_deal_offers_menu(&game)? {
                    // This action (AcceptDeal or RejectDeal) will be processed in the next loop iteration
                    // by the process_turn call if we let it fall through.
                    // Or, we can process it immediately. Let's process immediately for clarity.
                    match game.process_turn(deal_choice_action.clone()) {
                        Ok(true) => {
                            // Game state was updated (e.g., deal accepted/rejected)
                            // Check if the action was an accept/reject to show a specific message
                            match deal_choice_action {
                                game::GameAction::AcceptDeal(_) => {
                                    ui.show_message("Record deal accepted!")?;
                                    // Display the game state again to show the new deal and advance.
                                }
                                game::GameAction::RejectDeal(_) => {
                                    ui.show_message("Record deal rejected.")?;
                                }
                                _ => {} // Should not happen if show_deal_offers_menu returns correctly
                            }
                        }
                        Ok(false) => { // Game over, should not happen from Accept/Reject
                            ui.show_game_over(&game)?;
                            break;
                        }
                        Err(e) => {
                            ui.show_error(&format!("Error processing deal action: {}", e))?;
                        }
                    }
                }
                // ViewDealOffers itself (and subsequent A/R) does not consume a standard turn.
            }
            game::GameAction::AcceptDeal(index) => { // Should ideally be handled by ViewDealOffers logic if called directly
                match game.process_turn(game::GameAction::AcceptDeal(index)) {
                    Ok(true) => ui.show_message("Deal accepted! Your life changes now.")?,
                    Ok(false) => {  ui.show_game_over(&game)?; break; }
                    Err(e) => ui.show_error(&format!("Error accepting deal: {}", e))?,
                }
            }
            game::GameAction::RejectDeal(index) => { // Should ideally be handled by ViewDealOffers logic
                 match game.process_turn(game::GameAction::RejectDeal(index)) {
                    Ok(true) => ui.show_message("Deal rejected. Maybe next time.")?,
                    Ok(false) => {  ui.show_game_over(&game)?; break; }
                    Err(e) => ui.show_error(&format!("Error rejecting deal: {}", e))?,
                }
            }
            _ => { // Handles all other actions that consume a turn
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

                // Check win/lose conditions after a standard turn
                if game.is_game_over() {
                    ui.show_game_over(&game)?;
                    break;
                }
            }
        }
    }

    Ok(())
}
