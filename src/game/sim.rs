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
use crate::game::music::ReleaseType;
use crate::game::player::Player;
use crate::game::timeline::MusicTimeline;
use crate::game::world::GameWorld;
use rand::SeedableRng;
use rand::rngs::StdRng;

use super::constants::{self, *};
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

/// Energy the bots require before attempting the matching action.
const GIG_ENERGY: u8 = 30;
const WRITE_ENERGY: u8 = 20;
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
    }
    if game.player.energy >= WRITE_ENERGY {
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
    if game.pending_support_offer.is_some() && game.player.energy >= GIG_ENERGY {
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
        return if game.player.energy >= WRITE_ENERGY {
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
        let win_year = median(
            runs.iter()
                .filter(|c| c.won())
                .map(|c| c.weeks / constants::WEEKS_PER_YEAR)
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
            win_weeks.push(career.weeks);
        }
    }
    println!(
        "balanced-indie won {wins}/{WIN_TARGET_SEEDS} careers by year 12 (median win week: {})",
        show(median(win_weeks))
    );
    assert!(
        wins * 2 > WIN_TARGET_SEEDS,
        "balanced-indie won only {wins}/{WIN_TARGET_SEEDS} careers by year 12"
    );
}
