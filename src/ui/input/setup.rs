use ratatui::crossterm::event::{KeyCode, KeyEvent};

use crate::data::constants;
use crate::game::genre::MusicGenre;
use crate::ui::app::{App, LogKind, Screen, SetupField};

impl App {
    pub(crate) fn handle_setup_key(&mut self, key: KeyEvent) {
        let Screen::Setup { field } = self.screen else {
            return;
        };

        if key.code == KeyCode::Esc {
            self.should_exit = true;
            return;
        }

        match field {
            SetupField::Name | SetupField::BandName => {
                let input = match field {
                    SetupField::Name => &mut self.name_input,
                    _ => &mut self.band_input,
                };
                match key.code {
                    KeyCode::Char(c) if input.len() < 24 => input.push(c),
                    KeyCode::Backspace => {
                        input.pop();
                    }
                    KeyCode::Enter if !input.trim().is_empty() => {
                        let next = if field == SetupField::Name {
                            SetupField::BandName
                        } else {
                            SetupField::Genre
                        };
                        self.screen = Screen::Setup { field: next };
                    }
                    _ => {}
                }
            }
            SetupField::Genre => {
                let count = MusicGenre::ALL.len();
                match key.code {
                    KeyCode::Up | KeyCode::Char('k') => {
                        self.genre_selected = super::cycle_index(self.genre_selected, count, false);
                    }
                    KeyCode::Down | KeyCode::Char('j') => {
                        self.genre_selected = super::cycle_index(self.genre_selected, count, true);
                    }
                    KeyCode::Enter => self.finish_setup(),
                    _ => {}
                }
            }
        }
    }

    /// Hand the chosen identity to the game and start playing.
    pub(crate) fn finish_setup(&mut self) {
        let name = self.name_input.trim().to_string();
        let band = self.band_input.trim().to_string();
        let genre = MusicGenre::ALL[self.genre_selected.min(MusicGenre::ALL.len() - 1)].clone();
        let genre_name = genre.name();
        self.game.initialize_player(&name, &band, genre);
        self.push_log(
            LogKind::Ui,
            format!(
                "Welcome to {}, {}. Make '{}' the biggest name in {}.",
                constants::STARTING_YEAR,
                name,
                band,
                genre_name
            ),
        );
        self.push_log(
            LogKind::Ui,
            "Tip: hotkeys act instantly — V reviews deal offers, M runs marketing.",
        );
        self.screen = Screen::Main;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::game::Game;
    use ratatui::crossterm::event::KeyModifiers;

    fn press(app: &mut App, code: KeyCode) {
        app.handle_setup_key(KeyEvent::new(code, KeyModifiers::empty()));
    }

    fn type_text(app: &mut App, text: &str) {
        for c in text.chars() {
            press(app, KeyCode::Char(c));
        }
    }

    #[test]
    fn setup_can_found_a_band_in_every_genre() {
        for (index, genre) in MusicGenre::ALL.iter().enumerate() {
            let mut app = App::new(Game::new().expect("data files present"));

            type_text(&mut app, "Ray");
            press(&mut app, KeyCode::Enter);
            type_text(&mut app, "The Rayguns");
            press(&mut app, KeyCode::Enter);
            for _ in 0..index {
                press(&mut app, KeyCode::Down);
            }
            press(&mut app, KeyCode::Enter);

            assert!(
                matches!(app.screen, Screen::Main),
                "setup should end on the main screen"
            );
            assert_eq!(app.game.band.genre, *genre);
            assert_eq!(app.game.band.name, "The Rayguns");
        }
    }
}
