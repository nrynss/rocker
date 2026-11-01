//! Instant actions during a running commitment (design §F, post-#27 / PR #37).
//!
//! An instant action — one `process_turn` does not spend a week on — used to
//! be swallowed while an activity ran: `process_turn` swapped it for the
//! activity's next week, so the action vanished *and* the week it was meant
//! not to cost was spent anyway. Since the fix every instant action goes
//! through a running commitment exactly once, takes no week, and leaves the
//! activity's progress where it was. Accepting a support slot is the one
//! deliberate exception: it starts a second commitment, so it is refused out
//! loud rather than swallowed.

use crate::game::music::MarketingCampaignType;
use crate::game::player::LifestyleTier;

use super::time::{TOUR_REGION, TOUR_RIG, TOUR_WEEKS, park_events, touring_band};
use super::*;

/// An unsigned touring band one week into a four-week tour, with the event
/// taps parked so nothing random moves money between the snapshots.
fn mid_tour(seed: u64) -> Game {
    let mut game = touring_band(seed);
    game.band.record_deal = None;
    game.pending_deal_offers.clear();
    game.pending_support_offer = None;
    game.process_turn(GameAction::GoOnTour(TOUR_REGION, TOUR_RIG, TOUR_WEEKS))
        .expect("the tour books");
    park_events(&mut game);
    assert!(
        matches!(game.ongoing_activity, Some(OngoingActivity::Tour(_))),
        "test setup: the band is on the road"
    );
    game
}

/// Where the calendar and the activity stand: the two things an instant
/// action must not move.
fn clock(game: &Game) -> (u32, Option<u8>) {
    (
        game.week,
        game.ongoing_activity
            .as_ref()
            .map(OngoingActivity::weeks_done),
    )
}

/// Submit `action` mid-activity and assert it cost no week and left the
/// activity exactly where it was.
fn submit_instant(game: &mut Game, action: GameAction) {
    let before = clock(game);
    let label = game.ongoing_activity.as_ref().map(OngoingActivity::label);
    game.process_turn(action.clone())
        .unwrap_or_else(|e| panic!("{action:?} should apply mid-activity: {e}"));
    assert_eq!(
        clock(game),
        before,
        "{action:?} must take no week and leave the activity's progress alone"
    );
    assert_eq!(
        game.ongoing_activity.as_ref().map(OngoingActivity::label),
        label,
        "{action:?} must not end or replace the running activity"
    );
}

#[test]
fn changing_lifestyle_mid_tour_moves_house_without_spending_a_week() {
    let mut game = mid_tour(31);
    assert_eq!(game.player.lifestyle, LifestyleTier::Squat);
    let money = game.player.money;
    let cost = LifestyleTier::SharedFlat.move_up_cost() as i32;

    submit_instant(
        &mut game,
        GameAction::ChangeLifestyle(LifestyleTier::SharedFlat),
    );

    assert_eq!(game.player.lifestyle, LifestyleTier::SharedFlat);
    assert_eq!(game.player.money, money - cost, "the move is paid once");
}

#[test]
fn accepting_a_deal_mid_tour_signs_it_without_spending_a_week() {
    let mut game = mid_tour(32);
    let offer = test_deal_offer(&game, None);
    let label = offer.label_name.clone();
    let advance = offer.advance as i32;
    game.pending_deal_offers.push(offer);
    let money = game.player.money;

    submit_instant(&mut game, GameAction::AcceptDeal(0));

    assert_eq!(
        game.band.current_deal().map(|d| d.label_name.clone()),
        Some(label),
        "the deal is signed"
    );
    assert!(game.pending_deal_offers.is_empty(), "the offer is consumed");
    assert_eq!(
        game.player.money,
        money + advance,
        "the advance is banked once"
    );
}

