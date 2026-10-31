use ratatui::crossterm::event::{KeyCode, KeyEvent};

use crate::data::format_money;
use crate::game::music::{DistributionChannel, ReleaseType};
use crate::game::studios::{self, RecordingStudio};
use crate::game::{Game, GameAction, PRESSING_TIERS, TourQuote, TourRig};
use crate::ui::app::{App, LogKind, Screen};

/// S-RECORD: the record action carries a song count and a studio (§A/§C).
///
/// S-UI (§H): the format → songs → studio picker lives in this file, below.
/// `default_record_action` survives as the *fallback* for the one-click path —
/// but the Record menu rows now open the picker, so a player who never opens it
/// is no longer silently denied the choice.
///
/// S-MASTER (§D): this books a session and nothing more. Pressing and
/// distribution have left it entirely — they are the vault's, below.
fn default_record_action(game: &Game, release_type: ReleaseType) -> GameAction {
    let songs = game
        .band
        .default_booking_songs(&release_type)
        .unwrap_or_else(|| studios::default_song_count(&release_type));
    let studio = RecordingStudio::default();
    match release_type {
        ReleaseType::Single => GameAction::RecordSingle { songs, studio },
        ReleaseType::Album => GameAction::RecordAlbum { songs, studio },
    }
}

impl App {
    /// Open the record picker (§H) on a format — what the Record menu rows now
    /// do. `book_recording` below is the no-choice path kept for callers that
    /// want the default booking without a screen.
    pub(crate) fn open_record_picker(&mut self, release_type: ReleaseType) {
        let songs = self
            .game
            .band
            .default_booking_songs(&release_type)
            .unwrap_or_else(|| studios::default_song_count(&release_type));
        self.screen = Screen::RecordPicker {
            release_type,
            songs,
            studio: RecordingStudio::default(),
        };
    }

    /// Book a session straight away with the default song count and room.
    /// Retained as the no-choice path; the picker is the player-facing route.
    #[allow(dead_code)]
    pub(crate) fn book_recording(&mut self, release_type: ReleaseType) {
        let action = default_record_action(&self.game, release_type);
        self.dispatch(action);
    }

    /// The vault (design §D/§H): the masters awaiting release, and where the
    /// Release action starts.
    pub(crate) fn handle_vault_key(&mut self, key: KeyEvent) {
        let Screen::Vault { selected } = self.screen else {
            return;
        };
        let count = self.game.vault().len();
        if count == 0 {
            self.screen = Screen::Main;
            return;
        }
        match key.code {
            KeyCode::Esc => self.screen = Screen::Main,
            KeyCode::Up | KeyCode::Char('k') => {
                self.screen = Screen::Vault {
                    selected: super::cycle_index(selected, count, false),
                };
            }
            KeyCode::Down | KeyCode::Char('j') => {
                self.screen = Screen::Vault {
                    selected: super::cycle_index(selected, count, true),
                };
            }
            KeyCode::Enter => self.open_release_picker(selected.min(count - 1)),
            _ => {}
        }
    }

    /// A signed band's label decides the run; an indie band picks one — and,
    /// while unsigned, a distribution channel alongside it (design §E-3, M6).
    /// Since §D this is the *release* decision, taken on a master that is
    /// already finished and paid for.
    pub(crate) fn open_release_picker(&mut self, master: usize) {
        if self.game.band.current_deal().is_some() {
            self.screen = Screen::Main;
            self.dispatch(GameAction::ReleaseMaster {
                master,
                pressing: None,
            });
        } else {
            self.screen = Screen::PressingPicker {
                master,
                selected: 0,
                channel: self.game.current_distribution_channel,
            };
        }
    }

