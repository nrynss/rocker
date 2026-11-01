//! Keyboard input event handlers for each screen family, implementing `App` input helpers.

mod deals;
mod file;
mod lifestyle;
mod main;
mod marketing;
mod pickers;
mod setup;

use crate::ui::app::App;

impl App {
    /// The running commitment, named for a sentence ("a tour", "a break"), or
    /// `None` when the band is free.
    ///
    /// Every menu row that opens a *booking* picker — Record, Play a Gig, Go
    /// on Tour — asks this before opening, and so does the support offer's
    /// Accept. While an activity runs, `process_turn` lets *instant* actions
    /// through but ignores anything that takes a week and plays the
    /// activity's next week instead (design §F), so a booking picker opened
    /// mid-tour would quote a booking, take the player's Enter, then silently
    /// drop it and spend the week on the tour. Refusing at the door is the
    /// only honest answer; the row's hotkey and Enter both route through
    /// `activate`, so a guard there covers every entry point.
    fn busy_with(&self) -> Option<String> {
        self.game
            .ongoing_activity()
            .map(|activity| format!("a {}", activity.label().to_lowercase()))
    }
}

/// Cycle a 0-based selection index one step over `count` items, wrapping at
/// both ends: `forward` (Down/Right) advances, otherwise (Up/Left) it steps
/// back. Shared by every picker's ↑↓/←→ navigation. `count` must be non-zero,
/// which every caller guarantees (fixed-length tables, or an early return on
/// an empty list).
fn cycle_index(current: usize, count: usize, forward: bool) -> usize {
    if forward {
        (current + 1) % count
    } else {
        current.checked_sub(1).unwrap_or(count - 1)
    }
}

/// The instant rows, driven through the keyboard exactly as a player would,
/// while a commitment runs (design §F, PR #37). Each one must apply, take no
/// week, and leave the activity's progress alone; the booking rows beside
/// them must still refuse.
#[cfg(test)]
mod instant_rows_mid_activity {
    use ratatui::crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

    use crate::game::music::ReleaseType;
    use crate::game::player::LifestyleTier;
    use crate::game::studios::RecordingStudio;
    use crate::game::world::PotentialDealOffer;
    use crate::game::{Game, GameAction, OngoingActivity, SupportTourOffer};
    use crate::ui::app::{App, MenuKind, Screen};

    fn key(code: KeyCode) -> KeyEvent {
        KeyEvent::new(code, KeyModifiers::empty())
    }

    /// Press the hotkey of the first menu row whose kind matches — the same
    /// route a player's keypress takes, so a hotkey clash would show here.
    fn press_row(app: &mut App, is: fn(&MenuKind) -> bool) {
        let hotkey = app
            .menu_entries()
            .iter()
            .find(|e| is(&e.kind))
            .map(|e| e.hotkey)
            .expect("the row exists");
        app.screen = Screen::Main;
        app.handle_main_key(key(KeyCode::Char(hotkey)));
    }

    fn clock(app: &App) -> (u32, Option<u8>) {
        (
            app.game.week,
            app.game
                .ongoing_activity
                .as_ref()
                .map(OngoingActivity::weeks_done),
        )
    }

    fn offer(app: &App) -> PotentialDealOffer {
        let label = app
            .game
            .data_files
            .get_record_labels_data()
            .independent_labels[0]
            .clone();
        PotentialDealOffer {
            label_name: label.name.clone(),
            label_tier: "Independent".to_string(),
            advance: 1_000,
            royalty_rate: 0.12,
            albums_required: 1,
            original_label_data: label,
            expires_week: None,
            term_weeks: 90,
            carry_forward_unrecouped: 0,
        }
    }

