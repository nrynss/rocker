//! Event outcomes: applies random incidents and historical events to the game
//! state. Cadence + weighted selection live in `events.rs`; the incident pool
//! is data (`data/incidents.json`); week orchestration stays in `turn.rs`.

use rand::Rng;

use crate::data_loader::Incident;

use super::constants;
use super::*;

impl Game {
    /// Roll the weekly incident gate; if it fires, pick a weighted incident
    /// from those eligible for the current fame/signed state and apply it.
    /// Gate, selection, and effect rolls all draw on the action stream
    /// (design §F/§G). The single entry point `turn.rs` calls.
    pub(super) fn maybe_trigger_incident(&mut self, rng: &mut impl Rng) -> Result<(), String> {
        if !self.events.try_trigger_event(self.week, rng) {
            return Ok(());
        }
        let fame = self.band.fame;
        let signed = self.band.has_record_deal();
        // Clone the chosen incident out of `data_files` so applying its
        // effects (which mutates `self`) doesn't hold a borrow of it.
        let chosen = {
            let eligible = self
                .data_files
                .incidents_data
                .eligible_incidents(fame, signed);
            events::weighted_pick(&eligible, rng).cloned()
        };
        if let Some(incident) = chosen {
            self.apply_incident(&incident, rng);
        }
        Ok(())
    }

    /// Apply one incident's effect ranges to the four bars, money, and fame.
    /// Ranges roll inclusively on the action stream in a fixed field order;
    /// bars clamp to 0–100, money may go negative, and fame *gains* route
    /// through the comeback-aware `gain_fame` while *losses* saturate — never
    /// through `gain_fame`. Then the incident's message hits the log.
    /// (design §A/§F)
    pub(super) fn apply_incident(&mut self, incident: &Incident, rng: &mut impl Rng) {
        let e = &incident.effects;
        apply_bar(
            &mut self.player.stress,
            roll_range(e.stress, rng),
            constants::MAX_STRESS,
        );
        apply_bar(
            &mut self.player.happiness,
            roll_range(e.happiness, rng),
            constants::MAX_HAPPINESS,
        );
        apply_bar(
            &mut self.player.creativity,
            roll_range(e.creativity, rng),
            constants::MAX_CREATIVITY,
        );
        apply_bar(
            &mut self.player.health,
            roll_range(e.health, rng),
            constants::MAX_HEALTH,
        );

        self.player.money += roll_range(e.money, rng);

        let fame_delta = roll_range(e.fame, rng);
        if fame_delta > 0 {
            self.band.gain_fame(fame_delta as u8);
        } else if fame_delta < 0 {
            self.band.fame = self.band.fame.saturating_sub((-fame_delta) as u8);
        }

        self.log(incident.message.clone());
    }

    pub(super) fn apply_historical_event(
        &mut self,
        event: &str,
        rng: &mut impl Rng,
    ) -> Result<(), String> {
        match event {
            event if event.contains("Beatles") => {
                if self.band.dominant_genres_match(&["Rock", "Folk Rock"]) {
                    self.band.gain_fame(5);
                    self.player.money += 200;
                }
            }
            event if event.contains("MTV") => {
                if self.timeline.get_image_importance() > 80 {
                    if self.band.reputation.media_presence > 60 {
                        self.band.gain_fame(10);
                        let earnings = rng.gen_range(1000..3000);
                        self.player.money += earnings;
                    } else {
                        self.band.fame = self.band.fame.saturating_sub(5);
                    }
                }
            }
            event if event.contains("Grunge emerges") => {
                if self.band.dominant_genres_match(&["Grunge", "Alternative"]) {
                    self.band.gain_fame(12);
                    let major_earnings = rng.gen_range(2000..5000);
                    self.player.money += major_earnings;
                } else if self
                    .band
                    .dominant_genres_match(&["Hair Metal", "Pop Metal"])
                {
                    self.band.fame = self.band.fame.saturating_sub(8);
                }
            }
            _ => match rng.gen_range(0..3) {
                0 => {
                    self.band.gain_fame(1);
                }
                1 => self.player.money += rng.gen_range(50..200),
                _ => {
                    self.band.reputation.critical_acclaim =
                        (self.band.reputation.critical_acclaim + 1).min(100)
                }
            },
        }

        Ok(())
    }
}

/// Roll an inclusive `[lo, hi]` effect range, or 0 for an omitted effect
/// (omitted effects consume no rng, keeping replays stable).
fn roll_range(range: Option<[i32; 2]>, rng: &mut impl Rng) -> i32 {
    match range {
        Some([lo, hi]) => rng.gen_range(lo..=hi),
        None => 0,
    }
}

/// Nudge a 0–`max` bar by a signed delta, clamping at both ends.
fn apply_bar(bar: &mut u8, delta: i32, max: u8) {
    let updated = i32::from(*bar) + delta;
    *bar = updated.clamp(0, i32::from(max)) as u8;
}