    pub(crate) fn handle_pressing_picker_key(&mut self, key: KeyEvent) {
        let Screen::PressingPicker {
            master,
            selected,
            channel,
        } = self.screen
        else {
            return;
        };
        let count = PRESSING_TIERS.len();
        match key.code {
            KeyCode::Esc => self.screen = Screen::Vault { selected: master },
            KeyCode::Up | KeyCode::Char('k') => {
                self.screen = Screen::PressingPicker {
                    master,
                    selected: super::cycle_index(selected, count, false),
                    channel,
                };
            }
            KeyCode::Down | KeyCode::Char('j') => {
                self.screen = Screen::PressingPicker {
                    master,
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
                    master,
                    selected,
                    channel: DistributionChannel::ALL[idx],
                };
            }
            KeyCode::Right | KeyCode::Char('l') => {
                let idx =
                    super::cycle_index(channel.ordinal(), DistributionChannel::ALL.len(), true);
                self.screen = Screen::PressingPicker {
                    master,
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
                // Persist the choice (M6): read by `action_release_master` to
                // charge this release's fee and stamp it onto the new
                // `Release`, and remembered as the default next time this
                // picker opens.
                self.game.current_distribution_channel = channel;
                self.screen = Screen::Main;
                self.dispatch(GameAction::ReleaseMaster {
                    master,
                    pressing: Some(selected),
                });
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

impl App {
    /// The record picker (§H): songs on ↑↓, studio on ←→, Enter books, Esc
    /// backs out. Every move recomputes the quote through
    /// `Game::quote_recording` — the same deterministic core the booking
    /// charges from — so what the player is shown cannot drift from what they
    /// are charged.
    ///
    /// Errors are *not* pushed to the log on navigation: a picker the player is
    /// still dialling in should not shout at them for an unaffordable room. The
    /// live quote panel shows the shortfall instead, and Enter is the only thing
    /// that reports (matching the tour booking picker's gate).
    pub(crate) fn handle_record_picker_key(&mut self, key: KeyEvent) {
        let Screen::RecordPicker {
            release_type,
            songs,
            studio,
        } = self.screen
        else {
            return;
        };
        let (min_songs, max_songs) = studios::song_count_bounds(&release_type);

        match key.code {
            KeyCode::Esc => self.screen = Screen::Main,
            // Songs on the vertical axis: a bounded spinner (§H).
            KeyCode::Up | KeyCode::Char('k') => {
                let next = if songs >= max_songs {
                    min_songs
                } else {
                    songs + 1
                };
                self.screen = Screen::RecordPicker {
                    release_type,
                    songs: next,
                    studio,
                };
            }
            KeyCode::Down | KeyCode::Char('j') => {
                let next = if songs <= min_songs {
                    max_songs
                } else {
                    songs - 1
                };
                self.screen = Screen::RecordPicker {
                    release_type,
                    songs: next,
                    studio,
                };
            }
            // Studio on the horizontal axis, the two-axis convention the tour
            // booking and pressing pickers already use.
            KeyCode::Left | KeyCode::Char('h') => {
                let idx = super::cycle_index(studio.ordinal(), RecordingStudio::ALL.len(), false);
                self.screen = Screen::RecordPicker {
                    release_type,
                    songs,
                    studio: RecordingStudio::ALL[idx],
                };
            }
            KeyCode::Right | KeyCode::Char('l') => {
                let idx = super::cycle_index(studio.ordinal(), RecordingStudio::ALL.len(), true);
                self.screen = Screen::RecordPicker {
                    release_type,
                    songs,
                    studio: RecordingStudio::ALL[idx],
                };
            }
            KeyCode::Enter => match self.game.quote_recording(release_type, songs, studio) {
                Ok(quote) => {
                    if self.game.player.can_afford(quote.cost) {
                        self.screen = Screen::Main;
                        self.dispatch(match release_type {
                            ReleaseType::Single => GameAction::RecordSingle { songs, studio },
                            ReleaseType::Album => GameAction::RecordAlbum { songs, studio },
                        });
                    } else {
                        self.push_log(
                            LogKind::Error,
                            format!(
                                "❌ That session costs {} — you have {}.",
                                format_money(quote.cost),
                                format_money(self.game.player.money)
                            ),
                        );
                    }
                }
                // The engine names the shortfall (too few songs, fame-gated
                // room) better than the UI can — surface it verbatim.
                Err(msg) => self.push_log(LogKind::Error, format!("❌ {msg}")),
            },
            _ => {}
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ratatui::crossterm::event::KeyModifiers;

    fn key(code: KeyCode) -> KeyEvent {
        KeyEvent::new(code, KeyModifiers::empty())
    }

    fn app_with_songs(songs: usize) -> App {
        let mut app = App::new(Game::new().expect("data files present"));
        app.game.initialize_player(
            "Ray",
            "The Rayguns",
            crate::game::genre::MusicGenre::ALL[0].clone(),
        );
        for i in 0..songs {
            app.game
                .band
                .unreleased_songs
                .push(crate::game::music::Song {
                    id: i as u32,
                    name: format!("Song {i}"),
                    songwriting_quality: 50,
                });
        }
        app
    }

    #[test]
    fn record_menu_row_opens_the_picker_instead_of_booking() {
        let mut app = app_with_songs(3);
        app.screen = Screen::Main;

        // The bug S-UI exists to fix: this used to dispatch the recording
        // action on the spot, in the default room, with the default song count.
        app.open_record_picker(ReleaseType::Single);

        match app.screen {
            Screen::RecordPicker {
                release_type,
                songs,
                studio,
            } => {
                assert_eq!(release_type, ReleaseType::Single);
                assert_eq!(songs, studios::default_song_count(&ReleaseType::Single));
                assert_eq!(studio, RecordingStudio::default());
            }
            _ => panic!("Record Single must open the picker, not book a session"),
        }
        // Nothing was booked: no session, no master, no money spent.
        assert!(app.game.ongoing_activity.is_none());
        assert!(app.game.vault().is_empty(), "no record was cut");
        assert_eq!(
            app.game.player.money, 1_500,
            "the starting purse must be untouched — opening a picker books nothing"
        );
    }

    #[test]
    fn songs_spin_within_the_format_bounds_and_wrap() {
        let mut app = app_with_songs(14);
        let (min, max) = studios::song_count_bounds(&ReleaseType::Album);
        app.screen = Screen::RecordPicker {
            release_type: ReleaseType::Album,
            songs: min,
            studio: RecordingStudio::DemoRoom,
        };

        // Down from the minimum wraps to the maximum — a bounded spinner.
        app.handle_record_picker_key(key(KeyCode::Down));
        assert!(
            matches!(app.screen, Screen::RecordPicker { songs, .. } if songs == max),
            "spin down at the floor should wrap to the ceiling ({max})"
        );

        // And up from the maximum wraps back to the minimum.
        app.handle_record_picker_key(key(KeyCode::Up));
        assert!(
            matches!(app.screen, Screen::RecordPicker { songs, .. } if songs == min),
            "spin up at the ceiling should wrap to the floor ({min})"
        );
    }

    #[test]
    fn studio_moves_along_the_five_room_ladder() {
        let mut app = app_with_songs(3);
        app.screen = Screen::RecordPicker {
            release_type: ReleaseType::Single,
            songs: 2,
            studio: RecordingStudio::Home,
        };

        app.handle_record_picker_key(key(KeyCode::Right));
        assert!(matches!(
            app.screen,
            Screen::RecordPicker {
                studio: RecordingStudio::DemoRoom,
                ..
            }
        ));

        app.handle_record_picker_key(key(KeyCode::Left));
        assert!(matches!(
            app.screen,
            Screen::RecordPicker {
                studio: RecordingStudio::Home,
                ..
            }
        ));
    }

    #[test]
    fn the_quote_on_screen_is_the_bill_that_gets_charged() {
        let mut app = app_with_songs(3);
        // Money enough for the room, so Enter actually books.
        app.game.player.money = 10_000;
        app.screen = Screen::RecordPicker {
            release_type: ReleaseType::Single,
            songs: 2,
            studio: RecordingStudio::DemoRoom,
        };

        let quoted = app
            .game
            .quote_recording(ReleaseType::Single, 2, RecordingStudio::DemoRoom)
            .expect("a two-song 45 in the demo room is quotable");
        let before = app.game.player.money;

        app.handle_record_picker_key(key(KeyCode::Enter));

        // A one-week session resolves on the booking turn itself, so the proof
        // that it was booked is the master in the vault, not an activity.
        assert!(
            !app.game.vault().is_empty(),
            "Enter on an affordable, valid quote must cut the record"
        );
        assert_eq!(
            app.game.player.money,
            before - quoted.cost,
            "the player must be charged exactly what the quote showed"
        );
    }

    #[test]
    fn enter_refuses_when_the_band_lacks_the_songs() {
        let mut app = app_with_songs(0);
        app.game.player.money = 10_000;
        app.screen = Screen::RecordPicker {
            release_type: ReleaseType::Single,
            songs: 2,
            studio: RecordingStudio::Home,
        };

        app.handle_record_picker_key(key(KeyCode::Enter));

        assert!(
            app.game.ongoing_activity.is_none(),
            "a band with no songs must not book a session"
        );
        assert!(
            matches!(app.screen, Screen::RecordPicker { .. }),
            "the picker should stay open so the player can fix the choice"
        );
    }

    #[test]
    fn esc_leaves_the_picker_without_booking() {
        let mut app = app_with_songs(3);
        app.screen = Screen::RecordPicker {
            release_type: ReleaseType::Single,
            songs: 2,
            studio: RecordingStudio::Home,
        };

        app.handle_record_picker_key(key(KeyCode::Esc));

        assert!(matches!(app.screen, Screen::Main));
        assert!(app.game.ongoing_activity.is_none());
    }
}