    #[test]
    fn every_instant_row_works_mid_activity_and_booking_rows_still_refuse() {
        let mut app = App::new(Game::new().expect("data files present"));
        app.game.initialize_player(
            "Ray",
            "The Rayguns",
            crate::game::genre::MusicGenre::ALL[0].clone(),
        );
        app.game.player.money = 1_000_000;
        for i in 0..4 {
            app.game
                .band
                .unreleased_songs
                .push(crate::game::music::Song {
                    id: 9_000 + i,
                    name: format!("Song {i}"),
                    songwriting_quality: 60,
                });
        }
        // Two one-week sessions: two masters in the vault, one to release
        // unsigned and one to release once signed.
        for _ in 0..2 {
            app.dispatch(GameAction::RecordSingle {
                songs: 2,
                studio: RecordingStudio::DemoRoom,
            });
        }
        assert_eq!(app.game.vault().len(), 2, "test setup: two masters");
        app.dispatch(GameAction::TakeBreak);
        assert!(
            matches!(app.game.ongoing_activity, Some(OngoingActivity::Break(_))),
            "test setup: on a break"
        );
        // Park the random-event clock so nothing but the rows touches state.
        app.game.events.last_event_week = u32::MAX;
        let start = clock(&app);

        // Lifestyle: the picker opens on the current tier; one down is a
        // move up.
        press_row(&mut app, |k| matches!(k, MenuKind::Lifestyle));
        assert!(matches!(app.screen, Screen::LifestylePicker { .. }));
        app.handle_lifestyle_picker_key(key(KeyCode::Down));
        app.handle_lifestyle_picker_key(key(KeyCode::Enter));
        assert_eq!(app.game.player.lifestyle, LifestyleTier::SharedFlat);
        assert_eq!(clock(&app), start, "moving house took a week");

        // Vault → pressing picker → release, unsigned.
        press_row(&mut app, |k| matches!(k, MenuKind::Vault));
        assert!(matches!(app.screen, Screen::Vault { .. }));
        app.handle_vault_key(key(KeyCode::Enter));
        assert!(matches!(app.screen, Screen::PressingPicker { .. }));
        app.handle_pressing_picker_key(key(KeyCode::Enter));
        assert_eq!(app.game.vault().len(), 1, "one master went out");
        assert_eq!(app.game.just_released_music.len(), 1);
        assert_eq!(clock(&app), start, "releasing took a week");

        // Marketing on the record that just came out.
        press_row(&mut app, |k| matches!(k, MenuKind::Marketing));
        assert!(matches!(app.screen, Screen::MarketingRelease { .. }));
        app.handle_marketing_release_key(key(KeyCode::Enter));
        app.handle_marketing_campaign_key(key(KeyCode::Enter));
        assert_eq!(
            app.game.just_released_music[0].active_marketing.len(),
            1,
            "the campaign started, once"
        );
        assert_eq!(clock(&app), start, "marketing took a week");

        // Re-press: make the release a sold-out catalogue record.
        let mut release = app.game.just_released_music.remove(0);
        assert_eq!(release.release_type, ReleaseType::Single);
        release.copies_sold = release.copies_pressed;
        let pressed = release.copies_pressed;
        let release_id = release.id;
        app.game.band.singles_released.push(release);
        press_row(&mut app, |k| matches!(k, MenuKind::RePress));
        assert!(matches!(app.screen, Screen::RePressPicker { .. }));
        app.handle_repress_picker_key(key(KeyCode::Enter));
        app.handle_repress_tier_picker_key(key(KeyCode::Enter));
        let repressed = app
            .game
            .band
            .singles_released
            .iter()
            .find(|r| r.id == release_id)
            .expect("release kept");
        assert!(
            repressed.copies_pressed > pressed,
            "a fresh run was pressed"
        );
        assert_eq!(clock(&app), start, "re-pressing took a week");

        // Support slot: accepting is refused (a second commitment), the
        // modal stays up with the offer intact; declining works.
        app.game.pending_support_offer = Some(SupportTourOffer {
            host_band: "Big Stars".to_string(),
            host_fame: 90,
            weeks: 3,
            pay: 1_000,
            fame_gain: 6,
            expires_week: app.game.week + 10,
        });
        press_row(&mut app, |k| matches!(k, MenuKind::SupportTour));
        assert!(matches!(app.screen, Screen::SupportOffer));
        app.log.clear();
        app.handle_support_offer_key(key(KeyCode::Char('a')));
        assert!(
            matches!(app.screen, Screen::SupportOffer),
            "the offer stays on screen"
        );
        assert!(app.game.pending_support_offer.is_some(), "and on the table");
        let refusal = app.log.last().expect("the refusal is explained");
        assert!(
            refusal.text.contains("middle of a break"),
            "{}",
            refusal.text
        );
        app.handle_support_offer_key(key(KeyCode::Char('r')));
        assert!(app.game.pending_support_offer.is_none(), "declined");
        assert!(matches!(app.screen, Screen::Main));
        assert_eq!(clock(&app), start, "the support slot took a week");

        // Deals: reject one, accept the other.
        let two = [offer(&app), offer(&app)];
        app.game.pending_deal_offers.extend(two);
        press_row(&mut app, |k| matches!(k, MenuKind::Deals));
        assert!(matches!(app.screen, Screen::Deals { .. }));
        app.handle_deals_key(key(KeyCode::Char('r')));
        assert_eq!(app.game.pending_deal_offers.len(), 1, "one rejected");
        app.handle_deals_key(key(KeyCode::Char('a')));
        assert!(app.game.pending_deal_offers.is_empty());
        assert!(app.game.band.current_deal().is_some(), "signed");
        assert_eq!(clock(&app), start, "the deal took a week");

        // Vault again, now signed: the label decides the run and the row
        // dispatches straight away.
        press_row(&mut app, |k| matches!(k, MenuKind::Vault));
        app.handle_vault_key(key(KeyCode::Enter));
        assert!(app.game.vault().is_empty(), "the label put it out");
        assert_eq!(clock(&app), start, "the signed release took a week");

        // And the booking rows still refuse, without spending a week.
        for hotkey in ['4', '5', '6', '7'] {
            app.screen = Screen::Main;
            app.handle_main_key(key(KeyCode::Char(hotkey)));
            assert!(
                matches!(app.screen, Screen::Main),
                "'{hotkey}' must not open a booking picker mid-activity"
            );
        }
        assert_eq!(clock(&app), start);
        assert!(matches!(
            app.game.ongoing_activity,
            Some(OngoingActivity::Break(_))
        ));
    }
}
