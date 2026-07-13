use ratatui::crossterm::event::{KeyCode, KeyEvent};

use crate::game::Game;
use crate::ui::app::{App, FileMode, LogKind, SAVE_FILE_DEFAULT, Screen};

impl App {
    pub(crate) fn handle_file_key(&mut self, key: KeyEvent) {
        let Screen::File { mode, input } = &mut self.screen else {
            return;
        };

        match key.code {
            KeyCode::Char(c) if input.len() < 40 => input.push(c),
            KeyCode::Backspace => {
                input.pop();
            }
            KeyCode::Esc => self.screen = Screen::Main,
            KeyCode::Enter => {
                let mode = *mode;
                let path = if input.trim().is_empty() {
                    SAVE_FILE_DEFAULT.to_string()
                } else {
                    input.trim().to_string()
                };
                self.screen = Screen::Main;
                match mode {
                    FileMode::Save => match self.game.save_game(&path) {
                        Ok(()) => self.push_log(LogKind::Ui, format!("💾 Game saved to {}.", path)),
                        Err(e) => self.push_log(LogKind::Error, format!("❌ Save failed: {}", e)),
                    },
                    FileMode::Load => match Game::load_game(&path) {
                        Ok(loaded) => {
                            self.game = loaded;
                            self.push_log(LogKind::Ui, format!("📂 Game loaded from {}.", path));
                            if self.game.is_game_over() {
                                self.screen = Screen::GameOver;
                            }
                        }
                        Err(e) => self.push_log(LogKind::Error, format!("❌ Load failed: {}", e)),
                    },
                }
            }
            _ => {}
        }
    }
}
