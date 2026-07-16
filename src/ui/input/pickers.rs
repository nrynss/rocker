use ratatui::crossterm::event::{KeyCode, KeyEvent};

use crate::data::format_money;
use crate::game::music::{DistributionChannel, ReleaseType};
use crate::game::{GameAction, PRESSING_TIERS, TourQuote, TourRig};
use crate::ui::app::{App, LogKind, Screen};

impl App {
    /// A signed band's label decides the run; an indie band picks one — and,
    /// while unsigned, a distribution channel alongside it (design §E-3, M6).
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
                channel: self.game.current_distribution_channel,
            };
        }
    }

    pub(crate) fn handle_pressing_picker_key(&mut self, key: KeyEvent) {
        let Screen::PressingPicker {
            release_type,
            selected,
            channel,
        } = self.screen
        else {
            return;
        };
        let count = PRESSING_TIERS.len();
        match key.code {
            KeyCode::Esc => self.screen = Screen::Main,
            KeyCode::Up | KeyCode::Char('k') => {
                self.screen = Screen::PressingPicker {
                    release_type,
                    selected: super::cycle_index(selected, count, false),
                    channel,
                };
            }
            KeyCode::Down | KeyCode::Char('j') => {
                self.screen = Screen::PressingPicker {
                    release_type,
                    selected: super::cycle_index(selected, count, true),
                    channel,
                };
            }
            // M6 (§E-3): cycle the distribution channel independently of the
            // pressing tier, same left/right-for-the-second-axis convention
            // as the tour booking picker's rig/length split.
            KeyCode::Left | KeyCode::Char('h') => {
                let idx =
                    super::cycle_index(channel.ordinal(), DistributionChannel::ALL.len(), false);
                self.screen = Screen::PressingPicker {
                    release_type,
                    selected,
                    channel: DistributionChannel::ALL[idx],
                };
            }
            KeyCode::Right | KeyCode::Char('l') => {
                let idx =
                    super::cycle_index(channel.ordinal(), DistributionChannel::ALL.len(), true);
                self.screen = Screen::PressingPicker {
                    release_type,
                    selected,
                    channel: DistributionChannel::ALL[idx],
                };
            }
            KeyCode::Enter => {
                if !channel.is_available(self.game.band.fame) {
                    self.push_log(
                        LogKind::Error,
                        format!(
                            "❌ {} needs {} fame — you're not there yet.",
                            channel.label(),
                            channel.fame_gate()
                        ),
                    );
                    return;
                }
                // Persist the choice (M6): read by `action_record_single`/
                // `action_record_album` to charge this release's fee and
                // stamp it onto the new `Release`, and remembered as the
                // default next time this picker opens.
                self.game.current_distribution_channel = channel;
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

    /// Which sold-out/low-stock release to re-press (design §E-1 indie
    /// half, M6). Enter drills into the tier picker for that one release.
    pub(crate) fn handle_repress_picker_key(&mut self, key: KeyEvent) {
        let Screen::RePressPicker { selected } = self.screen else {
            return;
        };
        let releases = self.game.repressable_releases();
        let count = releases.len();
        if count == 0 {
            self.screen = Screen::Main;
            return;
        }
        match key.code {
            KeyCode::Esc => self.screen = Screen::Main,
            KeyCode::Up | KeyCode::Char('k') => {
                self.screen = Screen::RePressPicker {
                    selected: super::cycle_index(selected, count, false),
                };
            }
            KeyCode::Down | KeyCode::Char('j') => {
                self.screen = Screen::RePressPicker {
                    selected: super::cycle_index(selected, count, true),
                };
            }
            KeyCode::Enter => {
                let release_id = releases[selected.min(count - 1)].id;
                self.screen = Screen::RePressTierPicker {
                    release_id,
                    selected: 0,
                };
            }
            _ => {}
        }
    }

    /// The pressing tier for a re-press, once the release is chosen.
    pub(crate) fn handle_repress_tier_picker_key(&mut self, key: KeyEvent) {
        let Screen::RePressTierPicker {
            release_id,
            selected,
        } = self.screen
        else {
            return;
        };
        let count = PRESSING_TIERS.len();
        match key.code {
            KeyCode::Esc => self.screen = Screen::RePressPicker { selected: 0 },
            KeyCode::Up | KeyCode::Char('k') => {
                self.screen = Screen::RePressTierPicker {
                    release_id,
                    selected: super::cycle_index(selected, count, false),
                };
            }
            KeyCode::Down | KeyCode::Char('j') => {
                self.screen = Screen::RePressTierPicker {
                    release_id,
                    selected: super::cycle_index(selected, count, true),
                };
            }
            KeyCode::Enter => {
                self.screen = Screen::Main;
                self.dispatch(GameAction::RePress {
                    release_id,
                    pressing: Some(selected),
                });
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
                self.screen = Screen::VenuePicker {
                    selected: super::cycle_index(selected, count, false),
                };
            }
            KeyCode::Down | KeyCode::Char('j') => {
                self.screen = Screen::VenuePicker {
                    selected: super::cycle_index(selected, count, true),
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
                self.screen = Screen::RegionPicker {
                    selected: super::cycle_index(selected, count, false),
                };
            }
            KeyCode::Down | KeyCode::Char('j') => {
                self.screen = Screen::RegionPicker {
                    selected: super::cycle_index(selected, count, true),
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
                let idx = super::cycle_index(rig.ordinal(), TourRig::ALL.len(), false);
                self.screen = Screen::TourBookingPicker {
                    region_index,
                    rig: TourRig::ALL[idx],
                    weeks,
                };
            }
            KeyCode::Down | KeyCode::Char('j') => {
                let idx = super::cycle_index(rig.ordinal(), TourRig::ALL.len(), true);
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
