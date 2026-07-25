//! The balance lab: whole careers played headlessly by bot policies.
//!
//! Compiled only for tests (`#[cfg(test)] mod sim;` in `game/mod.rs`).
//! Each bot is deliberately boring — a match on game state returning the
//! next `GameAction` — so a run reads like a play-through, not a framework.
//!
//! Two speeds:
//! - CI-safe smoke tests (not ignored): short horizons, deterministic-safe
//!   invariants only.
//! - `#[ignore]`d sweeps: many seeds over fifteen game-years, printing a
//!   summary table. Run with `cargo test -- --ignored --nocapture`.
//!
//! Determinism: worldgen, the weekly world update, and (since Track B)
//! every player-action roll derive from `world_seed`, so a seed plus a
//! policy replays the same career exactly — exact-value assertions are
//! fair game. `seeded_worlds_are_reproducible_in_the_harness` pins this.

use crate::data_loader::GameDataFiles;
use crate::game::band::Band;
use crate::game::events::EventManager;
use crate::game::music;
use crate::game::music::DistributionChannel;
use crate::game::music::ReleaseType;
use crate::game::player::LifestyleTier;
use crate::game::player::Player;
use crate::game::timeline::MusicTimeline;
use crate::game::world::GameWorld;
use crate::game::world::{LabelStrategy, LabelTier};
use rand::SeedableRng;
use rand::rngs::StdRng;

use super::constants::{self, *};
use super::shows;
use super::*;
use std::panic::{AssertUnwindSafe, catch_unwind};

/// Pressing tiers the bots buy, as indices into `PRESSING_TIERS`.
const GARAGE_RUN: usize = 0;
const CLUB_RUN: usize = 1;

const SMOKE_HORIZON: u32 = 5 * constants::WEEKS_PER_YEAR;
const SWEEP_HORIZON: u32 = 15 * constants::WEEKS_PER_YEAR;
const WIN_TARGET: u32 = 12 * constants::WEEKS_PER_YEAR;
const SWEEP_SEEDS: u64 = 60;
const WIN_TARGET_SEEDS: u64 = 48;

/// Below this health a bot drops everything and looks after itself.
const HEALTH_FLOOR: u8 = 40;
/// At this stress a bot takes a real break.
const STRESS_CEILING: u8 = 70;

/// Consecutive `process_turn` calls that may leave the calendar untouched
/// (deal paperwork and the like) before we call the policy broken.
const STALL_LIMIT: u32 = 100;

// ---------------------------------------------------------------------------
// Building games
// ---------------------------------------------------------------------------

/// Build a game on a chosen world seed.
///
/// `Game::new` takes its seed from the `ROCKER_SEED` environment variable,
/// which is process-global while test threads run in parallel (and mutating
/// it is `unsafe` in edition 2024). Every field of `Game` is public, so the
/// harness assembles the identical state directly instead. Keep this in
/// lockstep with `Game::new`; `seeded_worlds_are_reproducible_in_the_harness`
/// guards the determinism property itself.
pub(super) fn seeded_game(seed: u64) -> Game {
    let data_files = GameDataFiles::load().expect("data files present");
    let mut init_rng = StdRng::seed_from_u64(seed);
    let world = GameWorld::new(&data_files, &mut init_rng);
    let mut game = Game {
        world_seed: seed,
        player: Player::default(),
        band: Band::default(),
        world,
        events: EventManager::new(),
        timeline: MusicTimeline::new(&data_files),
        data_files,
        pending_deal_offers: Vec::new(),
        pending_support_offer: None,
        regional_fame: std::collections::HashMap::new(),
        idle_streak: 0,
        decay_streak: 0,
        genre_trend_reported: 0,
        writing_streak: 0,
        week: 1,
        game_over: false,
        next_song_id: 0,
        next_release_id: 0,
        just_released_music: Vec::new(),
        last_tour_report: None,
        turn_log: Vec::new(),
        rockstar_achieved: false,
        current_distribution_channel: music::DistributionChannel::default(),
        last_sales_pass_week: None,
        // S-TIME (§F): nobody starts a career mid-tour.
        ongoing_activity: None,
    };
    game.initialize_player("Sim Driver", "The Test Pattern", genre::MusicGenre::Rock);
    game
}

/// Silence both story-event taps so a run exercises only the core loop:
/// mark every era's historical events as already told, and park the
/// random-event clock on the current week (the caller re-parks it each
/// turn). Used where an invariant must be deterministic-safe.
fn suppress_story_events(game: &mut Game) {
    let every_event: Vec<String> = game
        .timeline
        .eras
        .values()
        .flat_map(|era| era.major_events.iter().cloned())
        .collect();
    game.timeline.triggered_events.extend(every_event);
    game.events.last_event_week = u32::MAX;
}

// ---------------------------------------------------------------------------
// Bots
// ---------------------------------------------------------------------------

/// The four careers the lab simulates.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Bot {
    /// Never records; always plays the biggest stage that will have them.
    GigGrinder,
    /// Never performs; writes constantly and ships garage-run records.
    StudioRat,
    /// The intended loop: write, press a club run, gig the sales window.
    /// Ignores record deals.
    BalancedIndie,
    /// Balanced-indie who signs the first deal offered and keeps delivering.
    LabelLoyalist,
    /// M7 (§F): never tours; matches its lifestyle tier to what it can
    /// afford. Must survive fifteen years without bankruptcy — the proof
    /// that lifestyle upkeep is a real cost, not a death spiral.
    Homebody,
    /// M7 (§F): tours constantly on the biggest rig it can afford. Must not
    /// trivially out-earn the release-focused grinders (touring buys fame
    /// and regional presence, not a bottomless purse).
    RoadDog,
    /// M7 (§F): never signs; buys the best indie distribution tier it can.
    /// Must be viable — slower than a signed peer, ahead of the old flat
    /// 0.15 indie floor.
    IndieLifer,
}

impl Bot {
    const ALL: [Bot; 7] = [
        Bot::GigGrinder,
        Bot::StudioRat,
        Bot::BalancedIndie,
        Bot::LabelLoyalist,
        Bot::Homebody,
        Bot::RoadDog,
        Bot::IndieLifer,
    ];

    fn name(self) -> &'static str {
        match self {
            Bot::GigGrinder => "gig-grinder",
            Bot::StudioRat => "studio-rat",
            Bot::BalancedIndie => "balanced-indie",
            Bot::LabelLoyalist => "label-loyalist",
            Bot::Homebody => "homebody",
            Bot::RoadDog => "road-dog",
            Bot::IndieLifer => "indie-lifer",
        }
    }

    /// Non-action state a policy needs to set before deciding — only the
    /// indie-lifer uses it, to buy the best distribution channel its fame
    /// and wallet allow (the channel is a picker-remembered `Game` field,
    /// applied at record time, not a `GameAction`). No-op for everyone else.
    fn pre_turn(self, game: &mut Game) {
        if self == Bot::IndieLifer {
            // Reserve the studio + pressing bill of the release the bot is
            // about to attempt (an album if one's banked, else a single) so
            // the chosen channel's fee can't push that release out of reach.
            // Picking a channel on its fee alone could make the full bill
            // unaffordable, and the bot would skip recording — skewing the
            // sweep. CLUB_RUN matches the pressing tier `indie_lifer` uses.
            let kind = if game.band.can_record_album() {
                ReleaseType::Album
            } else {
                ReleaseType::Single
            };
            let reserve = release_bill(game, kind, Some(CLUB_RUN));
            game.current_distribution_channel = best_affordable_channel(game, reserve);
        }
    }

    /// Look at the game, pick one action. No memory, no cleverness.
    fn decide(self, game: &Game) -> GameAction {
        if let Some(action) = self_care(game) {
            return action;
        }
        match self {
            Bot::GigGrinder => gig_grinder(game),
            Bot::StudioRat => studio_rat(game),
            Bot::BalancedIndie => indie_loop(game, Some(CLUB_RUN), true),
            Bot::LabelLoyalist => label_loyalist(game),
            Bot::Homebody => homebody(game),
            Bot::RoadDog => road_dog(game),
            Bot::IndieLifer => indie_lifer(game),
        }
    }
}

/// Honest upkeep shared by every bot: patch up before falling apart, take a
/// real break before burning out, otherwise carry on with the day job.
fn self_care(game: &Game) -> Option<GameAction> {
    let player = &game.player;
    if player.health < HEALTH_FLOOR {
        return Some(if player.can_afford(constants::DOCTOR_VISIT_COST) {
            GameAction::VisitDoctor
        } else {
            GameAction::TakeBreak
        });
    }
    if player.stress >= STRESS_CEILING {
        return Some(GameAction::TakeBreak);
    }
    None
}

/// The biggest stage whose door policy admits the band right now
/// (venue gate: `prestige <= fame + 20`).
fn biggest_open_venue(game: &Game) -> usize {
    (0..game.world.venues.len())
        .filter(|&i| game.world.venues[i].prestige <= game.band.fame.saturating_add(20))
        .max_by_key(|&i| game.world.venues[i].capacity)
        .expect("at least one venue is always open")
}

fn gig_or_rest(game: &Game) -> GameAction {
    if game.player.stress < GIG_STRESS_GUARD && game.player.health >= GIG_HEALTH_GUARD {
        GameAction::Gig(biggest_open_venue(game))
    } else {
        GameAction::LazeAround
    }
}

/// The out-of-pocket bill to record and press `kind` right now: studio time
/// always, plus the pressing run when unsigned (a label presses for free).
fn release_bill(game: &Game, kind: ReleaseType, pressing: Option<usize>) -> i32 {
    let mut bill = game.recording_cost(&kind);
    if game.band.current_deal().is_none() {
        let (_, copies) = PRESSING_TIERS[pressing.unwrap_or(GARAGE_RUN)];
        bill += game.pressing_cost(&kind, copies);
    }
    bill
}

fn gig_grinder(game: &Game) -> GameAction {
    gig_or_rest(game)
}

fn studio_rat(game: &Game) -> GameAction {
    if game.band.can_record_album() {
        if game
            .player
            .can_afford(release_bill(game, ReleaseType::Album, Some(GARAGE_RUN)))
        {
            return GameAction::RecordAlbum {
                pressing: Some(GARAGE_RUN),
            };
        }
        // Can't afford the album yet: press a spare song (anything beyond
        // the eight banked for the album) as a single to raise the cash.
        if game.band.unreleased_songs.len() > constants::MIN_ALBUM_SONGS as usize
            && game
                .player
                .can_afford(release_bill(game, ReleaseType::Single, Some(GARAGE_RUN)))
        {
            return GameAction::RecordSingle {
                pressing: Some(GARAGE_RUN),
            };
        }
        // Blocked on money with nothing spare to single off either: keep
        // writing. This is the bot's only bootstrap out of "broke, one
        // album's worth of songs banked, can't afford to record any of
        // them" — the next song written is what crosses the `> MIN_ALBUM_
        // SONGS` line above and unlocks a cheap single sale. (Tried
        // swapping this for `Practice` once stress allows, on the theory
        // that it avoids the writing-streak fatigue penalty for "no new
        // inventory" weeks — measured a *regression*: studio-rat's win
        // rate dropped from 65% to 36% and bankruptcies rose from 16/60 to
        // 34/60 in the 15-year sweep, because it cut off the only escape
        // valve from this exact state. Reverted; left as a documented
        // dead end rather than silently discarded.)
    }
    if game.player.stress < STUDIO_STRESS_BLOCK {
        GameAction::WriteSongs
    } else {
        GameAction::LazeAround
    }
}

