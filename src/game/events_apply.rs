//! Event outcomes: applies random incidents and historical events to the game
//! state. Cadence + weighted selection live in `events.rs`; the incident pool
//! is data (`data/incidents.json`); week orchestration stays in `turn.rs`.

use rand::Rng;

use crate::data_loader::{ActivityScope, Incident};

use super::constants;
use super::*;

/// The event scope a commitment in progress puts the band in (design §G).
///
/// The catch-all is deliberate and load-bearing: `OngoingActivity::Recording`
/// is S-MASTER's to land (§D), and the studio pool in `data/incidents.json`
/// is already written and already gated `"recording"`. Mapping anything this
/// task cannot yet name to `Recording` means the studio's incidents start
/// firing the week the variant appears, with no edit here — which is the
/// whole point of gating on data rather than on code.
fn scope_of_activity(activity: &OngoingActivity) -> ActivityScope {
    match activity {
        OngoingActivity::Tour(_) => ActivityScope::Tour,
        OngoingActivity::SupportTour(_) => ActivityScope::SupportTour,
        OngoingActivity::Break(_) => ActivityScope::Break,
        #[allow(unreachable_patterns)]
        _ => ActivityScope::Recording,
    }
}

impl Game {
    /// §G tap point — the one line `turn.rs`'s `resolve_activity_week` adds,
    /// at the top, *before* the activity is taken and possibly settled.
    /// Without it the week that ends a tour looks, to the event taps, like a
    /// week at home; with it every week of a commitment is in scope.
    pub(in crate::game) fn note_activity_week(&mut self) {
        let scope = self.ongoing_activity.as_ref().map(scope_of_activity);
        if let Some(scope) = scope {
            self.events.pending_activity = Some(scope);
        }
    }

    /// §G tap point — the one line `actions/live.rs`'s `action_play_gig`
    /// adds, once its guards have passed. A one-off gig is a night on a
    /// stage, not an `OngoingActivity`, so nothing else can tell the tap it
    /// happened.
    pub(in crate::game) fn note_gig_week(&mut self) {
        self.events.pending_activity = Some(ActivityScope::Gig);
    }

    /// Roll the week's incident taps (design §G). The activity tap goes
    /// first: if the band is in the middle of something that has its own
    /// hazards, that is the more specific story, and it takes the week's one
    /// incident. Otherwise the ordinary weekly tap runs exactly as it always
    /// has. Gate, selection, and effect rolls all draw on the action stream
    /// (design §F/§G) — never the activity stream, whose steps belong to the
    /// tour's nights and must keep rolling what they rolled before §G. The
    /// single entry point `turn.rs` calls.
    pub(super) fn maybe_trigger_incident(&mut self, rng: &mut impl Rng) -> Result<(), String> {
        if self.maybe_trigger_activity_incident(rng) {
            return Ok(());
        }
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

    /// The §G activity tap. Returns whether it fired — a hit takes the week's
    /// one incident and the general tap is skipped.
    ///
    /// Two orderings here are deliberate. The latch is consumed
    /// **unconditionally**, so a week's scope can never leak into the next
    /// one. And the pool is resolved **before** the gate is rolled: an
    /// activity with nothing eligible (a break, or a fame tier no scoped
    /// incident admits) consumes no rng and cannot burn the week's incident
    /// on an empty pick — which is what lets the general tap keep firing on
    /// the road.
    fn maybe_trigger_activity_incident(&mut self, rng: &mut impl Rng) -> bool {
        let latched = self.events.pending_activity.take();
        let Some(scope) = latched.or_else(|| self.ongoing_activity.as_ref().map(scope_of_activity))
        else {
            return false;
        };

        let fame = self.band.fame;
        let signed = self.band.has_record_deal();
        if self
            .data_files
            .incidents_data
            .eligible_activity_incidents(fame, signed, scope)
            .is_empty()
        {
            return false;
        }
        if !self.events.try_trigger_activity_event(self.week, rng) {
            return false;
        }

        // Cloned out of `data_files` for the same borrow reason as above.
        let chosen = {
            let eligible = self
                .data_files
                .incidents_data
                .eligible_activity_incidents(fame, signed, scope);
            events::weighted_pick(&eligible, rng).cloned()
        };
        if let Some(incident) = chosen {
            self.apply_incident(&incident, rng);
        }
        true
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
                    self.player.money += 600; // L-SCALE §B: ×3
                }
            }
            event if event.contains("MTV") => {
                if self.timeline.get_image_importance() > 80 {
                    if self.band.reputation.media_presence > 60 {
                        self.band.gain_fame(10);
                        let earnings = rng.gen_range(3000..9000); // L-SCALE §B: ×3
                        self.player.money += earnings;
                    } else {
                        self.band.fame = self.band.fame.saturating_sub(5);
                    }
                }
            }
            event if event.contains("Grunge emerges") => {
                if self.band.dominant_genres_match(&["Grunge", "Alternative"]) {
                    self.band.gain_fame(12);
                    let major_earnings = rng.gen_range(6000..15000); // L-SCALE §B: ×3
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
                1 => self.player.money += rng.gen_range(150..600), // L-SCALE §B: ×3
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
