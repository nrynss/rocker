use ratatui::crossterm::event::{KeyCode, KeyEvent};

use crate::game::GameAction;
use crate::ui::app::{App, MenuKind, Screen};

impl App {
    pub(crate) fn handle_main_key(&mut self, key: KeyEvent) {
        let entries = self.menu_entries();
        match key.code {
            KeyCode::Up | KeyCode::Char('k') => {
                self.menu_selected = self
                    .menu_selected
                    .checked_sub(1)
                    .unwrap_or(entries.len() - 1);
            }
            KeyCode::Down | KeyCode::Char('j') => {
                self.menu_selected = (self.menu_selected + 1) % entries.len();
            }
            KeyCode::Enter => {
                let kind = entries[self.menu_selected].kind.clone();
                self.activate(kind);
            }
            KeyCode::Char(c) => {
                let c = c.to_ascii_lowercase();
                if let Some(entry) = entries.iter().find(|e| e.hotkey == c) {
                    let kind = entry.kind.clone();
                    self.activate(kind);
                }
            }
            _ => {}
        }
    }

    pub(crate) fn activate(&mut self, kind: MenuKind) {
        match kind {
            MenuKind::Action(action) => self.dispatch(action),
            MenuKind::RecordSingle => {
                self.open_pressing_picker(crate::game::music::ReleaseType::Single)
            }
            MenuKind::RecordAlbum => {
                self.open_pressing_picker(crate::game::music::ReleaseType::Album)
            }
            MenuKind::Deals => {
                if self.game.pending_deal_offers.is_empty() {
                    self.push_log(
                        crate::ui::app::LogKind::Ui,
                        "No deal offers on the table right now.",
                    );
                } else {
                    self.screen = Screen::Deals {
                        selected: 0,
                        detail: false,
                    };
                }
            }
            MenuKind::SupportTour => {
                if self.game.pending_support_offer.is_some() {
                    self.screen = Screen::SupportOffer;
                } else {
                    self.push_log(
                        crate::ui::app::LogKind::Ui,
                        "No support slots on offer — get noticed by the bigger acts first.",
                    );
                }
            }
            MenuKind::Charts => self.screen = Screen::Charts,
            MenuKind::Marketing => {
                let signed = self.game.band.current_deal().is_some();
                let targets = self.marketing_targets();
                if signed {
                    self.push_log(
                        crate::ui::app::LogKind::Ui,
                        "Promotion is your label's job — their people are already on it.",
                    );
                } else if targets.is_empty() {
                    self.push_log(
                        crate::ui::app::LogKind::Ui,
                        "Record something first — there's nothing to promote.",
                    );
                } else {
                    self.screen = Screen::MarketingRelease { selected: 0 };
                }
            }
            MenuKind::Gig => {
                if self.game.player.stress >= crate::game::GIG_STRESS_GUARD {
                    self.push_log(
                        crate::ui::app::LogKind::Ui,
                        "You're too stressed out to perform!",
                    );
                } else if self.game.player.health < crate::game::GIG_HEALTH_GUARD {
                    self.push_log(crate::ui::app::LogKind::Ui, "You're too unwell to perform!");
                } else {
                    self.screen = Screen::VenuePicker { selected: 0 };
                }
            }
            MenuKind::GoOnTour => {
                if self.game.player.stress >= crate::game::TOUR_STRESS_GUARD {
                    self.push_log(
                        crate::ui::app::LogKind::Ui,
                        "You're too stressed to go on tour!",
                    );
                } else if self.game.player.health < crate::game::TOUR_HEALTH_GUARD {
                    self.push_log(
                        crate::ui::app::LogKind::Ui,
                        "You're too unwell to go on tour!",
                    );
                } else if self.game.band.fame < 25 {
                    self.push_log(
                        crate::ui::app::LogKind::Ui,
                        "You need more fame before promoters will book a tour!",
                    );
                } else {
                    self.screen = Screen::RegionPicker { selected: 0 };
                }
            }
            MenuKind::Save => {
                self.screen = Screen::File {
                    mode: crate::ui::app::FileMode::Save,
                    input: String::new(),
                };
            }
            MenuKind::Load => {
                self.screen = Screen::File {
                    mode: crate::ui::app::FileMode::Load,
                    input: String::new(),
                };
            }
            MenuKind::Quit => self.dispatch(GameAction::Quit),
        }
    }

    pub(crate) fn handle_charts_key(&mut self, key: KeyEvent) {
        if key.code == KeyCode::Esc {
            self.screen = Screen::Main;
        }
    }
}