/// The intended player loop. `pressing` is the run an unsigned band buys;
/// when signed the label presses regardless (`plan_pressing` checks the
/// deal first), so `None` is fine there.
fn indie_loop(game: &Game, pressing: Option<usize>, allow_support: bool) -> GameAction {
    // A support slot is exposure money can't buy — and it pays. The
    // homebody skips it (never tours); everyone else grabs it.
    if allow_support
        && game.pending_support_offer.is_some()
        && game.player.stress < TOUR_STRESS_GUARD
        && game.player.health >= TOUR_HEALTH_GUARD
    {
        return GameAction::AcceptSupportTour;
    }
    // Work the room while there's a record on the shelves.
    if !game.just_released_music.is_empty() {
        return gig_or_rest(game);
    }
    // Albums when possible: eight songs banked and the bill covered.
    if game.band.can_record_album()
        && game
            .player
            .can_afford(release_bill(game, ReleaseType::Album, pressing))
    {
        return GameAction::RecordAlbum { pressing };
    }
    // Songs above the album pile become singles: cash flow, a higher live
    // ceiling, and something on the shelves worth gigging on.
    if game.band.unreleased_songs.len() > constants::MIN_ALBUM_SONGS as usize
        && game
            .player
            .can_afford(release_bill(game, ReleaseType::Single, pressing))
    {
        return GameAction::RecordSingle { pressing };
    }
    // Build the song pile toward the next album...
    if !game.band.can_record_album() {
        return if game.player.stress < STUDIO_STRESS_BLOCK {
            GameAction::WriteSongs
        } else {
            GameAction::LazeAround
        };
    }
    // ...or gig until the album bill is affordable.
    gig_or_rest(game)
}

fn label_loyalist(game: &Game) -> GameAction {
    // Sign whatever lands on the table, sight unseen. Loyalty!
    if !game.pending_deal_offers.is_empty() {
        return GameAction::AcceptDeal(0);
    }
    let pressing = if game.band.has_record_deal() {
        None
    } else {
        Some(CLUB_RUN)
    };
    indie_loop(game, pressing, true)
}

/// M7: the best distribution channel this act's fame and wallet allow, after
/// reserving `reserve` for the studio + pressing cost of the release it's
/// about to record. `DistributionChannel::ALL` is ascending in reach, so the
/// last one that clears its fame gate and whose fee still fits alongside the
/// reserved bill wins. Reserving the full bill (rather than weighing the fee
/// in isolation) stops the bot from picking a channel that then makes the
/// release unaffordable, gets it skipped, and skews the balance sweep.
fn best_affordable_channel(game: &Game, reserve: i32) -> DistributionChannel {
    let mut best = DistributionChannel::ALL[0];
    for &channel in DistributionChannel::ALL.iter() {
        if channel.is_available(game.band.fame) && game.player.can_afford(reserve + channel.fee()) {
            best = channel;
        }
    }
    best
}

/// M7 (§F): never tours (nor takes a support slot), and keeps its lifestyle
/// tier matched to its means — trading up only when flush enough to cover
/// several deposits over, trading down the moment it goes into the red.
/// Both moves are instant and there are only five tiers, so the run of
/// consecutive `ChangeLifestyle` turns is bounded well under `STALL_LIMIT`.
fn homebody(game: &Game) -> GameAction {
    let cur = game.player.lifestyle;
    let money = game.player.money;

    let next_up = LifestyleTier::ALL
        .iter()
        .copied()
        .find(|t| *t > cur && t.down() == Some(cur));
    if let Some(up) = next_up {
        // Only climb when the deposit is trivial relative to cash on hand,
        // so upkeep stays a sane slice of income rather than a spiral.
        if money > up.move_up_cost() as i32 * 3 {
            return GameAction::ChangeLifestyle(up);
        }
    }
    if money < 0
        && let Some(down) = cur.down()
    {
        return GameAction::ChangeLifestyle(down);
    }
    // The indie day job, minus the road: never accept a support tour.
    indie_loop(game, Some(CLUB_RUN), false)
}

/// M7 (§F): the biggest tour this act can afford right now — a region it
/// qualifies for, the grandest available rig, the longest available length,
/// all vetted through the same `quote_tour` the picker shows the player.
fn plan_biggest_tour(game: &Game) -> Option<(usize, TourRig, u8)> {
    let regions = game.get_sorted_regions();
    let region_index = regions
        .iter()
        .position(|(_, _, _, _, _, fame_req)| game.band.fame >= *fame_req)?;
    for &rig in TourRig::ALL.iter().rev() {
        if !game.rig_is_available(rig) {
            continue;
        }
        for weeks in (1..=4u8).rev() {
            if !game.tour_length_is_available(weeks) {
                continue;
            }
            if let Ok(quote) = game.quote_tour(region_index, rig, weeks)
                && game.player.can_afford(quote.cost)
            {
                return Some((region_index, rig, weeks));
            }
        }
    }
    None
}

/// M7 (§F): lives on the road. Tours the biggest affordable rig whenever
/// stress and health allow; otherwise writes/records so there's fresh
/// material (and a higher live cap) to tour behind.
fn road_dog(game: &Game) -> GameAction {
    if game.player.stress < TOUR_STRESS_GUARD
        && game.player.health >= TOUR_HEALTH_GUARD
        && let Some((region, rig, weeks)) = plan_biggest_tour(game)
    {
        return GameAction::GoOnTour(region, rig, weeks);
    }
    indie_loop(game, Some(CLUB_RUN), true)
}

/// M7 (§F): the eternal independent — never signs (it simply never accepts;
/// offers expire on their own, and explicitly rejecting them is a no-week
/// action that would only spin the calendar). The `pre_turn` hook keeps it
/// on the best distribution tier its fame and wallet allow, which is what
/// sets it apart from balanced-indie's default mail-order reach.
fn indie_lifer(game: &Game) -> GameAction {
    indie_loop(game, Some(CLUB_RUN), true)
}

// ---------------------------------------------------------------------------
// Careers and the runner
// ---------------------------------------------------------------------------

/// How a simulated career ended.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Ending {
    /// Rockstar achieved flag was set (fame >= 90 with ≥5 albums).
    /// Game continues after this milestone; it's not an ending, but a tracking flag.
    Rockstar,
    Died,
    WentBroke,
    /// Still playing when the horizon arrived.
    StillGoing,
}

/// Categorize the outcome: death/broke (hard endings), rockstar milestone (played to that point),
/// or still going (neither ended nor achieved rockstar by horizon).
fn ending_of(game: &Game) -> Ending {
    if game.player.health == 0 {
        Ending::Died
    } else if game.player.money < 0 && game.band.fame < 10 {
        Ending::WentBroke
    } else if game.rockstar_achieved {
        Ending::Rockstar
    } else {
        Ending::StillGoing
    }
}

/// Everything one simulated career leaves behind.
struct Career {
    bot: Bot,
    ending: Ending,
    weeks: u32,
    final_fame: u8,
    peak_fame: u8,
    final_money: i32,
    albums: usize,
    singles: usize,
    weeks_to_first_album: Option<u32>,
    weeks_to_fame_50: Option<u32>,
    /// Week the rockstar milestone flag first flipped true (§E — since L9 the
    /// milestone doesn't end the game, so `career.weeks` is the loop's exit
    /// week, not the achievement week; this is the field that answers "when
    /// did they actually make it").
    weeks_to_rockstar: Option<u32>,
    first_deal_week: Option<u32>,
    /// Term length of the first deal signed (§E-4), stamped alongside
    /// `first_deal_week`.
    first_deal_term_weeks: Option<u16>,
    /// Week the first deal's recoupment ledger (§E-2) first cleared — the
    /// §F target reads this against the term's halfway mark. `None` if that
    /// deal never recouped inside the horizon.
    first_deal_recoup_week: Option<u32>,
    deals_signed: u32,
    sell_outs: u32,
    /// M7 (§E-4): contract breaches — a term ran out with albums owed.
    breaches: u32,
    /// M7 (§D): released records certified silver or better by the end.
    certifications: usize,
    /// M7 (§B): lifestyle tier at the end, as an `ALL` index (0 = Squat).
    final_lifestyle: u8,
    /// Band fame sampled at each completed game-year boundary.
    fame_by_year: Vec<u8>,
}

impl Career {
    fn won(&self) -> bool {
        self.ending == Ending::Rockstar
    }
}

fn run_career(bot: Bot, seed: u64, horizon_weeks: u32) -> Career {
    let mut game = seeded_game(seed);
    drive(bot, &mut game, horizon_weeks)
}

/// Drive a game to the horizon (or the end screen), one decision per turn.
/// Rejected actions are the game saying no — the bot rests instead of
/// arguing. Any panic below `process_turn` propagates and fails the test.
fn drive(bot: Bot, game: &mut Game, horizon_weeks: u32) -> Career {
    let mut career = Career {
        bot,
        ending: Ending::StillGoing,
        weeks: 0,
        final_fame: 0,
        peak_fame: 0,
        final_money: 0,
        albums: 0,
        singles: 0,
        weeks_to_first_album: None,
        weeks_to_fame_50: None,
        weeks_to_rockstar: None,
        first_deal_week: None,
        first_deal_term_weeks: None,
        first_deal_recoup_week: None,
        deals_signed: 0,
        sell_outs: 0,
        breaches: 0,
        certifications: 0,
        final_lifestyle: 0,
        fame_by_year: Vec::new(),
    };
    let mut had_deal = false;
    let mut stalled_turns: u32 = 0;

    while !game.is_game_over() && game.week <= horizon_weeks {
        let week_before = game.week;
        bot.pre_turn(game);
        let action = bot.decide(game);
        if game.process_turn(action).is_err() {
            game.process_turn(GameAction::LazeAround)
                .expect("lazing is always allowed");
        }

        for line in game.take_turn_log() {
            if line.contains("sold out") {
                career.sell_outs += 1;
            }
            // M7: a term-expiry breach (§E-4) — the label drops the band
            // with albums still owed. The steady releasers should never
            // trip this; a sign-then-neglect policy will.
            if line.contains("the contract ran out with albums still owed") {
                career.breaches += 1;
            }
        }
        observe(&mut career, game, &mut had_deal);

        if game.week == week_before {
            stalled_turns += 1;
            assert!(
                stalled_turns < STALL_LIMIT,
                "{} stalled the calendar at week {} on seed {}",
                bot.name(),
                game.week,
                game.world_seed
            );
        } else {
            stalled_turns = 0;
        }
    }

    career.ending = ending_of(game);
    career.weeks = game.week;
    career.final_fame = game.band.fame;
    career.final_money = game.player.money;
    career.albums = game.band.albums_released.len();
    career.singles = game.band.singles_released.len();
    career.certifications = game
        .band
        .albums_released
        .iter()
        .chain(game.band.singles_released.iter())
        .filter(|r| r.certified >= 1)
        .count();
    career.final_lifestyle = LifestyleTier::ALL
        .iter()
        .position(|t| *t == game.player.lifestyle)
        .unwrap_or(0) as u8;
    career
}

/// Milestone bookkeeping after each turn.
fn observe(career: &mut Career, game: &Game, had_deal: &mut bool) {
    career.peak_fame = career.peak_fame.max(game.band.fame);
    if career.weeks_to_first_album.is_none() && !game.band.albums_released.is_empty() {
        career.weeks_to_first_album = Some(game.week);
    }
    if career.weeks_to_fame_50.is_none() && game.band.fame >= 50 {
        career.weeks_to_fame_50 = Some(game.week);
    }
    if career.weeks_to_rockstar.is_none() && game.rockstar_achieved {
        career.weeks_to_rockstar = Some(game.week);
    }
    let signed = game.band.has_record_deal();
    if signed && !*had_deal {
        career.deals_signed += 1;
        if career.first_deal_week.is_none() {
            career.first_deal_week = Some(game.week);
            career.first_deal_term_weeks = game.band.current_deal().map(|deal| deal.term_weeks);
        }
    }
    // §F recoupment target: the week the FIRST deal's ledger clears. Only
    // watched while that first deal is still the active one — a later deal's
    // fresh (smaller) ledger must not masquerade as the first recouping.
    if signed
        && career.deals_signed == 1
        && career.first_deal_recoup_week.is_none()
        && game
            .band
            .current_deal()
            .is_some_and(|deal| deal.unrecouped <= 0)
    {
        career.first_deal_recoup_week = Some(game.week);
    }
    *had_deal = signed;
    while career.fame_by_year.len() < (game.week / constants::WEEKS_PER_YEAR) as usize {
        career.fame_by_year.push(game.band.fame);
    }
}

