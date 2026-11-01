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
                // X abandons a running commitment (design §H, S-UI). It is not
                // a menu row because it only exists while an activity runs, and
                // the activity strip advertises it in place of a menu entry.
                if c == 'x' && self.game.ongoing_activity.is_some() {
                    self.dispatch(GameAction::AbandonActivity);
                    return;
                }
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
            // S-UI (§H): the Record rows open the format → songs → studio
            // picker. They used to book the default room and song count on the
            // spot, which made the whole studio ladder unreachable in play.
            MenuKind::RecordSingle => {
                self.open_record_picker(crate::game::music::ReleaseType::Single)
            }
            MenuKind::RecordAlbum => {
                self.open_record_picker(crate::game::music::ReleaseType::Album)
            }
            MenuKind::Vault => {
                if self.game.vault().is_empty() {
                    self.push_log(
                        crate::ui::app::LogKind::Ui,
                        "Nothing in the vault — book a session and finish a record first.",
                    );
                } else {
                    self.screen = Screen::Vault { selected: 0 };
                }
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
                    // Albums open first: it's the board a career is measured
                    // on, and it's the one a pre-0.9 save's history lives on.
                    format: crate::game::music::ReleaseType::Album,
                    scroll: 0,
                }
            }
            MenuKind::Discography => {
                self.screen = Screen::Discography {
                    scroll: 0,
                    format: None,
                }
            }
            MenuKind::Labels => self.screen = Screen::Labels { scroll: 0 },
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
            MenuKind::RePress => {
                if self.game.band.current_deal().is_some() {
                    self.push_log(
                        crate::ui::app::LogKind::Ui,
                        "Your label restocks the catalog automatically.",
                    );
                } else if self.game.repressable_releases().is_empty() {
                    self.push_log(
                        crate::ui::app::LogKind::Ui,
                        "Nothing is sold out or low on stock right now.",
                    );
                } else {
                    self.screen = Screen::RePressPicker { selected: 0 };
                }
            }
            MenuKind::Quit => self.dispatch(GameAction::Quit),
        }
    }

    pub(crate) fn handle_charts_key(&mut self, key: KeyEvent) {
        let Screen::Charts {
            region,
            format,
            scroll,
        } = self.screen
        else {
            return;
        };
        match key.code {
            KeyCode::Esc => self.screen = Screen::Main,
            // ←/→ cycle Local → UK → Europe → America → Japan → Worldwide.
            KeyCode::Left | KeyCode::Char('h') => {
                self.screen = Screen::Charts {
                    region: region.prev_tab(),
                    format,
                    scroll: 0,
                };
            }
            KeyCode::Right | KeyCode::Char('l') => {
                self.screen = Screen::Charts {
                    region: region.next_tab(),
                    format,
                    scroll: 0,
                };
            }
            // Tab flips the format axis (§E). Two boards, so one key both
            // ways — the same "one key, the other view" shape the deals
            // modal's detail toggle uses.
            KeyCode::Tab | KeyCode::BackTab => {
                self.screen = Screen::Charts {
                    region,
                    format: match format {
                        crate::game::music::ReleaseType::Single => {
                            crate::game::music::ReleaseType::Album
                        }
                        crate::game::music::ReleaseType::Album => {
                            crate::game::music::ReleaseType::Single
                        }
                    },
                    // A different board is a different list; start at its top
                    // rather than stranding the view past its end.
                    scroll: 0,
                };
            }
            KeyCode::Up | KeyCode::Char('k') => {
                self.screen = Screen::Charts {
                    region,
                    format,
                    scroll: scroll.saturating_sub(1),
                };
            }
            KeyCode::Down | KeyCode::Char('j') => {
                let count = self.charts_region_entries(region, format).len();
                let max_scroll = count.saturating_sub(1);
                self.screen = Screen::Charts {
                    region,
                    format,
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

    pub(crate) fn handle_discography_key(&mut self, key: KeyEvent) {
        let Screen::Discography { scroll, format } = self.screen else {
            return;
        };
        let count = self.discography_rows(format).len();
        match key.code {
            KeyCode::Esc => self.screen = Screen::Main,
            // Tab cycles the §H split: everything → singles → albums.
            KeyCode::Tab => {
                let next = match format {
                    None => Some(crate::game::music::ReleaseType::Single),
                    Some(crate::game::music::ReleaseType::Single) => {
                        Some(crate::game::music::ReleaseType::Album)
                    }
                    Some(crate::game::music::ReleaseType::Album) => None,
                };
                self.screen = Screen::Discography {
                    scroll: 0,
                    format: next,
                };
            }
            KeyCode::Up | KeyCode::Char('k') => {
                self.screen = Screen::Discography {
                    scroll: scroll.saturating_sub(1),
                    format,
                };
            }
            KeyCode::Down | KeyCode::Char('j') => {
                let max_scroll = count.saturating_sub(1);
                self.screen = Screen::Discography {
                    scroll: (scroll + 1).min(max_scroll),
                    format,
                };
            }
            _ => {}
        }
    }

    pub(crate) fn handle_labels_key(&mut self, key: KeyEvent) {
        let Screen::Labels { scroll } = self.screen else {
            return;
        };
        let count = self.game.world.labels.len();
        match key.code {
            KeyCode::Esc => self.screen = Screen::Main,
            KeyCode::Up | KeyCode::Char('k') => {
                self.screen = Screen::Labels {
                    scroll: scroll.saturating_sub(1),
                };
            }
            KeyCode::Down | KeyCode::Char('j') => {
                let max_scroll = count.saturating_sub(1);
                self.screen = Screen::Labels {
                    scroll: (scroll + 1).min(max_scroll),
                };
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
    #[test]
    fn discography_tab_cycles_the_format_split() {
        use crate::game::music::ReleaseType;
        let mut app = App::new(Game::new().expect("data files present"));
        app.game.initialize_player(
            "Ray",
            "The Rayguns",
            crate::game::genre::MusicGenre::ALL[0].clone(),
        );
        app.screen = Screen::Discography {
            scroll: 0,
            format: None,
        };

        // all -> singles -> albums -> all, resetting the scroll each time so a
        // filter switch never leaves the view scrolled past the shorter list.
        for expected in [Some(ReleaseType::Single), Some(ReleaseType::Album), None] {
            app.handle_discography_key(key(KeyCode::Tab));
            assert!(
                matches!(app.screen, Screen::Discography { format, scroll: 0 } if format == expected),
                "Tab should cycle the discography to {expected:?}"
            );
        }
    }

    #[test]
    fn x_abandons_a_running_activity_but_is_inert_without_one() {
        let mut app = App::new(Game::new().expect("data files present"));
        app.game.initialize_player(
            "Ray",
            "The Rayguns",
            crate::game::genre::MusicGenre::ALL[0].clone(),
        );
        app.screen = Screen::Main;

        // With nothing running, `x` must not be swallowed as an unknown
        // hotkey error or start anything.
        app.handle_main_key(key(KeyCode::Char('x')));
        assert!(app.game.ongoing_activity.is_none());

        // Start a break, then abandon it with `x`.
        app.dispatch(crate::game::GameAction::TakeBreak);
        assert!(
            app.game.ongoing_activity.is_some(),
            "a break should be a multi-week commitment"
        );
        app.screen = Screen::Main;
        app.handle_main_key(key(KeyCode::Char('x')));
        assert!(
            app.game.ongoing_activity.is_none(),
            "'x' on Main should abandon the running activity"
        );
    }
    /// Only *turn-consuming* menu rows advance a running activity.
    ///
    /// This pins the distinction the activity strip describes. Charts,
    /// Discography, Save and Load open a screen and dispatch nothing, so a
    /// player who ducks into the charts mid-tour must come back to the same
    /// week — and a claim that "any menu action plays the next week" is false
    /// precisely because of them.
    #[test]
    fn screen_opening_rows_do_not_advance_a_running_activity() {
        let mut app = App::new(Game::new().expect("data files present"));
        app.game.initialize_player(
            "Ray",
            "The Rayguns",
            crate::game::genre::MusicGenre::ALL[0].clone(),
        );
        app.screen = Screen::Main;
        app.dispatch(crate::game::GameAction::TakeBreak);
        assert!(
            app.game.ongoing_activity.is_some(),
            "a break is a commitment"
        );

        let week = app.game.week;
        let done = app
            .game
            .ongoing_activity
            .as_ref()
            .map(crate::game::OngoingActivity::weeks_done)
            .expect("activity is running");

        // Every screen-opening row, in turn: none may move the calendar.
        for kind in [
            MenuKind::Charts,
            MenuKind::Discography,
            MenuKind::Save,
            MenuKind::Load,
        ] {
            app.screen = Screen::Main;
            app.activate(kind);
            assert_eq!(
                app.game.week, week,
                "opening a screen must not consume the week"
            );
            assert_eq!(
                app.game
                    .ongoing_activity
                    .as_ref()
                    .map(crate::game::OngoingActivity::weeks_done),
                Some(done),
                "opening a screen must not advance the activity"
            );
        }
    }
}
