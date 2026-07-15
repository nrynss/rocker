use ratatui::crossterm::event::{KeyCode, KeyEvent};

use crate::data::format_money;
use crate::game::music::ReleaseType;
use crate::game::{GameAction, PRESSING_TIERS, TourQuote, TourRig};
use crate::ui::app::{App, LogKind, Screen};

impl App {
    /// A signed band's label decides the run; an indie band picks one.
    pub(crate) fn open_pressing_picker(&mut self, release_type: ReleaseType) {
        if self.game.band.current_deal().is_some() {
            let action = match release_type {
                ReleaseType::Single => GameAction::RecordSingle { pressing: None },
                ReleaseType::Album => GameAction::RecordAlbum { pressing: None },
            };
            self.dispatch(action);
        } else {
            self.screen = Screen::PressingPicker {
                release_type,
                selected: 0,
            };
        }
    }

    pub(crate) fn handle_pressing_picker_key(&mut self, key: KeyEvent) {
        let Screen::PressingPicker {
            release_type,
            selected,
        } = self.screen
        else {
            return;
        };
        let count = PRESSING_TIERS.len();
        match key.code {
            KeyCode::Esc => self.screen = Screen::Main,
            KeyCode::Up | KeyCode::Char('k') => {
                let selected = selected.checked_sub(1).unwrap_or(count - 1);
                self.screen = Screen::PressingPicker {
                    release_type,
                    selected,
                };
            }
            KeyCode::Down | KeyCode::Char('j') => {
                self.screen = Screen::PressingPicker {
                    release_type,
                    selected: (selected + 1) % count,
                };
            }
            KeyCode::Enter => {
                self.screen = Screen::Main;
                let action = match release_type {
                    ReleaseType::Single => GameAction::RecordSingle {
                        pressing: Some(selected),
                    },
                    ReleaseType::Album => GameAction::RecordAlbum {
                        pressing: Some(selected),
                    },
                };
                self.dispatch(action);
            }
            _ => {}
        }
    }

    pub(crate) fn handle_venue_picker_key(&mut self, key: KeyEvent) {
        let Screen::VenuePicker { selected } = self.screen else {
            return;
        };
        let count = self.game.world.venues.len();
        match key.code {
            KeyCode::Esc => self.screen = Screen::Main,
            KeyCode::Up | KeyCode::Char('k') => {
                let selected = selected.checked_sub(1).unwrap_or(count - 1);
                self.screen = Screen::VenuePicker { selected };
            }
            KeyCode::Down | KeyCode::Char('j') => {
                self.screen = Screen::VenuePicker {
                    selected: (selected + 1) % count,
                };
            }
            KeyCode::Enter => {
                let venue = &self.game.world.venues[selected];
                if venue.prestige > self.game.band.fame.saturating_add(20) {
                    self.push_log(
                        LogKind::Error,
                        format!(
                            "❌ '{}' is out of your league! Get more famous first.",
                            venue.name
                        ),
                    );
                } else {
                    self.screen = Screen::Main;
                    self.dispatch(GameAction::Gig(selected));
                }
            }
            _ => {}
        }
    }

    pub(crate) fn handle_region_picker_key(&mut self, key: KeyEvent) {
        let Screen::RegionPicker { selected } = self.screen else {
            return;
        };
        let sorted_regions = self.game.get_sorted_regions();
        let count = sorted_regions.len();
        match key.code {
            KeyCode::Esc => self.screen = Screen::Main,
            KeyCode::Up | KeyCode::Char('k') => {
                let selected = selected.checked_sub(1).unwrap_or(count - 1);
                self.screen = Screen::RegionPicker { selected };
            }
            KeyCode::Down | KeyCode::Char('j') => {
                self.screen = Screen::RegionPicker {
                    selected: (selected + 1) % count,
                };
            }
            KeyCode::Enter => {
                let (_, _, region_name, _, _, fame_req) = &sorted_regions[selected];
                if self.game.band.fame < *fame_req {
                    self.push_log(
                        LogKind::Error,
                        format!(
                            "❌ Your band needs at least {} fame to tour '{}'.",
                            fame_req, region_name
                        ),
                    );
                } else {
                    // The rig/length choice — and its quote — come next
                    // (design §A, M1): fame gates the region, never the cost.
                    self.screen = Screen::TourBookingPicker {
                        region_index: selected,
                        rig: TourRig::Van,
                        weeks: 1,
                    };
                }
            }
            _ => {}
        }
    }

    /// Rig + length picker (design §A, M1): navigates independently over
    /// rigs (↑↓) and weeks (←→), and shows the live quote via
    /// `draw_tour_booking_picker_modal`. Booking only dispatches once the
    /// quote resolves and the player can afford it — the gate check mirrors
    /// `Game::quote_tour`/`action_go_on_tour` so the error the player sees
    /// here always matches what booking would say.
    pub(crate) fn handle_tour_booking_picker_key(&mut self, key: KeyEvent) {
        let Screen::TourBookingPicker {
            region_index,
            rig,
            weeks,
        } = self.screen
        else {
            return;
        };

        match key.code {
            KeyCode::Esc => {
                self.screen = Screen::RegionPicker {
                    selected: region_index,
                }
            }
            KeyCode::Up | KeyCode::Char('k') => {
                let count = TourRig::ALL.len();
                let idx = rig.ordinal().checked_sub(1).unwrap_or(count - 1);
                self.screen = Screen::TourBookingPicker {
                    region_index,
                    rig: TourRig::ALL[idx],
                    weeks,
                };
            }
            KeyCode::Down | KeyCode::Char('j') => {
                let idx = (rig.ordinal() + 1) % TourRig::ALL.len();
                self.screen = Screen::TourBookingPicker {
                    region_index,
                    rig: TourRig::ALL[idx],
                    weeks,
                };
            }
            KeyCode::Left | KeyCode::Char('h') => {
                let new_weeks = if weeks <= 1 { 4 } else { weeks - 1 };
                self.screen = Screen::TourBookingPicker {
                    region_index,
                    rig,
                    weeks: new_weeks,
                };
            }
            KeyCode::Right | KeyCode::Char('l') => {
                let new_weeks = if weeks >= 4 { 1 } else { weeks + 1 };
                self.screen = Screen::TourBookingPicker {
                    region_index,
                    rig,
                    weeks: new_weeks,
                };
            }
            KeyCode::Enter => {
                let quote: Result<TourQuote, String> =
                    self.game.quote_tour(region_index, rig, weeks);
                match quote {
                    Ok(quote) => {
                        if self.game.player.can_afford(quote.cost) {
                            self.screen = Screen::Main;
                            self.dispatch(GameAction::GoOnTour(region_index, rig, weeks));
                        } else {
                            self.push_log(
                                LogKind::Error,
                                format!(
                                    "❌ You need {} to finance this tour!",
                                    format_money(quote.cost)
                                ),
                            );
                        }
                    }
                    Err(msg) => self.push_log(LogKind::Error, format!("❌ {msg}")),
                }
            }
            _ => {}
        }
    }
}