// ---------------------------------------------------------------------------
// Summaries
// ---------------------------------------------------------------------------

fn median<T: Copy + Ord>(mut values: Vec<T>) -> Option<T> {
    if values.is_empty() {
        return None;
    }
    values.sort_unstable();
    Some(values[values.len() / 2])
}

fn show<T: std::fmt::Display>(value: Option<T>) -> String {
    value.map_or_else(|| "--".to_string(), |v| v.to_string())
}

fn print_report(careers: &[Career]) {
    println!();
    println!(
        "{:<15} {:>4} {:>4} {:>5} {:>7} {:>9} {:>9} {:>9} {:>9} {:>8} {:>8} {:>5} {:>5} {:>5}",
        "bot",
        "runs",
        "wins",
        "win%",
        "win.yr",
        "med.fame",
        "med.peak",
        "med.$",
        "alb1.wk",
        "fame50",
        "sellout",
        "died",
        "broke",
        "going"
    );
    for bot in Bot::ALL {
        let runs: Vec<&Career> = careers.iter().filter(|c| c.bot == bot).collect();
        if runs.is_empty() {
            continue;
        }
        let n = runs.len();
        let wins = runs.iter().filter(|c| c.won()).count();
        // `weeks_to_rockstar`, not `c.weeks` — since L9 the milestone doesn't
        // end the run, so `c.weeks` is just the horizon's exit week, not when
        // the band actually made it.
        let win_year = median(
            runs.iter()
                .filter_map(|c| c.weeks_to_rockstar)
                .map(|w| w / constants::WEEKS_PER_YEAR)
                .collect(),
        );
        let died = runs.iter().filter(|c| c.ending == Ending::Died).count();
        let broke = runs
            .iter()
            .filter(|c| c.ending == Ending::WentBroke)
            .count();
        let going = runs
            .iter()
            .filter(|c| c.ending == Ending::StillGoing)
            .count();
        println!(
            "{:<15} {:>4} {:>4} {:>4}% {:>7} {:>9} {:>9} {:>9} {:>9} {:>8} {:>8.1} {:>5} {:>5} {:>5}",
            bot.name(),
            n,
            wins,
            wins * 100 / n,
            show(win_year),
            show(median(runs.iter().map(|c| c.final_fame).collect())),
            show(median(runs.iter().map(|c| c.peak_fame).collect())),
            show(median(runs.iter().map(|c| c.final_money).collect())),
            show(median(
                runs.iter().filter_map(|c| c.weeks_to_first_album).collect()
            )),
            show(median(
                runs.iter().filter_map(|c| c.weeks_to_fame_50).collect()
            )),
            runs.iter().map(|c| c.sell_outs).sum::<u32>() as f32 / n as f32,
            died,
            broke,
            going
        );
    }

    println!();
    println!("median fame at year (-- = no run reached that year):");
    for bot in Bot::ALL {
        let runs: Vec<&Career> = careers.iter().filter(|c| c.bot == bot).collect();
        if runs.is_empty() {
            continue;
        }
        let at_year = |year: usize| {
            median(
                runs.iter()
                    .filter_map(|c| c.fame_by_year.get(year - 1).copied())
                    .collect(),
            )
        };
        println!(
            "  {:<15} y1:{:>3}  y2:{:>3}  y3:{:>3}  y5:{:>3}  y8:{:>3}  y12:{:>3}  y15:{:>3}",
            bot.name(),
            show(at_year(1)),
            show(at_year(2)),
            show(at_year(3)),
            show(at_year(5)),
            show(at_year(8)),
            show(at_year(12)),
            show(at_year(15)),
        );
    }

    println!();
    println!("record deals (first signing week is a median over signed runs):");
    for bot in Bot::ALL {
        let runs: Vec<&Career> = careers.iter().filter(|c| c.bot == bot).collect();
        if runs.is_empty() {
            continue;
        }
        let signed_runs = runs.iter().filter(|c| c.deals_signed > 0).count();
        println!(
            "  {:<15} signed in {:>2}/{} runs, first deal wk {:>4}, deals/run {:.2}, med albums {:>2}, med singles {:>2}",
            bot.name(),
            signed_runs,
            runs.len(),
            show(median(
                runs.iter().filter_map(|c| c.first_deal_week).collect()
            )),
            runs.iter().map(|c| c.deals_signed).sum::<u32>() as f32 / runs.len() as f32,
            show(median(runs.iter().map(|c| c.albums).collect())),
            show(median(runs.iter().map(|c| c.singles).collect())),
        );
    }

    // M7 (§F): the v0.7 money-cycle metrics — certifications a career
    // reaches, contract breaches, bankruptcies, and where each bot ends up
    // living. These are the numbers the design's targets are read against.
    println!();
    println!("v0.7 money cycle (median certs; breaches & bankruptcies are totals over all runs):");
    for bot in Bot::ALL {
        let runs: Vec<&Career> = careers.iter().filter(|c| c.bot == bot).collect();
        if runs.is_empty() {
            continue;
        }
        let bankruptcies = runs
            .iter()
            .filter(|c| c.ending == Ending::WentBroke)
            .count();
        let final_tier = median(runs.iter().map(|c| c.final_lifestyle).collect())
            .map(|i| LifestyleTier::ALL[i as usize].label())
            .unwrap_or("--");
        println!(
            "  {:<15} med certs {:>2}, max certs {:>2}, breaches {:>3}, bankruptcies {:>2}/{}, med end home {}",
            bot.name(),
            show(median(runs.iter().map(|c| c.certifications).collect())),
            runs.iter().map(|c| c.certifications).max().unwrap_or(0),
            runs.iter().map(|c| c.breaches).sum::<u32>(),
            bankruptcies,
            runs.len(),
            final_tier,
        );
    }
    println!();
}

fn panic_text(payload: Box<dyn std::any::Any + Send>) -> String {
    if let Some(text) = payload.downcast_ref::<&str>() {
        (*text).to_string()
    } else if let Some(text) = payload.downcast_ref::<String>() {
        text.clone()
    } else {
        "non-string panic payload".to_string()
    }
}

// ---------------------------------------------------------------------------
// CI-safe smoke tests: fast, no flake risk.
// ---------------------------------------------------------------------------

#[test]
fn seeded_worlds_are_reproducible_in_the_harness() {
    let band_names =
        |game: &Game| -> Vec<String> { game.world.bands.iter().map(|b| b.name.clone()).collect() };
    assert_eq!(
        band_names(&seeded_game(42)),
        band_names(&seeded_game(42)),
        "same seed, same scene"
    );
    assert_ne!(
        band_names(&seeded_game(42)),
        band_names(&seeded_game(43)),
        "different seed, different scene"
    );

    // Since Track B seeded the action stream too, an entire short career —
    // not just the opening scene — replays exactly.
    let career_facts = |seed: u64| {
        let career = run_career(Bot::BalancedIndie, seed, 2 * constants::WEEKS_PER_YEAR);
        (
            career.weeks,
            career.final_fame,
            career.peak_fame,
            career.final_money,
            career.albums,
            career.singles,
            career.sell_outs,
            career.weeks_to_first_album,
        )
    };
    assert_eq!(
        career_facts(42),
        career_facts(42),
        "same seed, same policy, same career — to the week and the dollar"
    );
}

#[test]
fn no_bot_panics_or_stalls_inside_five_years() {
    for bot in Bot::ALL {
        for seed in [11u64, 27, 43] {
            let career = run_career(bot, seed, SMOKE_HORIZON);
            // Week jumps are bounded: a break adds 4, a support tour at
            // most 5. Anything larger means the calendar broke.
            assert!(
                career.weeks <= SMOKE_HORIZON + 6,
                "{} on seed {seed} overshot the horizon: week {}",
                bot.name(),
                career.weeks
            );
        }
    }
}

/// §F: "Median signed act recoups the advance before the term's halfway
/// mark." This is the assertion whose absence let issue #20 slide: the M7
/// sales rescale tripled the copies every pressing charges to the ledger
/// while `SALES_INCOME_DIVISOR` kept royalty dollars flat, so below a 30%
/// royalty the ledger grew monotonically and no signed act could ever
/// recoup — and no test noticed. Label-loyalist careers, first deal each:
/// the median ledger-clear point must land before 50% of the term. A deal
/// that never recoups inside the horizon stays in the sample at an
/// over-the-horizon percentage, dragging the median honestly instead of
/// silently vanishing.
#[test]
fn median_signed_act_recoups_the_advance_before_the_terms_halfway_mark() {
    // 8 years, not the 5-year smoke horizon: a first deal signed in year
    // 3-4 needs its full term (up to 156 weeks) inside the window, or a
    // healthy deal gets counted as "never recouped" purely because the
    // clock ran out on the measurement.
    const HORIZON: u32 = 8 * constants::WEEKS_PER_YEAR;
    const SEEDS: u64 = 12;
    let mut recoup_pct: Vec<u32> = Vec::new();
    for seed in 1..=SEEDS {
        let career = run_career(Bot::LabelLoyalist, seed, HORIZON);
        let (Some(signed_week), Some(term)) =
            (career.first_deal_week, career.first_deal_term_weeks)
        else {
            continue;
        };
        let term = u32::from(term.max(1));
        let recoup_week = career.first_deal_recoup_week.unwrap_or(HORIZON + term);
        recoup_pct.push(recoup_week.saturating_sub(signed_week) * 100 / term);
    }
    assert!(
        recoup_pct.len() >= SEEDS as usize * 2 / 3,
        "the label-loyalist should sign on most seeds inside eight years; only {}/{SEEDS} did",
        recoup_pct.len()
    );
    recoup_pct.sort_unstable();
    println!("first-deal recoup points as % of term ({SEEDS} seeds): {recoup_pct:?}");
    let median_pct = recoup_pct[recoup_pct.len() / 2];
    assert!(
        median_pct < 50,
        "§F target: the median signed act must recoup the advance before the term's \
         halfway mark; median recoup point was {median_pct}% of the term ({recoup_pct:?})"
    );
}

