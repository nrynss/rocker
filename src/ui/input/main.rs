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
            MenuKind::Charts => {
                self.screen = Screen::Charts {
                    region: crate::game::world::ChartRegion::Local,
                    scroll: 0,
                }
            }
            MenuKind::TourReport => self.screen = Screen::TourReport { scroll: 0 },
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
            MenuKind::Lifestyle => {
                let selected = crate::game::player::LifestyleTier::ALL
                    .iter()
                    .position(|&t| t == self.game.player.lifestyle)
                    .unwrap_or(0);
                self.screen = Screen::LifestylePicker { selected };
            }
            MenuKind::Quit => self.dispatch(GameAction::Quit),
        }
    }

    pub(crate) fn handle_charts_key(&mut self, key: KeyEvent) {
        let Screen::Charts { region, scroll } = self.screen else {
            return;
        };
        match key.code {
            KeyCode::Esc => self.screen = Screen::Main,
            // ←/→ cycle Local → UK → Europe → America → Japan → Worldwide.
            KeyCode::Left | KeyCode::Char('h') => {
                self.screen = Screen::Charts {
                    region: region.prev_tab(),
                    scroll: 0,
                };
            }
            KeyCode::Right | KeyCode::Char('l') => {
                self.screen = Screen::Charts {
                    region: region.next_tab(),
                    scroll: 0,
                };
            }
            KeyCode::Up | KeyCode::Char('k') => {
                self.screen = Screen::Charts {
                    region,
                    scroll: scroll.saturating_sub(1),
                };
            }
            KeyCode::Down | KeyCode::Char('j') => {
                let count = self.charts_region_entries(region).len();
                let max_scroll = count.saturating_sub(1);
                self.screen = Screen::Charts {
                    region,
                    scroll: (scroll + 1).min(max_scroll),
                };
            }
            _ => {}
        }
    }

    pub(crate) fn handle_tour_report_key(&mut self, key: KeyEvent) {
        let Screen::TourReport { scroll } = self.screen else {
            return;
        };
        let count = self
            .game
            .last_tour_report
            .as_ref()
            .map_or(0, |report| report.rows.len());
        match key.code {
            KeyCode::Esc => self.screen = Screen::Main,
            KeyCode::Up | KeyCode::Char('k') => {
                let scroll = scroll.saturating_sub(1);
                self.screen = Screen::TourReport { scroll };
            }
            KeyCode::Down | KeyCode::Char('j') => {
                let scroll = if count == 0 {
                    0
                } else {
                    (scroll + 1).min(count - 1)
                };
                self.screen = Screen::TourReport { scroll };
            }
            _ => {}
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::game::Game;
    use ratatui::crossterm::event::KeyModifiers;

    fn key(code: KeyCode) -> KeyEvent {
        KeyEvent::new(code, KeyModifiers::empty())
    }

    #[test]
    fn r_opens_the_tour_report_and_esc_returns_to_main() {
        let mut app = App::new(Game::new().expect("data files present"));
        app.screen = Screen::Main;

        app.handle_main_key(key(KeyCode::Char('r')));
        assert!(
            matches!(app.screen, Screen::TourReport { scroll: 0 }),
            "'r' on Main should open the tour report at the top"
        );

        app.handle_tour_report_key(key(KeyCode::Esc));
        assert!(
            matches!(app.screen, Screen::Main),
            "Esc from the tour report should return to Main"
        );
    }

    #[test]
    fn tour_report_scroll_is_bounded_by_row_count() {
        use crate::game::{ShowReport, TourReport};

        let mut app = App::new(Game::new().expect("data files present"));
        app.game.last_tour_report = Some(TourReport {
            rows: vec![
                ShowReport {
                    week: 1,
                    venue_name: "The Roxy (Springfield)".into(),
                    verdict: "great".into(),
                    reception: 80,
                    attendance: 400,
                    capacity: 500,
                    take: 1200,
                },
                ShowReport {
                    week: 1,
                    venue_name: "The Bowl (Shelbyville)".into(),
                    verdict: "solid".into(),
                    reception: 55,
                    attendance: 300,
                    capacity: 600,
                    take: 900,
                },
            ],
            avg_reception: 67,
            total_gross: 2100,
            fame_gained: 2,
        });
        app.screen = Screen::TourReport { scroll: 0 };

        // Scrolling down twice should clamp at the last row (index 1), not
        // panic or run past the end.
        app.handle_tour_report_key(key(KeyCode::Down));
        app.handle_tour_report_key(key(KeyCode::Down));
        assert!(matches!(app.screen, Screen::TourReport { scroll: 1 }));

        // Scrolling up from 0 stays at 0.
        app.screen = Screen::TourReport { scroll: 0 };
        app.handle_tour_report_key(key(KeyCode::Up));
        assert!(matches!(app.screen, Screen::TourReport { scroll: 0 }));
    }
}
