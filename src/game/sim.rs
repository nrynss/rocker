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
use crate::game::music::ReleaseType;
use crate::game::player::Player;
use crate::game::timeline::MusicTimeline;
use crate::game::world::GameWorld;
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
}

impl Bot {
    const ALL: [Bot; 4] = [
        Bot::GigGrinder,
        Bot::StudioRat,
        Bot::BalancedIndie,
        Bot::LabelLoyalist,
    ];

    fn name(self) -> &'static str {
        match self {
            Bot::GigGrinder => "gig-grinder",
            Bot::StudioRat => "studio-rat",
            Bot::BalancedIndie => "balanced-indie",
            Bot::LabelLoyalist => "label-loyalist",
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
            Bot::BalancedIndie => indie_loop(game, Some(CLUB_RUN)),
            Bot::LabelLoyalist => label_loyalist(game),
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
fn indie_loop(game: &Game, pressing: Option<usize>) -> GameAction {
    // A support slot is exposure money can't buy — and it pays.
    if game.pending_support_offer.is_some()
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
    indie_loop(game, pressing)
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
    deals_signed: u32,
    sell_outs: u32,
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
        deals_signed: 0,
        sell_outs: 0,
        fame_by_year: Vec::new(),
    };
    let mut had_deal = false;
    let mut stalled_turns: u32 = 0;

    while !game.is_game_over() && game.week <= horizon_weeks {
        let week_before = game.week;
        let action = bot.decide(game);
        if game.process_turn(action).is_err() {
            game.process_turn(GameAction::LazeAround)
                .expect("lazing is always allowed");
        }

        for line in game.take_turn_log() {
            if line.contains("sold out") {
                career.sell_outs += 1;
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
        }
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
    const FIXED_FAME: u8 = 40; // "regional" tier: same tour_weeks/cost for every run below.

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

        game.process_turn(GameAction::GoOnTour(region_index))
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