/// L-SIM (design §I) — **bidding works, and the major-label path is actually
/// swept.** A fame-95 act with a real catalogue must draw a *bigger* priced
/// offer than the same catalogue as a fame-30 unknown, and must reach the
/// Major tier — the path no career bot exercises (the label-loyalist renews
/// onto its first *indie* forever, a gap flagged repeatedly across the cycle,
/// so majors were never swept in play). We drive the live offer market
/// (`generate_deal_offers`) directly with a synthetic act at each fame, which
/// is exactly the entry point `turn.rs` calls, seeded so it replays.
///
/// This is the *market/integration* echo of the confound-controlled pricing
/// proofs in `world::deals::tests` (a_hotter_act…, a_broke_label_cannot_outbid…,
/// royalty_tracks_reach…, artist_friendly_offers_a_higher_royalty…): those pin
/// the pure `priced_advance`/`priced_royalty` functions holding everything but
/// the one variable fixed; this proves the whole seeded population produces the
/// same shape end-to-end. Fast (offer generation only, no career sim) → CI-safe.
///
/// Measured 2026-07-25 (60 seeds): a fame-95 act drew a Major offer on **57/60**
/// seeds with a median best advance **north of $1M**, versus a fame-30 unknown
/// capped at an indie/boutique best advance of ~$7k — two orders of magnitude
/// smaller. Strategy split cleanly (§G): HitFactory labels led on advance
/// (~$998k median) while ArtistFriendly led on royalty (0.322 vs 0.138). NB
/// **royalty is reach-driven, not a clean function of fame across tiers** — by
/// design (§E) a high-reach major pays the *least* royalty and competes on the
/// advance, a low-reach boutique competes on royalty — so the robust market-
/// level "hotter act, better terms" lever pinned here is the *advance*.
#[test]
fn a_big_act_reaching_a_major_draws_bigger_priced_offers_than_an_unknown() {
    const SEEDS: u64 = 60;
    const UNKNOWN_FAME: u8 = 30;
    const STAR_FAME: u8 = 95;

    fn minimal_release(id: u32, release_type: ReleaseType) -> music::Release {
        music::Release {
            id,
            name: "Catalogue".to_string(),
            release_type,
            release_quality: 60,
            week_released: 1,
            songs_involved_quality_avg: 60,
            active_marketing: Vec::new(),
            marketing_level_achieved: 0,
            initial_sales_score: 0,
            total_income_generated: 0,
            genre: Some(genre::MusicGenre::Rock),
            copies_pressed: 0,
            copies_sold: 0,
            peak_chart_position: None,
            singles_cut: 0,
            certified: 0,
            distribution_channel: None,
            label_market_reach: None,
        }
    }
    // A synthetic act: enough catalogue + buzz to clear the Major interest gate
    // (fame 40 / 1 album / 3 releases / buzz 30) when famous enough.
    fn act(fame: u8) -> Band {
        Band {
            fame,
            genre: genre::MusicGenre::Rock,
            singles_released: (0..4)
                .map(|i| minimal_release(i, ReleaseType::Single))
                .collect(),
            albums_released: (0..6)
                .map(|i| minimal_release(100 + i, ReleaseType::Album))
                .collect(),
            ..Band::default()
        }
    }

    // Per-fame: best advance + best royalty each seed, and how often a Major bit.
    let mut best_adv: std::collections::BTreeMap<u8, Vec<u32>> = std::collections::BTreeMap::new();
    let mut best_roy: std::collections::BTreeMap<u8, Vec<f32>> = std::collections::BTreeMap::new();
    let mut major_offer_seeds = 0u32;
    // Strategy buckets over every offer the star drew, keyed by the offering
    // label's live strategy (§G).
    let mut adv_by_strategy: std::collections::BTreeMap<String, Vec<u32>> =
        std::collections::BTreeMap::new();
    let mut roy_by_strategy: std::collections::BTreeMap<String, Vec<f32>> =
        std::collections::BTreeMap::new();

    // Label state is RNG-free (`seed_labels` is a pure function of
    // `record_labels.json`), so one world serves every seed — `strategy_of` and
    // every input `generate_deal_offers` reads are seed-independent. Only the
    // per-(seed, fame) offer-market rng below varies. Hoisting the world out of
    // the loop keeps the draws byte-identical while paying worldgen once.
    let game = seeded_game(1);
    let strategy_of: std::collections::HashMap<String, LabelStrategy> = game
        .world
        .labels
        .values()
        .map(|l| (l.name.clone(), l.strategy))
        .collect();

    for seed in 1..=SEEDS {
        for fame in [UNKNOWN_FAME, STAR_FAME] {
            let band = act(fame);
            // A fresh rng per (seed, fame) so the two draws are independent and
            // reproducible; the world itself is identical.
            let mut rng = StdRng::seed_from_u64(seed.wrapping_mul(7).wrapping_add(u64::from(fame)));
            let offers = game
                .world
                .generate_deal_offers(&band, &game.data_files, &mut rng);

            best_adv
                .entry(fame)
                .or_default()
                .push(offers.iter().map(|o| o.advance).max().unwrap_or(0));
            best_roy
                .entry(fame)
                .or_default()
                .push(offers.iter().map(|o| o.royalty_rate).fold(0.0f32, f32::max));

            if fame == STAR_FAME {
                if offers.iter().any(|o| o.label_tier == "Major") {
                    major_offer_seeds += 1;
                }
                for o in &offers {
                    if let Some(strategy) = strategy_of.get(&o.label_name) {
                        let key = format!("{strategy:?}");
                        adv_by_strategy
                            .entry(key.clone())
                            .or_default()
                            .push(o.advance);
                        roy_by_strategy.entry(key).or_default().push(o.royalty_rate);
                    }
                }
            }
        }
    }

    let med_u32 = |v: &[u32]| -> u32 {
        let mut v = v.to_vec();
        v.sort_unstable();
        v.get(v.len() / 2).copied().unwrap_or(0)
    };
    let med_f32 = |v: &[f32]| -> f32 {
        let mut v = v.to_vec();
        v.sort_by(|a, b| a.partial_cmp(b).unwrap());
        v.get(v.len() / 2).copied().unwrap_or(0.0)
    };

    let unknown_adv = med_u32(&best_adv[&UNKNOWN_FAME]);
    let star_adv = med_u32(&best_adv[&STAR_FAME]);
    let unknown_roy = med_f32(&best_roy[&UNKNOWN_FAME]);
    let star_roy = med_f32(&best_roy[&STAR_FAME]);

    println!("bidding market, {SEEDS} seeds (median best offer the act drew):");
    println!(
        "  fame {UNKNOWN_FAME:>3} (unknown): best advance ${unknown_adv:>9}, best royalty {unknown_roy:.3}"
    );
    println!(
        "  fame {STAR_FAME:>3} (star)   : best advance ${star_adv:>9}, best royalty {star_roy:.3}"
    );
    println!("  the star drew a Major-tier offer on {major_offer_seeds}/{SEEDS} seeds");
    println!("offers the star drew, by offering-label strategy:");
    for (strategy, advs) in &adv_by_strategy {
        let roys = &roy_by_strategy[strategy];
        println!(
            "  {strategy:<14} n={:>3}  median advance ${:>9}  median royalty {:.3}",
            advs.len(),
            med_u32(advs),
            med_f32(roys),
        );
    }

    // Bidding works: the hotter act commands a bigger advance through the live
    // market (the robust direction — see the note on royalty above).
    assert!(
        star_adv > unknown_adv,
        "a fame-{STAR_FAME} act should draw a bigger advance than the same catalogue at fame {UNKNOWN_FAME}: ${star_adv} vs ${unknown_adv}"
    );
    // The major-label path is actually swept: a big act reaches the majors.
    assert!(
        major_offer_seeds * 2 >= SEEDS as u32,
        "a fame-{STAR_FAME} act should reach the Major tier on most seeds; only {major_offer_seeds}/{SEEDS} drew a Major offer"
    );
    // Era-real scale: the star's advance is at least the era's six figures (it
    // medians north of $1M), not the v0.7 compressed thousands.
    assert!(
        star_adv >= 100_000,
        "a superstar's advance should be era-real (six figures or more), got median ${star_adv}"
    );
    // §G strategy colours the market: a HitFactory leads with the advance, an
    // ArtistFriendly with the royalty. (Confound-controlled proof: deals::tests.)
    if let (Some(hf_adv), Some(af_adv)) = (
        adv_by_strategy.get("HitFactory"),
        adv_by_strategy.get("ArtistFriendly"),
    ) {
        assert!(
            med_u32(hf_adv) > med_u32(af_adv),
            "a HitFactory should lead with a bigger advance than an ArtistFriendly through the market: ${} vs ${}",
            med_u32(hf_adv),
            med_u32(af_adv)
        );
    }
    if let (Some(hf_roy), Some(af_roy)) = (
        roy_by_strategy.get("HitFactory"),
        roy_by_strategy.get("ArtistFriendly"),
    ) {
        assert!(
            med_f32(af_roy) > med_f32(hf_roy),
            "an ArtistFriendly should lead with a better royalty than a HitFactory through the market: {:.3} vs {:.3}",
            med_f32(af_roy),
            med_f32(hf_roy)
        );
    }
}

