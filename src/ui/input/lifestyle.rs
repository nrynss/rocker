//! Lifestyle tier picker input (design §B). Opening the picker itself is
//! wired from the main menu (`MenuKind::Lifestyle` in `main.rs`).

use ratatui::crossterm::event::{KeyCode, KeyEvent};

use crate::game::GameAction;
use crate::game::player::LifestyleTier;
use crate::ui::app::{App, LogKind, Screen};

impl App {
    pub(crate) fn handle_lifestyle_picker_key(&mut self, key: KeyEvent) {
        let Screen::LifestylePicker { selected } = self.screen else {
            return;
        };
        let count = LifestyleTier::ALL.len();
        match key.code {
            KeyCode::Esc => self.screen = Screen::Main,
            KeyCode::Up | KeyCode::Char('k') => {
                self.screen = Screen::LifestylePicker {
                    selected: super::cycle_index(selected, count, false),
                };
            }
            KeyCode::Down | KeyCode::Char('j') => {
                self.screen = Screen::LifestylePicker {
                    selected: super::cycle_index(selected, count, true),
                };
            }
            KeyCode::Enter => {
                let tier = LifestyleTier::ALL[selected];
                if tier == self.game.player.lifestyle {
                    self.push_log(LogKind::Ui, "You already live there.");
                    return;
                }
                self.screen = Screen::Main;
                self.dispatch(GameAction::ChangeLifestyle(tier));
            }
            _ => {}
        }
    }
}
