use ratatui::crossterm::event::{KeyCode, KeyEvent};

use crate::game::music::ReleaseType;
use crate::game::{GameAction, PRESSING_TIERS};
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
                let (country_key, _, region_name, _, _, fame_req) = &sorted_regions[selected];
                if self.game.band.fame < *fame_req {
                    self.push_log(
                        LogKind::Error,
                        format!(
                            "❌ Your band needs at least {} fame to tour '{}'.",
                            fame_req, region_name
                        ),
                    );
                } else {
                    // Check if player can afford the tour cost to give clear feedback
                    let tier_name = if self.game.band.fame < 35 {
                        "local"
                    } else if self.game.band.fame < 60 {
                        "regional"
                    } else if self.game.band.fame < 80 {
                        "national"
                    } else {
                        "international"
                    };
                    let country_travel_mult = match country_key.as_str() {
                        "united_states" => 1.5,
                        "united_kingdom" => 0.8,
                        "europe" => 1.2,
                        "japan" => 1.0,
                        "australia" => 1.4,
                        _ => 1.0,
                    };
                    if let Some(touring_costs) = self
                        .game
                        .data_files
                        .markets_data
                        .market_modifiers
                        .touring_costs
                        .get(tier_name)
                    {
                        let cost =
                            (touring_costs.base_cost_per_show as f32 * country_travel_mult) as i32;
                        if !self.game.player.can_afford(cost) {
                            self.push_log(
                                LogKind::Error,
                                format!("❌ You need ${} to finance this tour!", cost),
                            );
                            return;
                        }
                    }
                    self.screen = Screen::Main;
                    self.dispatch(GameAction::GoOnTour(selected));
                }
            }
            _ => {}
        }
    }
}