/// L-SIM (design §I) — **label solvency, gangbuster scale, and mobility over
/// fifteen years.** The signed player (label-loyalist) drives one label's
/// ledger to superstar volume while the rest of the industry lives off its own
/// rosters; across every seed the population must stay
///
/// 1. **solvent** — no label ends underwater (`capital < 0`);
/// 2. **sane** — capital lands in the era's millions, never billions or
///    negative (the wholesale ledger neither hyper-inflates nor collapses);
/// 3. **gangbuster** (§I "the gangbuster target") — a signed, productive
///    superstar's label clears era-real millions, and the player banks era-real
///    millions too (not the v0.7 compressed thousands);
/// 4. **stable with mobility** — majors stay dominant in aggregate, yet at
///    least one label crosses a tier line across the seed set.
///
/// Promoted from the L-ENTITY print-only preview to a pinned assertion sweep
/// (L-SIM owns the calibrated bounds). Measured 2026-07-25 (24 seeds × 15 yrs,
/// release build): **0 insolvent**; leanest label **$25,920** (a static,
/// roster-less boutique — solvent, just tiny), richest **$48.4M** (Atlantic, a
/// Major); the signed player's own label nets **+$19M…+$37M**; the player ends
/// with a **median ~$16.7M** in the bank (cross-checked against the balance
/// report's `med.$`). At least one indie the player supercharges crosses into
/// the Major tier across the set (mobility), while the six seeded majors still
/// hold the aggregate lead (dominance).
///
/// **Distress-floor design-call (refereed here, L-SIM).** The floor is kept at
/// $0 (an intended safety valve, not natural attrition — see
/// `game::tests::labels::mna_resolves_injected_distress_within_the_fold_horizon`
/// and `constants::DISTRESS_CAPITAL_FLOOR`). In natural play the wholesale
/// ledger only *grows* labels, so nobody folds; that is precisely why this
/// sweep sees 0 insolvent and 0 folds. §F's "failures occur across the seed
/// set" is exercised by the injected-distress sweep; the *rarity* half —
/// "failure is not the median label's fate" — is what the 0 natural folds here
/// pin. Raising the floor to force attrition would falsely distress solvent-
/// but-lean boutiques and risk the validated solvency, so it was rejected.
#[test]
#[ignore = "long balance sweep: cargo test -- --ignored --nocapture"]
fn label_capital_trajectories_over_fifteen_years() {
    const HORIZON: u32 = 15 * constants::WEEKS_PER_YEAR;
    const SEEDS: u64 = 24;

    fn tier_rank(t: LabelTier) -> u8 {
        match t {
            LabelTier::Boutique => 0,
            LabelTier::Independent => 1,
            LabelTier::Major => 2,
        }
    }

    let mut player_label: Vec<(String, i64, i64)> = Vec::new(); // (name, seeded, end)
    let mut player_money: Vec<i32> = Vec::new();
    let mut signed_at_end = 0u32;
    let mut min_capital_any = i64::MAX;
    let mut max_capital_any = i64::MIN;
    let mut insolvent_instances = 0u32;
    // Mobility + dominance bookkeeping.
    let mut tier_changes = 0u32;
    let mut promotions = 0u32;
    let mut richest_is_major = 0u32;
    let mut cap_by_tier = [0i64; 3]; // indexed by tier_rank: [Boutique, Indie, Major]

    for seed in 1..=SEEDS {
        let mut game = seeded_game(seed);
        let seeded: std::collections::BTreeMap<u32, (i64, LabelTier)> = game
            .world
            .labels
            .iter()
            .map(|(id, l)| (*id, (l.capital, l.tier)))
            .collect();
        let _career = drive(Bot::LabelLoyalist, &mut game, HORIZON);

        player_money.push(game.player.money);
        if let Some(id) = game.band.current_deal().and_then(|d| d.label_id)
            && let Some(label) = game.world.labels.get(&id)
        {
            signed_at_end += 1;
            let start = seeded.get(&id).map(|(c, _)| *c).unwrap_or(0);
            player_label.push((label.name.clone(), start, label.capital));
        }

        let mut richest: Option<(i64, LabelTier)> = None;
        for (id, label) in &game.world.labels {
            min_capital_any = min_capital_any.min(label.capital);
            max_capital_any = max_capital_any.max(label.capital);
            if label.capital < 0 {
                insolvent_instances += 1;
            }
            cap_by_tier[tier_rank(label.tier) as usize] += label.capital.max(0);
            if let Some((_, seed_tier)) = seeded.get(id)
                && *seed_tier != label.tier
            {
                tier_changes += 1;
                if tier_rank(label.tier) > tier_rank(*seed_tier) {
                    promotions += 1;
                }
            }
            if richest.is_none_or(|(cap, _)| label.capital > cap) {
                richest = Some((label.capital, label.tier));
            }
        }
        if matches!(richest, Some((_, LabelTier::Major))) {
            richest_is_major += 1;
        }
    }

    player_label.sort_by_key(|&(_, _, end)| end);
    println!(
        "player's label capital after 15 yrs (label-loyalist, {SEEDS} seeds; signed at end: {signed_at_end}):"
    );
    for (name, start, end) in &player_label {
        println!(
            "  {name:<22} seeded ${start:>12}  ->  end ${end:>13}  (Δ ${:>+13})",
            end - start
        );
    }
    player_money.sort_unstable();
    let median_player_money = player_money[player_money.len() / 2];
    println!(
        "all labels/seeds at end: lowest capital ${min_capital_any}, highest ${max_capital_any}, insolvent instances (capital<0): {insolvent_instances}"
    );
    println!("player's own bank at end (median over {SEEDS} seeds): ${median_player_money}");
    println!(
        "aggregate capital by tier: boutique ${}, independent ${}, major ${}",
        cap_by_tier[0], cap_by_tier[1], cap_by_tier[2]
    );
    println!(
        "tier mobility across the set: {tier_changes} labels changed tier ({promotions} promotions); richest label was a Major on {richest_is_major}/{SEEDS} seeds"
    );

    // (1) Solvent: nobody underwater.
    assert_eq!(
        insolvent_instances, 0,
        "the industry must stay solvent in natural play — {insolvent_instances} label-instances ended with negative capital"
    );
    assert!(
        min_capital_any > 0,
        "the leanest label should still be above water in natural play (lowest ${min_capital_any})"
    );
    // (2) Sane band: era millions, never runaway billions.
    assert!(
        max_capital_any < 1_000_000_000,
        "no label should hyper-inflate into the billions (richest ${max_capital_any}); the ledger is decompressed, not broken"
    );
    // (3) Gangbuster: the signed superstar's label clears era-real millions, and
    // the player banks era-real millions — not the v0.7 compressed thousands.
    assert!(
        signed_at_end * 2 >= SEEDS as u32,
        "the label-loyalist should be signed at the horizon on most seeds; only {signed_at_end}/{SEEDS}"
    );
    for (name, start, end) in &player_label {
        assert!(
            *end > 1_000_000,
            "the signed superstar's label {name} should clear era-real millions, ended ${end}"
        );
        assert!(
            end > start,
            "a productive signed act must leave its label richer than it seeded it ({name}: ${start} -> ${end})"
        );
    }
    assert!(
        median_player_money > 1_000_000,
        "the player as a recouped superstar should bank era-real millions (median ${median_player_money}), not the old compressed thousands"
    );
    // (4) Stable with mobility: majors dominant in aggregate, but the ladder moves.
    assert!(
        cap_by_tier[2] > cap_by_tier[1] && cap_by_tier[2] > cap_by_tier[0],
        "majors should hold the aggregate capital lead (boutique ${}, indie ${}, major ${})",
        cap_by_tier[0],
        cap_by_tier[1],
        cap_by_tier[2]
    );
    assert!(
        tier_changes > 0,
        "the industry should show tier mobility across the seed set — no label changed tier in 15 years on any seed"
    );
    assert!(
        richest_is_major * 2 >= SEEDS as u32,
        "a Major should top the capital table on most seeds (only {richest_is_major}/{SEEDS})"
    );
}

/// With both story-event taps silenced (they can hand out fame), a band
/// that never records is fully deterministic: fame never exceeds
/// `LIVE_FAME_BASE_CAP`, on every seed, every time.
///
/// v0.6 note: `gig_or_rest` now gates on the stress/health guards (L3) —
/// gigging raises stress (§B), and `self_care` forces a real break once
/// stress crosses `STRESS_CEILING` well before the gig guard itself would
/// ever fire, so breaks are interspersed but the grind still saturates the
/// base live cap. Re-check the balance in L10's sim lab.
#[test]
fn a_pure_gig_grinder_stalls_at_the_base_live_cap() {
    for seed in [3u64, 5, 8] {
        let mut game = seeded_game(seed);
        suppress_story_events(&mut game);
        while !game.is_game_over() && game.week <= SMOKE_HORIZON {
            let week_before = game.week;
            let action = Bot::GigGrinder.decide(&game);
            if game.process_turn(action).is_err() {
                game.process_turn(GameAction::LazeAround)
                    .expect("lazing is always allowed");
            }
            game.take_turn_log();
            // Adapted for v0.6 (L3): gigging now costs stress (§B), so
            // `self_care` can force a real `TakeBreak` (jumps `BREAK_WEEKS`)
            // once stress crosses `STRESS_CEILING` — event suppression only
            // needs `last_event_week` re-parked after every turn, which it
            // is below, regardless of how many weeks a turn advances.
            assert!(
                game.week <= week_before + BREAK_WEEKS,
                "event suppression re-parks every turn; no action should ever jump further than a break (seed {seed})"
            );
            game.events.last_event_week = u32::MAX;
            assert!(
                game.band.fame <= LIVE_FAME_BASE_CAP,
                "gigging pushed fame to {} past the live cap at week {} (seed {seed})",
                game.band.fame,
                game.week
            );
        }
        assert_eq!(
            game.band.fame, LIVE_FAME_BASE_CAP,
            "gig-grinding should saturate the base live cap (seed {seed})"
        );
    }
}

/// §A asks whether laze-wear ever actually kills anyone. It does — but only
/// on a genuinely neglectful policy (never work, never break, never see a
/// doctor), and only after a couple of years of it: stress stays pinned near
/// 0 the whole time (lazing is the stress-relief action), so health decay is
/// driven by `LAZE_WEAR_THRESHOLD_WEEKS` alone, not the stress-driven term.
/// None of the four bots in the balance lab behave this way (`self_care`
/// forces a break long before this), so this is a dedicated worst-case, not
/// a bot career — it's the "is the death path real" half of the design's own
/// question, the other half (do the four bots avoid it) being answered by
/// `died == 0` across every bot/seed in the long sweep.
#[test]
fn sustained_lazing_alone_eventually_kills_a_healthy_player() {
    let mut game = seeded_game(7);
    suppress_story_events(&mut game);
    let mut weeks_lazed = 0u32;
    while !game.is_game_over() && weeks_lazed < 500 {
        game.process_turn(GameAction::LazeAround)
            .expect("lazing is always allowed");
        game.take_turn_log();
        game.events.last_event_week = u32::MAX;
        weeks_lazed += 1;
    }
    assert!(
        game.is_game_over(),
        "500 straight weeks of lazing should eventually end the game"
    );
    assert_eq!(
        game.player.health, 0,
        "the ending should be health hitting zero from laze-wear, not going broke \
         (lazing neither earns nor spends money)"
    );
    println!(
        "pure lazing killed the player at week {} ({:.1} years) — confirms 'turtling is \
         safe for months, not years' (design §A) rather than being unreachable",
        game.week,
        game.week as f32 / constants::WEEKS_PER_YEAR as f32
    );
}

/// §B asks whether reception distributions land the designed tiers: a
/// starting band mostly rough/solid, a 100-skill band reliably great or
/// transcendent. `band_base` (0.7·skill + 0.3·live_performance) is the
/// dominant, non-random term by design, so the tiers should be provable from
/// the formula's own arithmetic, not just "usually" true over a sample — this
/// pins both the arithmetic and the sampled distribution in one test.
#[test]
fn reception_lands_the_designed_verdict_tiers_by_skill() {
    fn band_with(skill: u8, live_performance: u8) -> Band {
        let mut band = Band {
            skill,
            ..Band::default()
        };
        for member in &mut band.members {
            member.skill = skill;
        }
        band.reputation.live_performance = live_performance;
        band
    }

    /// (rough, solid, great, transcendent) counts over `samples` rolls at a
    /// steady mid-career condition (stress 30, health 80 — neither condition
    /// penalty active) and average creativity (50).
    fn verdict_counts(band: &Band, seed: u64, samples: u32) -> [u32; 4] {
        let mut rng = StdRng::seed_from_u64(seed);
        let mut counts = [0u32; 4];
        for _ in 0..samples {
            let reception = shows::compute_reception(band, 30, 80, 1.0, 50, &mut rng);
            let idx = match shows::ShowVerdict::from_reception(reception) {
                shows::ShowVerdict::Rough => 0,
                shows::ShowVerdict::Solid => 1,
                shows::ShowVerdict::Great => 2,
                shows::ShowVerdict::Transcendent => 3,
            };
            counts[idx] += 1;
        }
        counts
    }

    let samples = 2_000;
    // Starting band (Band::default()'s own numbers: skill ~25, live_perf 15).
    let starting = verdict_counts(&band_with(25, 15), 900, samples);
    // A credible mid-career band.
    let mid = verdict_counts(&band_with(60, 50), 901, samples);
    // A maxed-out band.
    let high = verdict_counts(&band_with(100, 90), 902, samples);

    println!("reception verdict distribution by skill tier ({samples} samples each):");
    println!(
        "  starting (skill 25, live_perf 15): rough {} solid {} great {} transcendent {}",
        starting[0], starting[1], starting[2], starting[3]
    );
    println!(
        "  mid      (skill 60, live_perf 50): rough {} solid {} great {} transcendent {}",
        mid[0], mid[1], mid[2], mid[3]
    );
    println!(
        "  high     (skill 100, live_perf 90): rough {} solid {} great {} transcendent {}",
        high[0], high[1], high[2], high[3]
    );

    // Starting band: band_base = 0.7*25 + 0.3*15 = 22; even the best-case
    // roll (+10 variance, +10 creativity upside) tops out at 42 — great
    // (>=70) and transcendent (>=85) are arithmetically unreachable.
    assert_eq!(
        starting[2] + starting[3],
        0,
        "a starting band's reception ceiling (42) is below the great threshold (70); \
         got great {} transcendent {} out of {samples}",
        starting[2],
        starting[3]
    );

    // High band: band_base = 0.7*100 + 0.3*90 = 97; even the worst-case roll
    // (-10 variance, 0 creativity upside, no condition bonus in play) is 87,
    // still above the transcendent threshold (85) — every single night.
    assert_eq!(
        high[3], samples,
        "a maxed-out band's reception floor (87) is above the transcendent threshold (85); \
         every night should land transcendent, got {}/{samples}",
        high[3]
    );

    // Mid band: band_base = 0.7*60 + 0.3*50 = 57; range is [47, 77] — always
    // at least solid, sometimes great, never rough or transcendent.
    assert_eq!(
        mid[0], 0,
        "a mid-tier band's reception floor (47) should never be rough"
    );
    assert_eq!(
        mid[3], 0,
        "a mid-tier band's reception ceiling (77) should never reach transcendent (85)"
    );
    assert!(
        mid[2] > 0,
        "a mid-tier band should occasionally catch a great night on a good roll"
    );
}

