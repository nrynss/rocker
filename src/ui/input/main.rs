use ratatui::crossterm::event::{KeyCode, KeyEvent};

use crate::game::GameAction;
use crate::ui::app::{App, LogKind, MenuKind, Screen};

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
                // X walks out of a running commitment (design §F/§H, S-UI). It
                // is not a menu row because it only exists while an activity
                // runs, and the activity strip advertises it in place of a menu
                // entry. §F says "one keypress, confirmed" (#41): a stray `x`
                // must not forfeit a session paid for in full, so this only
                // opens the confirmation; `handle_confirm_abandon_key`
                // dispatches.
                if c == 'x' && self.game.ongoing_activity.is_some() {
                    self.screen = Screen::ConfirmAbandon;
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
            // A gig and a tour are bookings too: while an activity runs
            // `process_turn` would drop them and play the activity's week
            // instead (§F), so their pickers refuse to open — the same guard,
            // and the same reason, as the Record rows' (`busy_with`).
            MenuKind::Gig => {
                if let Some(busy) = self.busy_with() {
                    self.push_log(
                        crate::ui::app::LogKind::Ui,
                        format!(
                            "You're in the middle of {busy} — no gigs until it's over \
                             (or press X to walk out)."
                        ),
                    );
                } else if self.game.player.stress >= crate::game::GIG_STRESS_GUARD {
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
                if let Some(busy) = self.busy_with() {
                    self.push_log(
                        crate::ui::app::LogKind::Ui,
                        format!(
                            "You're in the middle of {busy} — no new tour until it's over \
                             (or press X to walk out)."
                        ),
                    );
                } else if self.game.player.stress >= crate::game::TOUR_STRESS_GUARD {
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
            // Never on one keypress: `q` used to dispatch `GameAction::Quit`,
            // which ended the career (game over) with nothing saved. Now it
            // asks to save first and only then leaves — and leaving is just
            // closing the app, so the save resumes the career, mid-activity
            // included (§F: a save mid-activity resumes).
            MenuKind::Quit => self.screen = Screen::ConfirmQuit { error: None },
        }
    }

    /// The "walk out?" confirmation (§F, #41). `Y` abandons; `N` or `Esc`
    /// carries on and takes no week; anything else is ignored, so a second
    /// stray key cannot confirm by accident.
    pub(crate) fn handle_confirm_abandon_key(&mut self, key: KeyEvent) {
        if self.game.ongoing_activity.is_none() {
            // Nothing left to walk out of (it cannot settle under an open
            // modal, but a stale screen must never dispatch).
            self.screen = Screen::Main;
            return;
        }
        match key.code {
            KeyCode::Char('y') | KeyCode::Char('Y') => {
                self.screen = Screen::Main;
                self.dispatch(GameAction::AbandonActivity);
            }
            KeyCode::Char('n') | KeyCode::Char('N') | KeyCode::Esc => {
                self.screen = Screen::Main;
            }
            _ => {}
        }
    }

    /// The save-first quit prompt. `S` saves and quits, `C` saves and
    /// continues playing, `N`/`Esc` cancel. Any other key — a second `q`
    /// above all, and Enter or the `j`/`k` menu-navigation keys a player's
    /// fingers may still be on — is ignored, so a stray press can't save
    /// over a file or exit.
    pub(crate) fn handle_confirm_quit_key(&mut self, key: KeyEvent) {
        let quit = match key.code {
            KeyCode::Char('s') | KeyCode::Char('S') => true,
            KeyCode::Char('c') | KeyCode::Char('C') => false,
            KeyCode::Char('n') | KeyCode::Char('N') | KeyCode::Esc => {
                self.screen = Screen::Main;
                return;
            }
            _ => return,
        };
        let path = self.save_path.clone();
        match self.game.save_game(&path) {
            Ok(()) => {
                if quit {
                    self.exit_message = Some(format!(
                        "💾 Saved to {path}. Load it from the menu (L) to pick up where you left off."
                    ));
                    self.should_exit = true;
                } else {
                    self.screen = Screen::Main;
                    self.push_log(LogKind::Ui, format!("💾 Game saved to {path}."));
                }
            }
            Err(e) => {
                // Don't quit on a failed save: stay on the prompt with the
                // reason, so nothing is lost and the player chooses again.
                let message = format!("Save to {path} failed: {e}");
                self.push_log(LogKind::Error, format!("❌ {message}"));
                self.screen = Screen::ConfirmQuit {
                    error: Some(message),
                };
            }
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
    fn x_asks_before_abandoning_and_is_inert_without_an_activity() {
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
        assert!(
            matches!(app.screen, Screen::Main),
            "no confirmation without something to walk out of"
        );

        // Start a break.
        app.dispatch(crate::game::GameAction::TakeBreak);
        assert!(
            app.game.ongoing_activity.is_some(),
            "a break should be a multi-week commitment"
        );
        app.screen = Screen::Main;
        let week = app.game.week;
        let weeks_done = |app: &App| match app.game.ongoing_activity() {
            Some(crate::game::OngoingActivity::Break(rest)) => rest.weeks_done,
            other => panic!("expected the break to still be running, got {other:?}"),
        };
        let done = weeks_done(&app);

        // `x` alone only asks (§F "one keypress, confirmed", #41).
        app.handle_main_key(key(KeyCode::Char('x')));
        assert!(matches!(app.screen, Screen::ConfirmAbandon));
        assert_eq!(weeks_done(&app), done, "asking must not abandon");
        assert_eq!(app.game.week, week, "asking must not take a week");

        // A stray key — even a second `x`, or `q` — neither confirms nor quits.
        for stray in ['x', 'q', '1', ' '] {
            app.handle_key(key(KeyCode::Char(stray)));
            assert!(
                matches!(app.screen, Screen::ConfirmAbandon),
                "{stray:?} should leave the confirmation up"
            );
        }
        app.handle_key(key(KeyCode::Enter));
        assert!(matches!(app.screen, Screen::ConfirmAbandon));
        assert!(!app.should_exit);

        // `n` and Esc both carry on: back to Main, nothing forfeited, no week.
        for cancel in [KeyCode::Char('n'), KeyCode::Char('N'), KeyCode::Esc] {
            app.handle_main_key(key(KeyCode::Char('x')));
            assert!(matches!(app.screen, Screen::ConfirmAbandon));
            app.handle_key(key(cancel));
            assert!(
                matches!(app.screen, Screen::Main),
                "{cancel:?} should cancel"
            );
            assert_eq!(weeks_done(&app), done, "{cancel:?} must not abandon");
            assert_eq!(app.game.week, week, "{cancel:?} must not take a week");
        }

        // `x` then `y` walks out — and, being instant (#37), takes no week.
        app.handle_main_key(key(KeyCode::Char('x')));
        app.handle_key(key(KeyCode::Char('y')));
        assert!(
            app.game.ongoing_activity.is_none(),
            "'x' then 'y' should abandon the running activity"
        );
        assert!(matches!(app.screen, Screen::Main));
        assert_eq!(app.game.week, week, "walking out takes no week");
    }

    /// #41: mid-activity, every week-taking row says what pressing it does —
    /// plays the activity's next week — and nothing it won't do: no doctor's
    /// fee, no greying-out on money (continuing is always possible). Out of
    /// an activity the rows go back to their own details and gates.
    #[test]
    fn week_taking_rows_say_continue_while_an_activity_runs() {
        use crate::game::GameAction;
        let mut app = App::new(Game::new().expect("data files present"));
        app.game.initialize_player(
            "Ray",
            "The Rayguns",
            crate::game::genre::MusicGenre::ALL[0].clone(),
        );
        app.dispatch(GameAction::TakeBreak);
        assert!(app.game.ongoing_activity.is_some());
        // Broke: the doctor row used to grey out on money it won't charge.
        // Stressed: Write and Practice used to grey out as "too stressed".
        app.game.player.money = 0;
        app.game.player.stress = 95;

        let entries = app.menu_entries();
        let row = |hotkey: char| {
            entries
                .iter()
                .find(|entry| entry.hotkey == hotkey)
                .unwrap_or_else(|| panic!("menu has a {hotkey:?} row"))
        };
        for hotkey in ['1', '2', '3', '8', '9'] {
            let entry = row(hotkey);
            assert_eq!(
                entry.detail, "keep resting",
                "{} should say it continues the break",
                entry.label
            );
            assert!(entry.enabled, "{} should stay pressable", entry.label);
        }
        assert!(
            !row('9').detail.contains('$'),
            "the doctor must not quote a fee it won't charge mid-activity"
        );
        // Instant and screen rows keep their own details (#37 lets the
        // instant ones apply mid-activity, so their text is still true).
        for hotkey in ['l', 'v', 'c', 'q'] {
            assert!(
                row(hotkey).detail != "keep resting",
                "{hotkey:?} is not a continue key"
            );
        }

        // A row labelled "keep resting" really does just play the week: the
        // break advances, and the doctor neither treats nor refuses. Solvent
        // and healthy for the press, so no bankruptcy or emergency can end
        // the break under the test; read from the log rather than the money,
        // which a week's random events may move.
        app.game.player.money = 100_000;
        app.game.player.health = 100;
        app.game.player.stress = 0;
        let done = |app: &App| match app.game.ongoing_activity() {
            Some(crate::game::OngoingActivity::Break(rest)) => rest.weeks_done,
            other => panic!("expected the break to still be running, got {other:?}"),
        };
        let before = done(&app);
        app.log.clear();
        app.handle_main_key(key(KeyCode::Char('9')));
        assert_eq!(
            done(&app),
            before + 1,
            "the doctor row played the break's next week"
        );
        assert!(
            app.log
                .iter()
                .all(|line| !line.text.contains("doctor") && !line.text.contains('❌')),
            "no visit and no refusal mid-break: {:?}",
            app.log.iter().map(|line| &line.text).collect::<Vec<_>>()
        );

        // Free again: the doctor quotes its fee and gates on money.
        app.game.ongoing_activity = None;
        app.game.player.money = 0;
        let entries = app.menu_entries();
        let doctor = entries.iter().find(|entry| entry.hotkey == '9').unwrap();
        assert!(doctor.detail.contains('$'));
        assert!(!doctor.enabled, "a broke player can't afford the doctor");
    }

    /// If the activity ends some other way while the confirmation is up (it
    /// can't today, but the screen must not dispatch into nothing), the
    /// confirmation just closes.
    #[test]
    fn the_confirmation_closes_itself_when_nothing_is_running() {
        let mut app = App::new(Game::new().expect("data files present"));
        app.game.initialize_player(
            "Ray",
            "The Rayguns",
            crate::game::genre::MusicGenre::ALL[0].clone(),
        );
        app.screen = Screen::ConfirmAbandon;
        let week = app.game.week;
        app.handle_key(key(KeyCode::Char('y')));
        assert!(matches!(app.screen, Screen::Main));
        assert_eq!(
            app.game.week, week,
            "nothing to abandon, nothing dispatched"
        );
    }
    /// Play a Gig and Go on Tour open booking pickers too, and a gig or a
    /// tour submitted mid-activity is dropped by `process_turn` exactly like
    /// a recording (§F). Both rows must refuse to open while a commitment
    /// runs — without moving the calendar — and open again once it is over.
    #[test]
    fn gig_and_tour_pickers_refuse_to_open_while_an_activity_runs() {
        let mut app = App::new(Game::new().expect("data files present"));
        app.game.initialize_player(
            "Ray",
            "The Rayguns",
            crate::game::genre::MusicGenre::ALL[0].clone(),
        );
        // Clear every other gate, so only the activity can refuse.
        app.game.band.fame = 50;
        app.game.player.stress = 0;
        app.game.player.health = 100;
        app.screen = Screen::Main;
        app.dispatch(crate::game::GameAction::TakeBreak);
        assert!(
            app.game.ongoing_activity.is_some(),
            "a break is a commitment"
        );
        app.game.player.stress = 0;
        app.game.player.health = 100;

        let week = app.game.week;
        for (hotkey, what) in [('6', "no gigs"), ('7', "no new tour")] {
            app.screen = Screen::Main;
            app.log.clear();
            app.handle_main_key(key(KeyCode::Char(hotkey)));
            assert!(
                matches!(app.screen, Screen::Main),
                "'{hotkey}' must not open a picker mid-activity"
            );
            let entry = app.log.last().expect("the refusal is explained");
            assert!(
                entry.text.contains("middle of a break") && entry.text.contains(what),
                "'{hotkey}' refusal should name the activity: {}",
                entry.text
            );
            let entries = app.menu_entries();
            let row = entries
                .iter()
                .find(|e| e.hotkey == hotkey)
                .expect("row exists");
            assert!(!row.enabled, "'{hotkey}' row is greyed out mid-activity");
            assert_eq!(row.detail, "busy: break");
        }
        assert_eq!(app.game.week, week, "refusing must not consume the week");

        // Walk out (`x`, confirmed with `y`), and both pickers open again.
        app.handle_main_key(key(KeyCode::Char('x')));
        app.handle_key(key(KeyCode::Char('y')));
        assert!(app.game.ongoing_activity.is_none());
        app.game.player.stress = 0;
        app.game.player.health = 100;
        app.handle_main_key(key(KeyCode::Char('6')));
        assert!(matches!(app.screen, Screen::VenuePicker { .. }));
        app.screen = Screen::Main;
        app.handle_main_key(key(KeyCode::Char('7')));
        assert!(matches!(app.screen, Screen::RegionPicker { .. }));
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

    /// A scratch save path unique to this test process and `name`, cleared
    /// before use so a previous run can't make an unsaved file look saved.
    fn scratch_save(name: &str) -> String {
        let path = std::env::temp_dir().join(format!(
            "rocker-quit-test-{}-{name}.sav",
            std::process::id()
        ));
        let _ = std::fs::remove_file(&path);
        path.to_string_lossy().into_owned()
    }

    fn app_ready_to_quit(name: &str) -> App {
        let mut app = App::new(Game::new().expect("data files present"));
        app.game.initialize_player(
            "Ray",
            "The Rayguns",
            crate::game::genre::MusicGenre::ALL[0].clone(),
        );
        app.screen = Screen::Main;
        app.save_path = scratch_save(name);
        app
    }

    /// `q` (and Enter on the Quit row) only opens the prompt; stray keys —
    /// a second `q`, Enter, the menu's `j`/`k`, `y`, `x` — neither save nor
    /// exit; `n`/`N`/Esc close it with nothing saved and no week spent.
    #[test]
    fn q_opens_the_save_first_prompt_and_stray_keys_do_nothing() {
        let mut app = app_ready_to_quit("stray");
        let week = app.game.week;

        app.handle_main_key(key(KeyCode::Char('q')));
        assert!(matches!(app.screen, Screen::ConfirmQuit { error: None }));
        assert!(!app.should_exit, "one keypress must not quit");
        assert!(!app.game.is_game_over(), "nor end the career");

        for stray in [
            KeyCode::Char('q'),
            KeyCode::Char('Q'),
            KeyCode::Char('x'),
            KeyCode::Char('y'),
            KeyCode::Char('j'),
            KeyCode::Char('k'),
            KeyCode::Char('1'),
            KeyCode::Enter,
            KeyCode::Up,
        ] {
            app.handle_key(key(stray));
            assert!(
                matches!(app.screen, Screen::ConfirmQuit { error: None }),
                "{stray:?} should leave the prompt up"
            );
            assert!(!app.should_exit, "{stray:?} must not quit");
        }
        assert!(
            !std::path::Path::new(&app.save_path).exists(),
            "stray keys must not save"
        );

        for cancel in [KeyCode::Char('n'), KeyCode::Char('N'), KeyCode::Esc] {
            app.handle_main_key(key(KeyCode::Char('q')));
            app.handle_key(key(cancel));
            assert!(matches!(app.screen, Screen::Main), "{cancel:?} cancels");
            assert!(!app.should_exit);
            assert!(!std::path::Path::new(&app.save_path).exists());
        }
        assert_eq!(app.game.week, week, "the prompt never takes a week");

        // The Quit row by Enter goes to the same prompt.
        let quit_row = app
            .menu_entries()
            .iter()
            .position(|entry| entry.hotkey == 'q')
            .expect("menu has a Quit row");
        app.menu_selected = quit_row;
        app.handle_main_key(key(KeyCode::Enter));
        assert!(matches!(app.screen, Screen::ConfirmQuit { error: None }));
        assert!(!app.should_exit);
    }

    /// `C` saves and keeps playing: the file is a loadable copy of this
    /// career, the prompt closes, and the app stays up.
    #[test]
    fn c_saves_and_keeps_playing() {
        let mut app = app_ready_to_quit("continue");
        let week = app.game.week;
        app.handle_main_key(key(KeyCode::Char('q')));
        app.handle_key(key(KeyCode::Char('c')));
        assert!(matches!(app.screen, Screen::Main));
        assert!(!app.should_exit, "save and continue must not quit");
        assert!(app.exit_message.is_none());
        assert_eq!(app.game.week, week, "saving takes no week");
        let saved = Game::load_game(&app.save_path).expect("the save loads");
        assert_eq!(saved.band.name, "The Rayguns");
        assert_eq!(saved.week, week);
        assert!(!saved.is_game_over());
        assert!(app.log.iter().any(|line| line.text.contains("saved")));
        let _ = std::fs::remove_file(&app.save_path);
    }

    /// `S` saves and quits — the app exits with a receipt naming the file,
    /// and the career is *not* ended: the save resumes it.
    #[test]
    fn s_saves_and_quits_without_ending_the_career() {
        let mut app = app_ready_to_quit("quit");
        app.handle_main_key(key(KeyCode::Char('q')));
        app.handle_key(key(KeyCode::Char('S')));
        assert!(app.should_exit, "save and quit leaves the app");
        assert!(
            app.exit_message
                .as_deref()
                .is_some_and(|message| message.contains(&app.save_path)),
            "the receipt names the file: {:?}",
            app.exit_message
        );
        assert!(!app.game.is_game_over(), "quitting is not retiring");
        let saved = Game::load_game(&app.save_path).expect("the save loads");
        assert!(!saved.is_game_over(), "the saved career carries on");
        let _ = std::fs::remove_file(&app.save_path);
    }

    /// A failed save never quits: both options stay on the prompt with the
    /// reason shown and logged, and Esc still backs out.
    #[test]
    fn a_failed_save_shows_the_error_and_does_not_quit() {
        let mut app = app_ready_to_quit("unused");
        let missing_dir = std::env::temp_dir().join(format!(
            "rocker-quit-test-{}-no-such-dir",
            std::process::id()
        ));
        let _ = std::fs::remove_dir_all(&missing_dir);
        app.save_path = missing_dir
            .join("career.sav")
            .to_string_lossy()
            .into_owned();

        for option in ['s', 'c'] {
            app.log.clear();
            app.handle_main_key(key(KeyCode::Char('q')));
            app.handle_key(key(KeyCode::Char(option)));
            assert!(!app.should_exit, "{option:?}: a failed save must not quit");
            assert!(app.exit_message.is_none());
            match &app.screen {
                Screen::ConfirmQuit { error: Some(error) } => {
                    assert!(error.contains("failed"), "{error}")
                }
                _ => panic!("{option:?}: the prompt should stay up with the error"),
            }
            assert!(
                app.log.iter().any(|line| line.text.contains("failed")),
                "the failure is logged too"
            );
            app.handle_key(key(KeyCode::Esc));
            assert!(matches!(app.screen, Screen::Main));
        }
    }

    /// Mid-activity, `q` asks the same question, the walk-out box ignores
    /// `q` (and this prompt ignores `x`), and save-and-quit keeps the
    /// commitment — in memory and in the file — instead of abandoning it
    /// the way the old one-keypress `GameAction::Quit` did.
    #[test]
    fn quitting_mid_activity_saves_the_activity_to_resume() {
        let mut app = app_ready_to_quit("activity");
        app.dispatch(crate::game::GameAction::TakeBreak);
        app.screen = Screen::Main;
        let done = match app.game.ongoing_activity() {
            Some(crate::game::OngoingActivity::Break(rest)) => rest.weeks_done,
            other => panic!("expected a break, got {other:?}"),
        };

        // The two prompts don't leak into each other.
        app.handle_main_key(key(KeyCode::Char('x')));
        app.handle_key(key(KeyCode::Char('q')));
        assert!(matches!(app.screen, Screen::ConfirmAbandon));
        app.handle_key(key(KeyCode::Esc));
        app.handle_main_key(key(KeyCode::Char('q')));
        app.handle_key(key(KeyCode::Char('x')));
        assert!(matches!(app.screen, Screen::ConfirmQuit { error: None }));
        assert!(app.game.ongoing_activity().is_some(), "nothing walked out");

        app.handle_key(key(KeyCode::Char('s')));
        assert!(app.should_exit);
        assert!(
            app.game.ongoing_activity().is_some(),
            "quitting no longer abandons the activity"
        );
        let saved = Game::load_game(&app.save_path).expect("the save loads");
        match saved.ongoing_activity() {
            Some(crate::game::OngoingActivity::Break(rest)) => {
                assert_eq!(rest.weeks_done, done, "the break resumes where it was")
            }
            other => panic!("the save should hold the break, got {other:?}"),
        }
        let _ = std::fs::remove_file(&app.save_path);
    }

    /// The prompt writes where the player has been saving: a save through
    /// the Save modal moves `save_path`, and `C` then writes there.
    #[test]
    fn the_prompt_saves_to_the_last_file_used() {
        let mut app = app_ready_to_quit("default");
        let chosen = scratch_save("chosen");
        app.screen = Screen::File {
            mode: crate::ui::app::FileMode::Save,
            input: chosen.clone(),
        };
        app.handle_file_key(key(KeyCode::Enter));
        assert_eq!(app.save_path, chosen);
        std::fs::remove_file(&chosen).expect("the modal saved");

        app.handle_main_key(key(KeyCode::Char('q')));
        app.handle_key(key(KeyCode::Char('c')));
        assert!(
            std::path::Path::new(&chosen).exists(),
            "C wrote to the chosen file"
        );
        let _ = std::fs::remove_file(&chosen);
    }

    /// CodeRabbit on #52: a save that fails at the last step (here the
    /// target is a directory, so the rename over it fails) reports the error
    /// on the prompt, quits nothing, leaves what was at the target alone,
    /// and cleans up its temp file. (`game::tests::save_compat` covers a
    /// write dying mid-way over an existing save.)
    #[test]
    fn a_failed_quit_save_leaves_the_target_and_no_temp_file() {
        let dir = std::env::temp_dir().join(format!(
            "rocker-quit-test-{}-rename-fails",
            std::process::id()
        ));
        let _ = std::fs::remove_dir_all(&dir);
        let target = dir.join("career.sav");
        std::fs::create_dir_all(&target).expect("a directory squats on the target");
        std::fs::write(target.join("keep.txt"), b"untouched").expect("seed");

        let mut app = app_ready_to_quit("unused-rename");
        app.save_path = target.to_string_lossy().into_owned();
        app.handle_main_key(key(KeyCode::Char('q')));
        app.handle_key(key(KeyCode::Char('s')));
        assert!(!app.should_exit, "the failed save must not quit");
        assert!(matches!(app.screen, Screen::ConfirmQuit { error: Some(_) }));
        assert_eq!(
            std::fs::read(target.join("keep.txt")).expect("still there"),
            b"untouched"
        );
        let leftovers: Vec<_> = std::fs::read_dir(&dir)
            .expect("dir lists")
            .map(|entry| entry.expect("entry").file_name())
            .collect();
        assert_eq!(leftovers.len(), 1, "only the target remains: {leftovers:?}");
        let _ = std::fs::remove_dir_all(&dir);
    }
}