#[test]
fn rejecting_a_deal_mid_tour_declines_it_without_spending_a_week() {
    let mut game = mid_tour(33);
    let offer = test_deal_offer(&game, None);
    game.pending_deal_offers.push(offer.clone());
    game.pending_deal_offers.push(offer);

    submit_instant(&mut game, GameAction::RejectDeal(0));

    assert_eq!(
        game.pending_deal_offers.len(),
        1,
        "exactly the one offer is turned down"
    );
    assert!(game.band.current_deal().is_none());
}

fn support_offer(game: &Game) -> SupportTourOffer {
    SupportTourOffer {
        host_band: "Big Stars".to_string(),
        host_fame: 90,
        weeks: 3,
        pay: 1_000,
        fame_gain: 6,
        expires_week: game.week + 10,
    }
}

#[test]
fn declining_a_support_slot_mid_tour_takes_no_week() {
    let mut game = mid_tour(34);
    game.pending_support_offer = Some(support_offer(&game));

    submit_instant(&mut game, GameAction::DeclineSupportTour);

    assert!(game.pending_support_offer.is_none(), "the slot is declined");
}

/// The one exception, on purpose: a support run is a commitment, and two
/// cannot run at once. It must be *refused* — offer kept, money untouched,
/// no week spent — not swallowed as another week of the tour.
#[test]
fn accepting_a_support_slot_mid_tour_is_refused_out_loud() {
    let mut game = mid_tour(35);
    game.pending_support_offer = Some(support_offer(&game));
    let before = clock(&game);
    let money = game.player.money;

    let err = game
        .process_turn(GameAction::AcceptSupportTour)
        .expect_err("a second commitment cannot start on top of the tour");

    assert!(err.contains("already committed"), "{err}");
    assert_eq!(clock(&game), before, "the refusal takes no week");
    assert_eq!(game.player.money, money, "no guarantee is banked");
    assert!(
        game.pending_support_offer.is_some(),
        "the offer stays on the table for after the tour"
    );
    assert!(matches!(
        game.ongoing_activity,
        Some(OngoingActivity::Tour(_))
    ));
}

#[test]
fn marketing_mid_tour_starts_the_campaign_without_spending_a_week() {
    let mut game = mid_tour(36);
    let mut release = test_release(777, ReleaseType::Single);
    release.copies_pressed = 1_000;
    game.band.singles_released.push(release);
    let money = game.player.money;
    let cost = MarketingCampaignType::BasicPress.spec().cost;

    submit_instant(
        &mut game,
        GameAction::StartMarketingCampaign(777, MarketingCampaignType::BasicPress),
    );

    let release = game
        .band
        .singles_released
        .iter()
        .find(|r| r.id == 777)
        .expect("release kept");
    assert_eq!(release.active_marketing.len(), 1, "one campaign, once");
    assert_eq!(game.player.money, money - cost, "the campaign is paid once");
}

#[test]
fn re_pressing_mid_tour_tops_up_stock_without_spending_a_week() {
    let mut game = mid_tour(37);
    let mut release = test_release(778, ReleaseType::Single);
    release.copies_pressed = 1_000;
    release.copies_sold = 950;
    game.band.singles_released.push(release);
    let money = game.player.money;
    let run = PRESSING_TIERS[0].1;
    let cost = game.pressing_cost(&ReleaseType::Single, run);

    submit_instant(
        &mut game,
        GameAction::RePress {
            release_id: 778,
            pressing: Some(0),
        },
    );

    let release = game
        .band
        .singles_released
        .iter()
        .find(|r| r.id == 778)
        .expect("release kept");
    assert_eq!(release.copies_pressed, 1_000 + run, "one fresh run");
    assert_eq!(game.player.money, money - cost, "the run is paid once");
}