/// §F asks what fraction of weeks actually fire an incident at the new 35%
/// cadence. `try_trigger_event`'s roll is unconditional — eligibility only
/// filters *which* incident gets picked afterward (`events_apply.rs`), so
/// tracking `events.last_event_week` against the current week across a long,
/// survivable career (gig-grinder: `self_care` keeps it alive the whole way)
/// measures the cadence directly, independent of which incidents are
/// eligible for this band.
#[test]
fn incident_cadence_matches_the_designed_weekly_chance() {
    let mut game = seeded_game(31);
    let weeks = 500;
    let mut fired = 0u32;
    let mut actual_weeks = 0u32;
    for _ in 0..weeks {
        if game.is_game_over() {
            break;
        }
        let action = Bot::GigGrinder.decide(&game);
        if game.process_turn(action).is_err() {
            game.process_turn(GameAction::LazeAround)
                .expect("lazing is always allowed");
        }
        game.take_turn_log();
        actual_weeks += 1;
        if game.events.last_event_week == game.week {
            fired += 1;
        }
    }
    let rate = fired * 100 / actual_weeks.max(1);
    println!(
        "incidents fired in {fired}/{actual_weeks} weeks ({rate}%); design target {INCIDENT_WEEKLY_CHANCE_PERCENT}%"
    );
    // A generous band around the designed rate — one seed's sample, not an
    // exact-probability proof (that belongs to a unit test over raw rolls,
    // not the sim lab).
    assert!(
        (25..=45).contains(&(rate as i32)),
        "observed incident cadence {rate}% is far from the designed {INCIDENT_WEEKLY_CHANCE_PERCENT}%"
    );
}

/// §C's ramp is its own clock (`Game::decay_streak`): decay always opens at
/// -1 and steps gently to the flat -5, even when fame crosses into a
/// shorter grace tier mid-decline (the edge L10 originally quantified —
/// fixed by the coordinator with a serialized decay-onset counter).
#[test]
fn fame_ramp_onset_is_gentle_even_across_grace_tier_boundaries() {
    let mut game = seeded_game(11);
    suppress_story_events(&mut game);
    game.band.fame = 92;
    game.band.peak_fame = 92; // peak >= 90 -> floor 60, comfortably below 92.

    let mut trace: Vec<(u32, u8, i16)> = Vec::new();
    let mut previous_fame = game.band.fame;
    for _ in 0..400 {
        if game.is_game_over() || game.band.fame <= 60 {
            break;
        }
        game.process_turn(GameAction::LazeAround)
            .expect("lazing is always allowed");
        game.take_turn_log();
        game.events.last_event_week = u32::MAX;
        let delta = previous_fame as i16 - game.band.fame as i16;
        if delta != 0 {
            trace.push((game.week, game.band.fame, delta));
        }
        previous_fame = game.band.fame;
    }

    println!("fame decay trace from a fame-92 band gone quiet (week, fame-after, weekly delta):");
    for (week, fame, delta) in &trace {
        println!("  week {week:>4}: fame {fame:>3} (-{delta})");
    }

    // Containment: whatever the onset looks like, no single week should ever
    // drop fame faster than the designed flat rate.
    for (week, fame, delta) in &trace {
        assert!(
            *delta <= FAME_RAMP_MAX_DECAY as i16,
            "week {week} dropped fame to {fame} (-{delta}), faster than the flat rate {}",
            FAME_RAMP_MAX_DECAY
        );
    }

    // The onset must be gentle everywhere: decay opens at -1 and never
    // steps up by more than 1 per week, tier boundaries notwithstanding.
    let (first_week, first_fame, first_delta) = trace.first().expect("the band must decay");
    assert_eq!(
        *first_delta, 1,
        "decay must open at -1 (week {first_week}, fame {first_fame}, got -{first_delta})"
    );
    for pair in trace.windows(2) {
        let (_, _, prev) = pair[0];
        let (week, fame, next) = pair[1];
        assert!(
            next <= prev + 1,
            "ramp jumped from -{prev} to -{next} at week {week} (fame {fame})"
        );
    }
}

// ---------------------------------------------------------------------------
// Long sweeps: run by hand, report the numbers.
// ---------------------------------------------------------------------------

/// The whole-population balance report — and the sweep that pins §I's
/// cross-bot targets. Beyond "no bot ever panics", it asserts:
///
/// - **Scale re-validation (§F): a signed act nets ≥ an indie over a career.**
///   The label-loyalist's median 15-year net worth must top both the eternal
///   indie-lifer's and the balanced-indie's — signing is a real upgrade, not a
///   trap. Measured 2026-07-25 (release, 60 seeds): loyalist **$16.7M** vs
///   indie-lifer **$3.5M** vs balanced-indie **$3.9M**.
/// - **No mass bankruptcy (§F "nobody dies of money").** Every bot keeps its
///   bankruptcy (`WentBroke`) rate a clear minority and its deaths rare — the
///   decompressed economy is survivable, not a death spiral. Measured: broke
///   4–8/60 across bots (worst: studio-rat 8/60 ≈ 13%); died 0 everywhere
///   except the road-dog's 2/60 touring deaths.
/// - **Certification frequency — ratified (design-call, L-SIM).** A signed
///   superstar now certifies a *median 71 records* (max 75) over a 15-year
///   career, up from the v0.7 "median 3 / 1–3 silver" target. This is the
///   **intended** direction of the cycle — records sell worldwide off a label's
///   reach, so gangbuster money is literal. The retired v0.7 target text lives
///   on in `docs/DESIGN-v0.7-money-cycle.md` (§D/§I) and `CHANGELOG.md`; those
///   are not L-SIM's to edit (docs / L-CLOSE own them) and are flagged for
///   reconciliation at cycle close. We ratify the new magnitude with a floor an
///   order above the old target and a sane ceiling — *not* by raising the copy
///   thresholds, which the user explicitly wanted lower-friction, not higher.
///   (Only the label-loyalist certifies at volume: an unsigned indie's mail-
///   order reach never clears the 50k Silver bar, so the other bots median 0 —
///   itself the proof that a label's international reach is what sells records.)
#[test]
#[ignore = "long balance sweep: cargo test -- --ignored --nocapture"]
fn fifteen_year_careers_never_panic_and_print_the_balance_report() {
    let mut careers = Vec::new();
    let mut panics = Vec::new();
    for bot in Bot::ALL {
        for seed in 1..=SWEEP_SEEDS {
            match catch_unwind(AssertUnwindSafe(|| run_career(bot, seed, SWEEP_HORIZON))) {
                Ok(career) => careers.push(career),
                Err(payload) => panics.push((bot.name(), seed, panic_text(payload))),
            }
        }
    }
    print_report(&careers);
    assert!(panics.is_empty(), "sim panicked: {panics:?}");

    // --- §I cross-bot assertions (measure-then-pin; see the doc comment) ---
    let runs_of = |bot: Bot| -> Vec<&Career> { careers.iter().filter(|c| c.bot == bot).collect() };
    let median_money = |bot: Bot| -> i32 {
        median(runs_of(bot).iter().map(|c| c.final_money).collect()).unwrap_or(0)
    };
    let median_certs = |bot: Bot| -> usize {
        median(runs_of(bot).iter().map(|c| c.certifications).collect()).unwrap_or(0)
    };

    // No mass bankruptcy: broke a clear minority, deaths rare — for every bot.
    for bot in Bot::ALL {
        let runs = runs_of(bot);
        let broke = runs
            .iter()
            .filter(|c| c.ending == Ending::WentBroke)
            .count();
        let died = runs.iter().filter(|c| c.ending == Ending::Died).count();
        assert!(
            broke * 3 < runs.len(),
            "{} went broke {broke}/{} times — bankruptcy should be a clear minority, not the economy's default",
            bot.name(),
            runs.len()
        );
        assert!(
            died * 10 <= runs.len(),
            "{} died {died}/{} times — the decompressed economy must not be a death spiral",
            bot.name(),
            runs.len()
        );
    }

    // Scale re-validation (§F): signing nets ≥ staying indie, over a full career.
    let loyalist = median_money(Bot::LabelLoyalist);
    let indie = median_money(Bot::IndieLifer);
    let balanced = median_money(Bot::BalancedIndie);
    assert!(
        loyalist >= indie && loyalist >= balanced,
        "a signed act should net at least an indie over a career: label-loyalist median ${loyalist} vs indie-lifer ${indie} / balanced-indie ${balanced}"
    );

    // Certification frequency ratified: the signed superstar certifies at
    // gangbuster volume — an order of magnitude above the retired v0.7 "med 3",
    // bounded below the absurd (it cannot certify more records than it ships).
    let certs = median_certs(Bot::LabelLoyalist);
    assert!(
        (30..=150).contains(&certs),
        "signed-superstar certification frequency out of the ratified band: median {certs} (expected ~71, an order above the retired v0.7 'median 3' target)"
    );
}

