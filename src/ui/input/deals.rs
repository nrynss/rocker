use ratatui::crossterm::event::{KeyCode, KeyEvent};

use crate::game::GameAction;
use crate::ui::app::{App, Screen};

impl App {
    pub(crate) fn handle_deals_key(&mut self, key: KeyEvent) {
        let Screen::Deals { selected, detail } = self.screen else {
            return;
        };
        let count = self.game.pending_deal_offers.len();
        if count == 0 {
            self.screen = Screen::Main;
            return;
        }

        match key.code {
            KeyCode::Esc => {
                self.screen = if detail {
                    Screen::Deals {
                        selected,
                        detail: false,
                    }
                } else {
                    Screen::Main
                };
            }
            KeyCode::Up | KeyCode::Char('k') if !detail => {
                self.screen = Screen::Deals {
                    selected: super::cycle_index(selected, count, false),
                    detail,
                };
            }
            KeyCode::Down | KeyCode::Char('j') if !detail => {
                self.screen = Screen::Deals {
                    selected: super::cycle_index(selected, count, true),
                    detail,
                };
            }
            KeyCode::Enter if !detail => {
                self.screen = Screen::Deals {
                    selected,
                    detail: true,
                };
            }
            KeyCode::Char('a') | KeyCode::Char('A') => {
                self.screen = Screen::Main;
                self.dispatch(GameAction::AcceptDeal(selected));
            }
            KeyCode::Char('r') | KeyCode::Char('R') => {
                self.dispatch(GameAction::RejectDeal(selected));
                let remaining = self.game.pending_deal_offers.len();
                self.screen = if remaining == 0 {
                    Screen::Main
                } else {
                    Screen::Deals {
                        selected: selected.min(remaining - 1),
                        detail: false,
                    }
                };
            }
            _ => {}
        }
    }

    pub(crate) fn handle_support_offer_key(&mut self, key: KeyEvent) {
        if self.game.pending_support_offer.is_none() {
            self.screen = Screen::Main;
            return;
        }
        match key.code {
            KeyCode::Esc => self.screen = Screen::Main,
            KeyCode::Char('a') | KeyCode::Char('A') | KeyCode::Enter => {
                self.screen = Screen::Main;
                self.dispatch(GameAction::AcceptSupportTour);
            }
            KeyCode::Char('r') | KeyCode::Char('R') => {
                self.screen = Screen::Main;
                self.dispatch(GameAction::DeclineSupportTour);
            }
            _ => {}
        }
    }
}