/// A band with one finished master in the vault, then sent on tour.
fn master_then_tour(seed: u64, signed: bool) -> Game {
    let mut game = touring_band(seed);
    game.band.record_deal = None;
    game.pending_deal_offers.clear();
    for i in 0..2 {
        game.band.unreleased_songs.push(crate::game::music::Song {
            id: 5_000 + i,
            name: format!("Vault Song {i}"),
            songwriting_quality: 60,
        });
    }
    game.action_record_single(2, TEST_STUDIO)
        .expect("a two-song session books");
    finish_activity_in_one_sitting(&mut game);
    assert_eq!(game.vault().len(), 1, "test setup: one master waiting");
    if signed {
        game.band.record_deal = Some(test_deal(70, 0.12));
    }
    game.process_turn(GameAction::GoOnTour(TOUR_REGION, TOUR_RIG, TOUR_WEEKS))
        .expect("the tour books");
    park_events(&mut game);
    assert!(game.ongoing_activity.is_some(), "test setup: on the road");
    game
}

#[test]
fn releasing_a_master_mid_tour_puts_it_out_without_spending_a_week() {
    let mut game = master_then_tour(38, false);
    let released_before = game.just_released_music.len() + game.band.total_releases();

    submit_instant(
        &mut game,
        GameAction::ReleaseMaster {
            master: 0,
            pressing: Some(0),
        },
    );

    assert!(game.vault().is_empty(), "the master left the vault");
    assert_eq!(
        game.just_released_music.len() + game.band.total_releases(),
        released_before + 1,
        "exactly one record came out"
    );
}

/// A signed act's label decides the run — the Vault dispatches straight
/// from the row with no pressing choice — and that path must survive a
/// running tour too.
#[test]
fn a_signed_act_releasing_mid_tour_takes_no_week() {
    let mut game = master_then_tour(39, true);
    let released_before = game.just_released_music.len() + game.band.total_releases();

    submit_instant(
        &mut game,
        GameAction::ReleaseMaster {
            master: 0,
            pressing: None,
        },
    );

    assert!(game.vault().is_empty(), "the label put the master out");
    assert_eq!(
        game.just_released_music.len() + game.band.total_releases(),
        released_before + 1
    );
}

/// The activity being a *session* is the case most likely to collide: the
/// record still in the room must stay in the room while an older master
/// goes out.
#[test]
fn releasing_an_older_master_mid_session_leaves_the_session_alone() {
    let mut game = touring_band(40);
    game.band.record_deal = None;
    for i in 0..10 {
        game.band.unreleased_songs.push(crate::game::music::Song {
            id: 6_000 + i,
            name: format!("Song {i}"),
            songwriting_quality: 60,
        });
    }
    game.action_record_single(2, TEST_STUDIO)
        .expect("a two-song session books");
    finish_activity_in_one_sitting(&mut game);
    game.process_turn(GameAction::RecordAlbum {
        songs: 8,
        studio: TEST_STUDIO,
    })
    .expect("the album session books");
    park_events(&mut game);
    assert!(
        matches!(game.ongoing_activity, Some(OngoingActivity::Recording(_))),
        "test setup: an album session that outlasts its booking week"
    );
    assert_eq!(
        game.vault().len(),
        1,
        "only the finished single is in the vault"
    );

    submit_instant(
        &mut game,
        GameAction::ReleaseMaster {
            master: 0,
            pressing: Some(0),
        },
    );

    assert!(game.vault().is_empty(), "the single went out");
    assert!(
        matches!(game.ongoing_activity, Some(OngoingActivity::Recording(_))),
        "the album is still being made"
    );
}

/// Week-taking actions are still the activity's: they play its next week
/// and do nothing else (this is what the UI's "turns advance" means).
#[test]
fn a_week_taking_action_still_plays_the_activity_week() {
    let mut game = mid_tour(41);
    let (week, done) = clock(&game);
    let health = game.player.health;

    game.process_turn(GameAction::VisitDoctor)
        .expect("the turn resolves as a tour week");

    assert_eq!(game.week, week + 1);
    assert_eq!(
        game.ongoing_activity
            .as_ref()
            .map(OngoingActivity::weeks_done),
        done.map(|d| d + 1),
        "the tour advanced one week"
    );
    assert!(
        game.player.health <= health,
        "no doctor's visit happened from the back of the van"
    );
}