/// L-SIM (design §F/§I) — **matched-lifestyle upkeep is a sane share of
/// income.** The homebody matches its lifestyle tier to what it can afford, so
/// at its settled tier the weekly upkeep should be a real bite of income
/// (§F target: 10–25%), not a rounding error and not a death spiral. We drive
/// a homebody to maturity, then measure over a two-year window of single-week
/// gig/rest turns only: with no recording, touring, deposits, doctor visits,
/// or (suppressed) incidents, the *only* money out is lifestyle upkeep, so
/// `income = Δmoney + upkeep` is exact each week. The tier is free to float
/// (upkeep is read from the current tier every week, so an eviction mid-window
/// keeps the arithmetic exact — and settling downward is itself the game's own
/// "match to affordability").
///
/// Measured 2026-07-25 (24 seeds): **median 21.4%**, every seed 20.4–22.3% —
/// the homebody settles in a Mansion and spends ~21% of a ~$4M/2yr income on
/// upkeep. The band is asserted, not a point value — the homebody is an
/// *unsigned* indie (mail-order reach, no certifications), so its income is
/// indie-scale and the ratio lands in the designed 10–25% range rather than
/// the near-zero a signed superstar's income would produce.
#[test]
#[ignore = "long balance sweep: cargo test -- --ignored --nocapture"]
fn matched_lifestyle_upkeep_is_a_sane_share_of_income() {
    const SEEDS: u64 = 24;
    const CAREER: u32 = 15 * constants::WEEKS_PER_YEAR;
    const WINDOW: u32 = 2 * constants::WEEKS_PER_YEAR;

    let mut ratios: Vec<f64> = Vec::new();
    let mut rows: Vec<(u64, &'static str, i64, i64, f64)> = Vec::new();

    for seed in 1..=SEEDS {
        let mut game = seeded_game(seed);
        drive(Bot::Homebody, &mut game, CAREER);
        // A career that already ended (broke/died) has no steady income to
        // measure — the survival property is the balance report's job.
        if game.is_game_over() {
            continue;
        }
        suppress_story_events(&mut game);

        let mut total_upkeep = 0i64;
        let mut total_income = 0i64;
        for _ in 0..WINDOW {
            if game.is_game_over() {
                break;
            }
            let upkeep = i64::from(game.player.lifestyle.upkeep_per_week());
            let before = game.player.money;
            // Single-week actions only (gig or laze): each advances exactly one
            // week and charges exactly one week of upkeep, keeping the money
            // decomposition exact. No TakeBreak (multi-week), no discretionary
            // spend.
            let action = gig_or_rest(&game);
            if game.process_turn(action).is_err() {
                game.process_turn(GameAction::LazeAround)
                    .expect("lazing is always allowed");
            }
            game.take_turn_log();
            game.events.last_event_week = u32::MAX;
            let after = game.player.money;
            total_upkeep += upkeep;
            total_income += i64::from(after - before) + upkeep;
        }

        let tier = game.player.lifestyle.label();
        if total_income > 0 {
            let ratio = total_upkeep as f64 / total_income as f64;
            ratios.push(ratio);
            rows.push((seed, tier, total_upkeep, total_income, ratio));
        }
    }

    println!(
        "matched-lifestyle upkeep as a share of income (homebody, {SEEDS} seeds, 2-yr window):"
    );
    for (seed, tier, upkeep, income, ratio) in &rows {
        println!(
            "  seed {seed:>2}: end tier {tier:<12} upkeep ${upkeep:>8}, income ${income:>9}  ->  {:.1}%",
            ratio * 100.0
        );
    }
    assert!(
        !ratios.is_empty(),
        "no surviving homebody career produced a measurable income window"
    );
    ratios.sort_by(|a, b| a.partial_cmp(b).unwrap());
    let median = ratios[ratios.len() / 2];
    println!("median upkeep/income ratio: {:.1}%", median * 100.0);

    // §F target: matched-lifestyle upkeep is 10–25% of income. Asserted on the
    // median at the design floor with slack above 25% for seed spread — this
    // catches upkeep going trivial (< 10%) or turning into a spiral (> 30%).
    assert!(
        (0.10..=0.30).contains(&median),
        "matched-lifestyle upkeep should be ~10–25% of income (§F); median was {:.1}%",
        median * 100.0
    );
}

#[test]
#[ignore = "long balance sweep: cargo test -- --ignored --nocapture"]
fn balanced_indie_reaches_the_win_screen_by_year_twelve_on_most_seeds() {
    let mut wins = 0u64;
    let mut win_weeks = Vec::new();
    for seed in 1..=WIN_TARGET_SEEDS {
        let career = run_career(Bot::BalancedIndie, seed, WIN_TARGET);
        if career.won() {
            wins += 1;
            // `weeks_to_rockstar`, not `career.weeks`: since L9 the milestone
            // doesn't end the run, `career.weeks` is just wherever the
            // 12-year horizon cut the loop off, not the week the band
            // actually reached fame 90 + 5 albums.
            if let Some(w) = career.weeks_to_rockstar {
                win_weeks.push(w);
            }
        }
    }
    println!(
        "balanced-indie won {wins}/{WIN_TARGET_SEEDS} careers by year 12 (median week the milestone was actually reached: {})",
        show(median(win_weeks))
    );
    assert!(
        wins * 2 > WIN_TARGET_SEEDS,
        "balanced-indie won only {wins}/{WIN_TARGET_SEEDS} careers by year 12"
    );
}

/// §B — tour box office by skill tier, holding fame (and so the tour-wide
/// pot and cost) fixed and varying only member skill/live_performance, so
/// any spread in total gross is attributable to reception (attendance
/// factor + momentum), not to fame differences. L3 measured a new band's
/// tour gross at ~85% of the pre-0.6 ballpark; this checks that ballpark
/// holds — or at least stays sane — across tiers, not just for one band.
#[test]
#[ignore = "long balance sweep: cargo test -- --ignored --nocapture"]
fn tour_gross_by_skill_tier_sweep() {
    const FIXED_FAME: u8 = 40; // clears the Tour bus gate (25); same rig/weeks/cost for every run below.
    const FIXED_RIG: TourRig = TourRig::Bus;
    const FIXED_WEEKS: u8 = 2;

    fn tour_at(seed: u64, skill: u8, live_performance: u8) -> (u8, u32, f32, f32) {
        let mut game = seeded_game(seed);
        suppress_story_events(&mut game);
        game.band.fame = FIXED_FAME;
        game.band.peak_fame = FIXED_FAME;
        game.band.skill = skill;
        for member in &mut game.band.members {
            member.skill = skill;
        }
        game.band.reputation.live_performance = live_performance;
        game.player.stress = 20;
        game.player.health = 90;
        game.player.money = 100_000; // never let the tour cost gate the run

        let region_index = game
            .get_sorted_regions()
            .iter()
            .position(|(_, _, _, _, _, fame_req)| *fame_req <= FIXED_FAME)
            .expect("at least one region open at fame 40");

        game.process_turn(GameAction::GoOnTour(region_index, FIXED_RIG, FIXED_WEEKS))
            .expect("tour should be affordable and unblocked");
        let report = game.last_tour_report.expect("tour produces a report");

        // Momentum isn't stored on the report — replay it from each show's
        // reception (§B — Momentum) to see how far a tour's word of mouth
        // actually swings.
        let mut momentum = MOMENTUM_START;
        let mut momentum_min = MOMENTUM_START;
        let mut momentum_max = MOMENTUM_START;
        for row in &report.rows {
            let verdict = shows::ShowVerdict::from_reception(row.reception);
            momentum = shows::apply_momentum_delta(momentum, verdict);
            momentum_min = momentum_min.min(momentum);
            momentum_max = momentum_max.max(momentum);
        }

        (
            report.avg_reception,
            report.total_gross,
            game.band.fame as f32 - FIXED_FAME as f32,
            momentum_max - momentum_min,
        )
    }

    println!("tour gross by skill tier, fame held at {FIXED_FAME} (avg over 20 seeds):");
    for (label, skill, live_performance) in
        [("starting", 25u8, 15u8), ("mid", 60, 50), ("high", 100, 90)]
    {
        let runs: Vec<(u8, u32, f32, f32)> = (1..=20)
            .map(|seed| tour_at(seed, skill, live_performance))
            .collect();
        let n = runs.len() as f32;
        let avg_reception = runs.iter().map(|r| r.0 as f32).sum::<f32>() / n;
        let avg_gross = runs.iter().map(|r| r.1 as f32).sum::<f32>() / n;
        let avg_momentum_spread = runs.iter().map(|r| r.3).sum::<f32>() / n;
        println!(
            "  {label:<9} (skill {skill:>3}, live_perf {live_performance:>3}): avg reception {avg_reception:.1}, avg total gross ${avg_gross:.0}, avg momentum spread {avg_momentum_spread:.3}"
        );
    }
}

/// §A/§F — tour profitability by rig, fame, and **regional fame**. A single
/// tour's money delta is its box office minus its quoted cost (box office is
/// per-show take, untouched by M10's record-sales presence model), so this
/// reads the design target directly: the van should clear a profit for a
/// small act, while a full-production rig only pays off once the act is big
/// enough to fill the bigger rooms its capacity multiplier books. Confounders
/// are minimized — a fresh band (no catalog tail), Squat lifestyle (no
/// upkeep), cash on hand so the cost never gates the booking.
///
/// The regional-fame axis was added by the tour-economics pass (T-RIG) and is
/// the reason the v0.8 reading was wrong: `regional_fame` is the dominant term
/// in the gross pot (`audience = fame/3 + regional_fame`, 0-100 against a
/// fame contribution capped at 33), but the sweep only ever measured a *cold*
/// territory at 0. "Full production never profits at any fame" was really
/// "a first visit with the big show doesn't pay" — true, intended, and not
/// the whole picture.
#[test]
#[ignore = "long balance sweep: cargo test -- --ignored --nocapture"]
fn tour_profitability_by_rig_and_fame_sweep() {
    fn net_at(seed: u64, fame: u8, regional_fame: u8, rig: TourRig, weeks: u8) -> Option<i64> {
        let mut game = seeded_game(seed);
        suppress_story_events(&mut game);
        game.band.fame = fame;
        game.band.peak_fame = fame;
        game.band.skill = 50;
        for member in &mut game.band.members {
            member.skill = 50;
        }
        game.band.reputation.live_performance = 40;
        game.player.stress = 20;
        game.player.health = 90;
        game.player.money = 1_000_000; // cost never gates the booking

        let regions = game.get_sorted_regions();
        let region_index = regions
            .iter()
            .position(|(_, _, _, _, _, fame_req)| fame >= *fame_req)?;
        if regional_fame > 0 {
            let (country_key, region_key, ..) = &regions[region_index];
            game.regional_fame
                .insert(format!("{country_key}:{region_key}"), regional_fame);
        }
        if !game.rig_is_available(rig) || !game.tour_length_is_available(weeks) {
            return None;
        }
        let money_before = game.player.money;
        game.process_turn(GameAction::GoOnTour(region_index, rig, weeks))
            .ok()?;
        // Net of the whole turn: box office earned minus the tour cost
        // spent (Squat upkeep is 0, no catalog to pay a tail).
        Some(game.player.money as i64 - money_before as i64)
    }

    println!(
        "tour net profit (box office − cost) by rig and fame, 2-week tour, cold territory \
         (regional fame 0), avg over 20 seeds:"
    );
    let fames = [25u8, 55, 80];
    // (fame, rig) -> avg net on a cold territory, for the rigs available there.
    let mut avgs: Vec<(u8, TourRig, f32)> = Vec::new();
    for fame in fames {
        for rig in TourRig::ALL {
            let nets: Vec<i64> = (1..=20u64)
                .filter_map(|seed| net_at(seed, fame, 0, rig, 2))
                .collect();
            if nets.is_empty() {
                println!(
                    "  fame {fame:>3}  {:<16}: (rig gated — unavailable at this fame)",
                    rig.label()
                );
                continue;
            }
            let avg = nets.iter().sum::<i64>() as f32 / nets.len() as f32;
            println!(
                "  fame {fame:>3}  {:<16}: avg net ${avg:>+9.0}  ({} seeds)",
                rig.label(),
                nets.len()
            );
            avgs.push((fame, rig, avg));
        }
    }

    // The real superstar tours at 95-100; keep those rows measured so the top
    // of the fame range can't silently drift.
    for fame in [95u8, 100] {
        for rig in TourRig::ALL {
            let nets: Vec<i64> = (1..=20u64)
                .filter_map(|seed| net_at(seed, fame, 0, rig, 2))
                .collect();
            if nets.is_empty() {
                continue;
            }
            let avg = nets.iter().sum::<i64>() as f32 / nets.len() as f32;
            println!(
                "  fame {fame:>3}  {:<16}: avg net ${avg:>+9.0}  ({} seeds)",
                rig.label(),
                nets.len()
            );
            avgs.push((fame, rig, avg));
        }
    }

    // T-RIG (tour-economics pass, resolving the v0.8 "full production is a
    // permanent money-loser" ruling): the same rigs on a territory the act has
    // actually built. This is the axis the old sweep never varied, and where
    // the rig ladder has to pay off — §A's "big enough to fill the rooms it
    // books" is a statement about *this* number, not about fame alone.
    println!("\nsame tours by regional fame in the territory (fame 80, 2 weeks):");
    // (regional fame, rig) -> avg net.
    let mut warm: Vec<(u8, TourRig, f32)> = Vec::new();
    for regional_fame in [0u8, 25, 50, 75, 100] {
        for rig in TourRig::ALL {
            let nets: Vec<i64> = (1..=20u64)
                .filter_map(|seed| net_at(seed, 80, regional_fame, rig, 2))
                .collect();
            if nets.is_empty() {
                continue;
            }
            let avg = nets.iter().sum::<i64>() as f32 / nets.len() as f32;
            println!(
                "  regional {regional_fame:>3}%  {:<16}: avg net ${avg:>+9.0}  ({} seeds)",
                rig.label(),
                nets.len()
            );
            warm.push((regional_fame, rig, avg));
        }
    }

    // T-LEN: the same tour at every bookable length, on a territory the act
    // has built (so every rig is somewhere it belongs — see the softer
    // moderate-territory probe below). The gross pot had no `weeks` term while
    // shows and cost both scaled with it, so this column used to fall
    // monotonically for *every* rig at *every* state — touring longer was a
    // strict loss, and the top rig's collapse (+$17,854 at one week to
    // −$50,948 at four) was mostly this bug wearing the rig's name.
    const BUILT_TERRITORY: u8 = 80;
    const MODERATE_TERRITORY: u8 = 40;
    let by_weeks_at = |regional_fame: u8| -> Vec<(u8, TourRig, f32)> {
        let mut rows = Vec::new();
        for weeks in 1..=TOUR_LENGTH_MAX_WEEKS {
            for rig in TourRig::ALL {
                let nets: Vec<i64> = (1..=20u64)
                    .filter_map(|seed| net_at(seed, 80, regional_fame, rig, weeks))
                    .collect();
                if nets.is_empty() {
                    continue;
                }
                let avg = nets.iter().sum::<i64>() as f32 / nets.len() as f32;
                println!(
                    "  {weeks} week(s)  {:<16}: avg net ${avg:>+9.0}  ({} seeds)",
                    rig.label(),
                    nets.len()
                );
                rows.push((weeks, rig, avg));
            }
        }
        rows
    };
    println!("\nsame tours by length (fame 80, regional fame {BUILT_TERRITORY} — built):");
    let by_weeks = by_weeks_at(BUILT_TERRITORY);
    println!("\nsame tours by length (fame 80, regional fame {MODERATE_TERRITORY} — moderate):");
    let by_weeks_moderate = by_weeks_at(MODERATE_TERRITORY);

    let avg_for = |fame: u8, rig: TourRig| {
        avgs.iter()
            .find(|&&(f, r, _)| f == fame && r == rig)
            .map(|&(_, _, a)| a)
    };
    let warm_avg_for = |regional_fame: u8, rig: TourRig| {
        warm.iter()
            .find(|&&(rf, r, _)| rf == regional_fame && r == rig)
            .map(|&(_, _, a)| a)
            .expect("every rig is available at fame 80")
    };
    let weeks_avg_for = |weeks: u8, rig: TourRig| {
        by_weeks
            .iter()
            .find(|&&(w, r, _)| w == weeks && r == rig)
            .map(|&(_, _, a)| a)
            .expect("every rig and length is available at fame 80")
    };
    let moderate_weeks_avg_for = |weeks: u8, rig: TourRig| {
        by_weeks_moderate
            .iter()
            .find(|&&(w, r, _)| w == weeks && r == rig)
            .map(|&(_, _, a)| a)
            .expect("every rig and length is available at fame 80")
    };

    // Target (design §A/§F): the van clears a profit for a small act — and
    // keeps clearing it as the act grows. It's the reliable workhorse; if it
    // ever loses money the low end of the touring economy is broken.
    for fame in fames {
        let van = avg_for(fame, TourRig::Van).expect("the van is available at every fame");
        assert!(
            van > 0.0,
            "van tour should clear a profit at fame {fame}, got avg net ${van:.0}"
        );
    }

    // Target: a full-production rig must not pay off too early. It only makes
    // economic sense once the act is big enough to fill the far bigger rooms
    // its capacity multiplier books — beyond the fame this sweep reaches. At
    // fame 80 (its first available tier here) it should still be firmly in the
    // red; if this flips, the premium rig has become profitable too early.
    if let Some(full) = avg_for(80, TourRig::Full) {
        assert!(
            full < 0.0,
            "full production should not be profitable at fame 80 yet, got avg net ${full:.0}"
        );
    }

    // On a **cold** territory the premium rigs are a capacity/reach
    // investment, not a profit play: the humble van out-nets them wherever
    // both are available. This is the "arriving somewhere new with the big
    // show doesn't pay" half of the design, and it is the only half the
    // pre-T-RIG sweep measured.
    for fame in fames {
        let van = avg_for(fame, TourRig::Van).expect("the van is available at every fame");
        for rig in [TourRig::Truck, TourRig::Full] {
            if let Some(heavy) = avg_for(fame, rig) {
                assert!(
                    van > heavy,
                    "on a cold territory the van (avg net ${van:.0}) should out-net {} \
                     (avg net ${heavy:.0}) at fame {fame}",
                    rig.label()
                );
            }
        }
    }

    // T-RIG, the other half — and the fix's real target. Once the act has
    // *built* the territory, the ladder must invert: each rig up the line
    // out-earns the one below it. Before the pass this was false at every
    // regional fame including 100 (van +$36,853, bus +$43,446, truck +$46,405,
    // full +$32,828): the top rig was **dominated**, never the profit-
    // maximizing choice anywhere on the map — a strictly worse button. Note
    // this is a stronger claim than "full production eventually profits",
    // which it already did from regional fame ~50; a rig nobody should ever
    // pick is still broken even when its net is positive.
    for rig_pair in TourRig::ALL.windows(2) {
        let (lower, higher) = (rig_pair[0], rig_pair[1]);
        let lower_net = warm_avg_for(100, lower);
        let higher_net = warm_avg_for(100, higher);
        assert!(
            higher_net > lower_net,
            "on a fully built territory {} (avg net ${higher_net:.0}) should out-net {} \
             (avg net ${lower_net:.0}) — a rig that is never the best choice is not a choice",
            higher.label(),
            lower.label()
        );
    }

    // ...without flipping the low end: the van must still be the right call
    // for an act that hasn't built the territory yet.
    for rig in [TourRig::Truck, TourRig::Full] {
        let van = warm_avg_for(0, TourRig::Van);
        let heavy = warm_avg_for(0, rig);
        assert!(
            van > heavy,
            "at regional fame 0 the van (avg net ${van:.0}) should still beat {} \
             (avg net ${heavy:.0})",
            rig.label()
        );
    }

    // T-LEN: on a territory the act has built, playing more nights must earn
    // more money — for every rig. The pot's missing `weeks` term made this
    // monotonically *decreasing* everywhere, which is what made length a
    // strictly dominated choice rather than a trade-off.
    for rig in TourRig::ALL {
        for weeks in 2..=TOUR_LENGTH_MAX_WEEKS {
            let shorter = weeks_avg_for(weeks - 1, rig);
            let longer = weeks_avg_for(weeks, rig);
            assert!(
                longer > shorter,
                "{}: on a built territory a {weeks}-week tour (avg net ${longer:.0}) should \
                 out-earn a {}-week one (avg net ${shorter:.0}) — length must be a \
                 trade-off, not a strict loss",
                rig.label(),
                weeks - 1
            );
        }
    }

    // On a territory that is only *moderately* built, the same must hold for
    // every rig the act should actually be booking there — but deliberately
    // not for one it shouldn't. A rig's weekly cost is flat while the pot
    // saturates, so dragging a rig too big for the market around it for a
    // month is supposed to bleed: at regional fame 40 the truck & crew is the
    // right call and full production is not, so full production is the one rig
    // allowed to fall off with length. Pin both halves — the "right rig scales
    // with length" rule *and* the fact that the exception is an over-booked
    // rig, not an arbitrary one.
    let best_moderate_rig = TourRig::ALL
        .iter()
        .copied()
        .max_by(|&a, &b| {
            moderate_weeks_avg_for(TOUR_LENGTH_MAX_WEEKS, a)
                .partial_cmp(&moderate_weeks_avg_for(TOUR_LENGTH_MAX_WEEKS, b))
                .expect("net averages are finite")
        })
        .expect("TourRig::ALL is non-empty");
    assert_ne!(
        best_moderate_rig,
        TourRig::Full,
        "full production should not be the best rig on a moderately built territory"
    );
    for rig in TourRig::ALL {
        if rig == TourRig::Full {
            continue;
        }
        for weeks in 2..=TOUR_LENGTH_MAX_WEEKS {
            let shorter = moderate_weeks_avg_for(weeks - 1, rig);
            let longer = moderate_weeks_avg_for(weeks, rig);
            assert!(
                longer > shorter,
                "{}: at regional fame {MODERATE_TERRITORY} a {weeks}-week tour (avg net \
                 ${longer:.0}) should still out-earn a {}-week one (avg net ${shorter:.0})",
                rig.label(),
                weeks - 1
            );
        }
    }

    // ...but with diminishing returns, or length stops being a decision and
    // the picker collapses to "always four weeks". Marginal net per added week
    // must fall as the tour lengthens.
    for rig in TourRig::ALL {
        let marginal: Vec<f32> = (2..=TOUR_LENGTH_MAX_WEEKS)
            .map(|weeks| weeks_avg_for(weeks, rig) - weeks_avg_for(weeks - 1, rig))
            .collect();
        for pair in marginal.windows(2) {
            assert!(
                pair[1] < pair[0],
                "{}: each added tour week should earn less than the last \
                 (market saturation), got marginals {marginal:?}",
                rig.label()
            );
        }
    }
}

/// §D — lifetime catalog income per release-quality tier, and whether the
/// living tail (fame/marketing-responsive) can produce runaway income for
/// an established star versus a modest release. Releases are hand-built
/// (bypassing the quality RNG) so quality is the only thing that varies;
/// each is aged 60 weeks past its launch window and the catalog tick run
/// directly (`process_music_releases_and_marketing`, §D) every week.
#[test]
#[ignore = "long balance sweep: cargo test -- --ignored --nocapture"]
fn sales_tail_income_by_quality_tier_sweep() {
    fn synthetic_release(initial_sales_score: u32, copies_pressed: u32) -> music::Release {
        music::Release {
            id: 1,
            name: "Sim Release".to_string(),
            release_type: ReleaseType::Album,
            release_quality: 0,
            week_released: 1,
            songs_involved_quality_avg: 0,
            active_marketing: Vec::new(),
            marketing_level_achieved: 0,
            initial_sales_score,
            total_income_generated: 0,
            genre: None,
            copies_pressed,
            copies_sold: 0,
            peak_chart_position: None,
            singles_cut: 0,
            certified: 0,
            distribution_channel: None,
            label_market_reach: None,
        }
    }

    fn lifetime_income_after(fame: u8, initial_sales_score: u32, weeks: u32) -> u32 {
        let mut game = seeded_game(21);
        suppress_story_events(&mut game);
        game.band.fame = fame;
        game.band.peak_fame = fame;
        game.band
            .albums_released
            .push(synthetic_release(initial_sales_score, 50_000));
        // Clear the initial-sales window immediately so every tick below
        // exercises the tail, not the launch-week resolution.
        game.week = 1 + INITIAL_SALES_WINDOW_WEEKS + 1;
        for _ in 0..weeks {
            game.process_turn(GameAction::LazeAround)
                .expect("lazing is always allowed");
            game.take_turn_log();
            game.events.last_event_week = u32::MAX;
        }
        game.band.albums_released[0].total_income_generated
    }

    println!("lifetime catalog income by release-quality tier (60 weeks of tail, fame 20):");
    for (label, score) in [
        ("rough", 20u32),
        ("solid", 60),
        ("great", 100),
        ("transcendent", 160),
    ] {
        let income = lifetime_income_after(20, score, 60);
        println!("  {label:<12} (initial_sales_score {score:>3}): lifetime income ${income}");
    }

    // Runaway-income check: does a famous act's catalog income scale
    // unboundedly with fame, or is it still bounded (by the pressed-copies
    // cap and the divisor's steady growth)? Same release, fame 20 vs fame 95.
    let modest_star_income = lifetime_income_after(20, 80, 104);
    let big_star_income = lifetime_income_after(95, 80, 104);
    println!(
        "same release (initial_sales_score 80) over 2 years: fame 20 -> ${modest_star_income}, fame 95 -> ${big_star_income} \
         (ratio {:.2}x)",
        big_star_income as f32 / modest_star_income.max(1) as f32
    );
}
