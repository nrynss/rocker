//! Track B's contract: a run is fully determined by its seed and choices.

use super::*;

/// Two games on the same seed, fed the same twenty turns, must land on the
/// same week with the same money and fame and an identical week-by-week
/// story; a different seed must tell a different one.
#[test]
fn same_seed_and_same_choices_replay_the_same_career() {
    fn scripted_run(seed: u64) -> (u32, i32, u8, String) {
        let mut game = crate::game::sim::seeded_game(seed);
        // A representative career slice: writing, gigging, idling, one
        // club-run single, and a multi-week break (so the per-week RNG
        // keying survives calendar jumps).
        let script = [
            GameAction::WriteSongs,
            GameAction::Gig(0),
            GameAction::LazeAround,
            GameAction::WriteSongs,
            GameAction::LazeAround,
            GameAction::RecordSingle { pressing: Some(1) },
            GameAction::Gig(0),
            GameAction::LazeAround,
            GameAction::Gig(0),
            GameAction::TakeBreak,
            GameAction::WriteSongs,
            GameAction::WriteSongs,
            GameAction::Gig(0),
            GameAction::LazeAround,
            GameAction::WriteSongs,
            GameAction::LazeAround,
            GameAction::Gig(0),
            GameAction::LazeAround,
            GameAction::Gig(0),
            GameAction::LazeAround,
        ];
        let mut log: Vec<String> = Vec::new();
        for action in script {
            // A rejection is part of the story too — it must replay.
            if let Err(rejection) = game.process_turn(action) {
                log.push(format!("[rejected] {rejection}"));
            }
            log.append(&mut game.take_turn_log());
        }
        (game.week, game.player.money, game.band.fame, log.join("\n"))
    }

    let (week_a, money_a, fame_a, story_a) = scripted_run(2025);
    let (week_b, money_b, fame_b, story_b) = scripted_run(2025);
    assert_eq!(week_a, week_b, "same seed, same calendar");
    assert_eq!(money_a, money_b, "same seed, same bank balance");
    assert_eq!(fame_a, fame_b, "same seed, same fame");
    assert_eq!(story_a, story_b, "same seed, same story, line for line");

    // The script must have exercised the seeded rolls for the proof to
    // mean anything: songs written and a single actually recorded.
    assert!(
        story_a.contains("🎼 Wrote"),
        "the script should write songs:\n{story_a}"
    );
    assert!(
        story_a.contains("🎙️ Recorded"),
        "the script should record a single:\n{story_a}"
    );

    let (_, _, _, story_c) = scripted_run(2026);
    assert_ne!(
        story_a, story_c,
        "a different seed must tell a different story"
    );
}
