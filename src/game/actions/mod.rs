//! The player's weekly actions: what each choice costs, what it does,
//! and the `execute_action` dispatch that routes a `GameAction` here.

mod business;
mod live;
mod rest;
mod studio;

use rand::Rng;

use super::*;

/// How a tour left the calendar (§F) — `turn.rs` names it when an emergency
/// forces the walk-out.
pub(in crate::game) use live::TourEnding;
pub use live::{TourQuote, TourRig};

impl Game {
    pub(super) fn execute_action(
        &mut self,
        action: GameAction,
        rng: &mut impl Rng,
    ) -> Result<(), String> {
        match action {
            GameAction::LazeAround => self.action_laze_around(),
            GameAction::WriteSongs => self.action_write_songs(rng),
            GameAction::Practice => self.action_practice(),
            // S-MASTER (§D): recording books a session and nothing else, and
            // its rolls come off the activity stream at booking — so neither
            // takes the turn's `rng`, exactly as `action_go_on_tour` stopped
            // taking one when S-TIME made a tour a commitment.
            GameAction::RecordSingle { songs, studio } => self.action_record_single(songs, studio),
            GameAction::RecordAlbum { songs, studio } => self.action_record_album(songs, studio),
            GameAction::ReleaseMaster { master, pressing } => {
                self.action_release_master(master, pressing)
            }
            GameAction::Gig(venue_index) => self.action_play_gig(venue_index, rng),
            GameAction::GoOnTour(region_index, rig, weeks) => {
                self.action_go_on_tour(region_index, rig, weeks)
            }
            GameAction::TakeBreak => self.action_take_break(),
            GameAction::VisitDoctor => self.action_visit_doctor(),
            GameAction::AcceptDeal(index) => self.action_accept_deal(index),
            GameAction::RejectDeal(index) => self.action_reject_deal(index, rng),
            GameAction::AcceptSupportTour => self.action_accept_support_tour(),
            GameAction::DeclineSupportTour => self.action_decline_support_tour(),
            GameAction::StartMarketingCampaign(release_id, campaign_type) => {
                self.action_start_marketing_campaign(release_id, campaign_type)
            }
            GameAction::ChangeLifestyle(tier) => self.action_change_lifestyle(tier),
            GameAction::RePress {
                release_id,
                pressing,
            } => self.action_re_press(release_id, pressing),
            GameAction::AbandonActivity => self.action_abandon_activity(),
            GameAction::Quit => {
                self.game_over = true;
                Ok(())
            }
        }
    }
}
