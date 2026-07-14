//! The player's weekly actions: what each choice costs, what it does,
//! and the `execute_action` dispatch that routes a `GameAction` here.

mod business;
mod live;
mod rest;
mod studio;

use rand::Rng;

use super::*;
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
            GameAction::RecordSingle { pressing } => self.action_record_single(pressing, rng),
            GameAction::RecordAlbum { pressing } => self.action_record_album(pressing, rng),
            GameAction::Gig(venue_index) => self.action_play_gig(venue_index, rng),
            GameAction::GoOnTour(region_index) => self.action_go_on_tour(region_index, rng),
            GameAction::TakeBreak => self.action_take_break(),
            GameAction::VisitDoctor => self.action_visit_doctor(),
            GameAction::AcceptDeal(index) => self.action_accept_deal(index),
            GameAction::RejectDeal(index) => self.action_reject_deal(index, rng),
            GameAction::AcceptSupportTour => self.action_accept_support_tour(rng),
            GameAction::DeclineSupportTour => self.action_decline_support_tour(),
            GameAction::StartMarketingCampaign(release_id, campaign_type) => {
                self.action_start_marketing_campaign(release_id, campaign_type)
            }
            GameAction::Quit => {
                self.game_over = true;
                Ok(())
            }
        }
    }
}
