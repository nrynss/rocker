use crossterm::{
    cursor,
    event::{self as CEvent, Event, KeyCode}, 
    execute,
    style::{Color, Print, ResetColor, SetForegroundColor},
    terminal::{Clear, ClearType},
};
use std::io::{self as StdIo}; 

use crate::data::{calculate_weeks_to_years_months, constants, format_money};
use crate::game::{Game, GameAction};
use crate::game::music::{MarketingCampaignType, ReleaseType};

pub struct TerminalUI {
    stdout: StdIo::Stdout, 
}

impl TerminalUI {
    pub fn new() -> Result<Self, Box<dyn std::error::Error>> {
        Ok(Self {
            stdout: StdIo::stdout(), 
        })
    }

    pub fn clear_screen(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        execute!(self.stdout, Clear(ClearType::All), cursor::MoveTo(0, 0))?;
        Ok(())
    }

    pub fn show_welcome(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        self.clear_screen()?;
        execute!(
            self.stdout,
            SetForegroundColor(Color::Red),
            Print("╔══════════════════════════════════════════╗\n"),
            Print("║              🎸 ROCKER 🎸               ║\n"),
            Print("║                                          ║\n"),
            Print("║      Rock Star Management Simulator     ║\n"),
            Print("║                                          ║\n"),
            Print("║  Build your band, write songs, tour     ║\n"),
            Print("║  the world, and become a ROCKSTAR!      ║\n"),
            Print("║                                          ║\n"),
            Print("╚══════════════════════════════════════════╝\n"),
            ResetColor,
            Print("\n"),
            Print("Press any key to continue..."),
        )?;
        self.wait_for_key()?;
        Ok(())
    }

    pub fn get_input(&mut self, prompt: &str) -> Result<String, Box<dyn std::error::Error>> {
        execute!(self.stdout, Print(prompt))?;
        let mut input = String::new();
        loop {
            if let Event::Key(key_event) = CEvent::read()? { 
                match key_event.code { 
                    KeyCode::Enter => break,
                    KeyCode::Char(c) => {
                        input.push(c);
                        execute!(self.stdout, Print(c))?;
                    }
                    KeyCode::Backspace => {
                        if !input.is_empty() {
                            input.pop();
                            execute!(
                                self.stdout,
                                cursor::MoveLeft(1),
                                Print(" "),
                                cursor::MoveLeft(1)
                            )?;
                        }
                    }
                    _ => {}
                }
            }
        }
        execute!(self.stdout, Print("\n"))?;
        Ok(input.trim().to_string())
    }

    pub fn display_game_state(&mut self, game: &Game) -> Result<(), Box<dyn std::error::Error>> {
        let current_year = 1970 + (game.week / 52);
        let week_in_year = game.week % 52;
        execute!(
            self.stdout,
            SetForegroundColor(Color::Cyan),
            Print("═══════════════════════════════════════════════════════════════\n"),
            Print(&format!(
                "  Week {} of {} | {} ({})\n", 
                week_in_year + 1,
                current_year,
                game.player.name,
                game.band.name
            )),
            Print(&format!(
                "  Era: {} | {}\n", 
                game.timeline.get_current_era().era_name,
                calculate_weeks_to_years_months(game.week)
            )),
            Print("═══════════════════════════════════════════════════════════════\n"),
            ResetColor,
        )?;
        execute!(
            self.stdout,
            SetForegroundColor(Color::Yellow),
            Print("💰 FINANCES & HEALTH\n"),
            ResetColor,
            Print(&format!("  Money: {}\n", format_money(game.player.money))), 
            Print(&format!(
                "  Health: {}% ({})\n", 
                game.player.health,
                game.player.get_health_status()
            )),
            Print(&format!(
                "  Energy: {}% ({})\n", 
                game.player.energy,
                game.player.get_energy_status()
            )),
            Print(&format!(
                "  Stress: {}% ({})\n", 
                game.player.stress,
                game.player.get_stress_status()
            )),
        )?;
        if game.player.health <= constants::CRITICAL_HEALTH_THRESHOLD {
            execute!(
                self.stdout,
                SetForegroundColor(Color::Red),
                Print("  ⚠️  CRITICAL HEALTH - Visit a doctor immediately!\n"),
                ResetColor,
            )?;
        }
        if game.player.is_addicted() {
            execute!(
                self.stdout,
                SetForegroundColor(Color::Yellow),
                Print("  ⚠️  ADDICTION PROBLEM - Consider rehabilitation!\n"),
                ResetColor,
            )?;
        }
        execute!(self.stdout, Print("\n"))?;
        execute!(
            self.stdout,
            SetForegroundColor(Color::Magenta),
            Print("🎸 BAND STATUS\n"),
            ResetColor,
            Print(&format!(
                "  Fame: {}% ({})\n", 
                game.band.fame,
                game.band.get_fame_level()
            )),
            Print(&format!(
                "  Skill: {}% ({}) | Avg Member: {}%\n", 
                game.band.skill,
                game.band.get_skill_level(),
                game.band.average_member_skill()
            )),
            Print(&format!("  Band Morale: {}%\n", game.band.band_morale())), 
            Print(&format!(
                "  Unreleased Songs: {}\n", 
                game.band.unreleased_songs.len()
            )),
            Print(&format!("  Singles Released: {}\n", game.band.singles_released.len())), 
            Print(&format!("  Albums Released: {}\n", game.band.albums_released.len())), 
        )?;
        if let Some(deal) = game.band.current_deal() {
            execute!(
                self.stdout,
                Print("\n"),
                SetForegroundColor(Color::Yellow),
                Print("✍️ RECORD DEAL\n"),
                ResetColor,
                Print(&format!("  Label: {} ({})\n", deal.label_name, deal.label_tier)), 
                Print(&format!(
                    "  Albums: {} / {}\n", 
                    deal.albums_delivered, deal.albums_required
                )),
                Print(&format!("  Royalty: {:.1}%\n", deal.royalty_rate * 100.0)), 
            )?;
        }
        execute!(
            self.stdout,
            Print("\n"),
            SetForegroundColor(Color::Green),
            Print("👥 BAND MEMBERS\n"),
            ResetColor,
        )?;
        for member in &game.band.members {
            let status = if member.drug_problem { "⚠️" } else { "✅" };
            execute!(
                self.stdout,
                Print(&format!( 
                    "  {} {} - {} (Skill: {}, Loyalty: {}) {}\n",
                    status,
                    member.name,
                    member.instrument,
                    member.skill,
                    member.loyalty,
                    if member.drug_problem {
                        "[Drug Problem]"
                    } else {
                        ""
                    }
                ))
            )?;
        }
        execute!(
            self.stdout,
            Print("\n"),
            SetForegroundColor(Color::Blue),
            Print("🎵 MUSIC SCENE\n"),
            ResetColor,
            Print(&format!(
                "  Trending Genres: {}\n", 
                game.timeline.get_trending_genres().join(", ")
            )),
            Print(&format!(
                "  Market Demand: {}%\n", 
                game.timeline
                    .get_current_era()
                    .market_conditions
                    .overall_demand
            )),
            Print(&format!(
                "  Innovation Climate: {}%\n", 
                game.timeline.get_innovation_bonus()
            )),
        )?;
        if let Some(historical_event) = game.get_last_historical_event() {
            execute!(
                self.stdout,
                SetForegroundColor(Color::Magenta),
                Print("📰 MUSIC NEWS: "),
                ResetColor,
                Print(&format!("{}
", historical_event)), 
            )?;
        }
        execute!(self.stdout, Print("\n"), SetForegroundColor(Color::DarkCyan), Print("💿 RELEASES & MARKETING\n"), ResetColor)?;
        let mut all_releases_display = Vec::new(); 
        for r_ref in game.just_released_music.iter().chain(game.band.singles_released.iter()).chain(game.band.albums_released.iter()) {
            all_releases_display.push(r_ref);
        }
        all_releases_display.sort_by(|a, b| b.week_released.cmp(&a.week_released)); 
        all_releases_display.dedup_by_key(|r| r.id);
        for (i, release_item) in all_releases_display.iter().take(5).enumerate() { 
            let type_str = if release_item.release_type == ReleaseType::Album { "Album" } else { "Single" };
            let marketing_info = if !release_item.active_marketing.is_empty() {
                format!("(Marketing: {:?} - {} weeks left)", 
                    release_item.active_marketing[0].campaign_type,
                    release_item.active_marketing[0].end_week.saturating_sub(game.week)
                )
            } else { "".to_string() };
            execute!(self.stdout, Print(&format!( 
                "  {}. {} ({}) - Q:{}/100, Sales:{}, Income:{} {}\n",
                i + 1, release_item.name, type_str, release_item.release_quality,
                release_item.initial_sales_score, format_money(release_item.total_income_generated as i32),
                marketing_info
            )))?;
        }
        if all_releases_display.is_empty() {
            execute!(self.stdout, Print("  No releases yet.\n"))?;
        }
        execute!(self.stdout, Print("\n"))?;
        Ok(())
    }

    pub fn show_main_menu(&mut self, game: &Game) -> Result<GameAction, Box<dyn std::error::Error>> {
        let single_cost = ((constants::SINGLE_RECORDING_COST as f32)
            * game.timeline.get_recording_cost_modifier()) as i32;
        let album_cost = ((constants::ALBUM_RECORDING_BASE_COST as f32)
            * game.timeline.get_recording_cost_modifier()) as i32;
        let single_available = game.band.can_record_single() && game.player.can_afford(single_cost);
        let album_available = game.band.can_record_album() && game.player.can_afford(album_cost);
        let doctor_available = game.player.can_afford(constants::DOCTOR_VISIT_COST);
        execute!(
            self.stdout,
            SetForegroundColor(Color::White),
            Print("🎵 WHAT DO YOU WANT TO DO THIS WEEK?\n"),
            ResetColor,
            Print("\n"),
            Print("1. Laze around\n"),
            Print("2. Write songs\n"),
            Print("3. Practice with the band\n"),
            Print(&format!("4. Record a single ({}) {}\n", format_money(single_cost), if single_available { "✅" } else { "❌" })), 
            Print(&format!("5. Record an album ({}) {}\n", format_money(album_cost), if album_available { "✅" } else { "❌" })), 
            Print("6. Play a gig\n"),
            Print("7. Go on tour\n"),
            Print("8. Take a break\n"),
            Print(&format!("9. Visit the doctor ({}) {}\n", format_money(constants::DOCTOR_VISIT_COST), if doctor_available { "✅" } else { "❌" })), 
            Print("K. Marketing Actions\n"),
            Print("S. Save Game\n"),
            Print("L. Load Game\n"),
        )?;
        if !game.pending_deal_offers.is_empty() {
            execute!(
                self.stdout,
                SetForegroundColor(Color::Green),
                Print("V. View Record Deal Offers 🔥\n"),
                ResetColor
            )?;
        }
        execute!(self.stdout, Print("Q. Quit game\n\nEnter your choice: "))?;
        loop {
            if let Event::Key(key_event) = CEvent::read()? { 
                let mut action = match key_event.code { 
                    KeyCode::Char('1') => Some(GameAction::LazeAround),
                    KeyCode::Char('2') => Some(GameAction::WriteSongs),
                    KeyCode::Char('3') => Some(GameAction::Practice),
                    KeyCode::Char('4') => Some(GameAction::RecordSingle),
                    KeyCode::Char('5') => Some(GameAction::RecordAlbum),
                    KeyCode::Char('6') => Some(GameAction::Gig),
                    KeyCode::Char('7') => Some(GameAction::GoOnTour),
                    KeyCode::Char('8') => Some(GameAction::TakeBreak),
                    KeyCode::Char('9') => Some(GameAction::VisitDoctor),
                    KeyCode::Char('k') | KeyCode::Char('K') => Some(GameAction::ShowMarketingMenu),
                    KeyCode::Char('s') | KeyCode::Char('S') => Some(GameAction::SaveGame),
                    KeyCode::Char('l') | KeyCode::Char('L') => Some(GameAction::LoadGame),
                    KeyCode::Char('q') | KeyCode::Char('Q') => Some(GameAction::Quit),
                    _ => None,
                };
                if action.is_none() {
                    if !game.pending_deal_offers.is_empty() {
                        if let KeyCode::Char('v') | KeyCode::Char('V') = key_event.code { 
                            action = Some(GameAction::ViewDealOffers);
                        }
                    }
                }
                if let Some(act) = action { 
                    execute!(self.stdout, Print("\n\n"))?;
                    return Ok(act);
                }
            }
        }
    }

    pub fn show_error(&mut self, message: &str) -> Result<(), Box<dyn std::error::Error>> {
        execute!(
            self.stdout,
            SetForegroundColor(Color::Red),
            Print("❌ ERROR: "), Print(message), Print("\n\n"), ResetColor,
            Print("Press any key to continue..."),
        )?;
        self.wait_for_key()?;
        Ok(())
    }

    pub fn show_message(&mut self, message: &str) -> Result<(), Box<dyn std::error::Error>> {
        execute!(
            self.stdout,
            SetForegroundColor(Color::Green),
            Print("✅ INFO: "), Print(message), Print("\n\n"), ResetColor,
            Print("Press any key to continue..."),
        )?;
        self.wait_for_key()?;
        Ok(())
    }

    pub fn show_game_over(&mut self, game: &Game) -> Result<(), Box<dyn std::error::Error>> {
        self.clear_screen()?;
        execute!(
            self.stdout,
            SetForegroundColor(Color::Red),
            Print("╔══════════════════════════════════════════╗\n"),
            Print("║                GAME OVER                 ║\n"),
            Print("╚══════════════════════════════════════════╝\n"),
            ResetColor, Print("\n"),
            Print(&format!("Final Status: {}\n", game.get_status_message())), 
            Print(&format!("Career Length: {}\n", calculate_weeks_to_years_months(game.week))), 
            Print(&format!("Final Fame Level: {}% ({})\n", game.band.fame, game.band.get_fame_level())), 
            Print(&format!("Money: {}\n", format_money(game.player.money))), 
            Print(&format!("Singles Released: {}\n", game.band.singles_released.len())), 
            Print(&format!("Albums Released: {}\n", game.band.albums_released.len())), 
            Print("\nThanks for playing ROCKER!\nPress any key to exit..."),
        )?;
        self.wait_for_key()?;
        Ok(())
    }

    fn wait_for_key(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        loop { if let Event::Key(_) = CEvent::read()? { break; } } 
        Ok(())
    }

    fn display_single_offer_details(&mut self, offer: &crate::game::world::PotentialDealOffer) -> Result<(), Box<dyn std::error::Error>> {
        execute!(
            self.stdout, SetForegroundColor(Color::Yellow), Print("--- Offer Details ---\n"), ResetColor,
            Print(&format!("Label: {} ({})\n", offer.label_name, offer.label_tier)), 
            Print(&format!("Advance: {}\n", format_money(offer.advance as i32))),  
            Print(&format!("Royalty Rate: {:.1}%\n", offer.royalty_rate * 100.0)), 
            Print(&format!("Albums Required: {}\n", offer.albums_required)), 
            Print("\n"), SetForegroundColor(Color::Cyan), Print("Label Info (from original data):\n"), ResetColor,
            Print(&format!("  Market Reach: {}/100\n", offer.original_label_data.market_reach)), 
            Print(&format!("  Financial Power: {}/100\n", offer.original_label_data.financial_power)), 
            Print(&format!("  Artist Development: {}/100\n", offer.original_label_data.artist_development)), 
            Print(&format!("  Creative Freedom: {}/100\n", offer.original_label_data.creative_freedom)), 
            Print(&format!("  Reputation: {}\n", offer.original_label_data.reputation)), 
            Print("\n"),
        )?;
        Ok(())
    }

    pub fn show_deal_offers_menu(&mut self, game: &Game) -> Result<Option<GameAction>, Box<dyn std::error::Error>> {
        loop {
            self.clear_screen()?;
            execute!(self.stdout, SetForegroundColor(Color::Green), Print("--- Record Deal Offers ---\n\n"), ResetColor)?;
            if game.pending_deal_offers.is_empty() {
                execute!(self.stdout, Print("No current offers.\n\nPress any key to return..."))?;
                self.wait_for_key()?;
                return Ok(None);
            }
            for (index, offer) in game.pending_deal_offers.iter().enumerate() {
                execute!(self.stdout, Print(&format!( 
                    "[{}| {} ({}): Advance {}, Royalty {:.1}%, {} Albums\n", 
                    index, offer.label_name, offer.label_tier,
                    format_money(offer.advance as i32),
                    offer.royalty_rate * 100.0, offer.albums_required
                )))?;
            }
            execute!(self.stdout, Print("\nEnter offer number to inspect (or 'b' to go back): "))?;
            let choice = self.get_input("")?;
            if choice.eq_ignore_ascii_case("b") { return Ok(None); }
            match choice.parse::<usize>() {
                Ok(selected_index) if selected_index < game.pending_deal_offers.len() => {
                    let selected_offer = &game.pending_deal_offers[selected_index];
                    loop {
                        self.clear_screen()?;
                        self.display_single_offer_details(selected_offer)?;
                        execute!(self.stdout, Print("[A]ccept Deal, [R]eject Deal, or [B]ack to offers list: "))?;
                        let decision_input = self.get_input("")?.to_lowercase();
                        match decision_input.as_str() {
                            "a" => return Ok(Some(GameAction::AcceptDeal(selected_index))),
                            "r" => return Ok(Some(GameAction::RejectDeal(selected_index))),
                            "b" => break,
                            _ => { self.show_error("Invalid choice. Please enter A, R, or B.")?; }
                        }
                    }
                }
                _ => { self.show_error("Invalid offer number. Please try again.")?; }
            }
        }
    }

    pub fn show_marketing_menu(&mut self, game: &Game) -> Result<Option<GameAction>, Box<dyn std::error::Error>> {
        loop {
            self.clear_screen()?;
            execute!(self.stdout, SetForegroundColor(Color::Yellow), Print("--- Marketing Actions ---\n"), ResetColor)?;
            execute!(self.stdout, Print("1. Launch New Campaign\n"))?;
            execute!(self.stdout, Print("2. View Active Campaigns\n"))?;
            execute!(self.stdout, Print("B. Back to Main Menu\n\nEnter your choice: "))?;
            let choice = self.get_input("")?.to_lowercase();
            match choice.as_str() {
                "1" => {
                    if let Some(release_id) = self.select_release_for_marketing(game)? {
                        if let Some(campaign_type) = self.select_marketing_campaign_type(game, release_id)? { 
                            return Ok(Some(GameAction::StartMarketingCampaign(release_id, campaign_type)));
                        }
                    }
                }
                "2" => {
                    self.display_active_campaigns(game)?;
                }
                "b" => return Ok(None),
                _ => self.show_error("Invalid choice. Please try again.")?,
            }
        }
    }

    fn select_release_for_marketing(&mut self, game: &Game) -> Result<Option<u32>, Box<dyn std::error::Error>> {
        loop {
            self.clear_screen()?;
            execute!(self.stdout, SetForegroundColor(Color::Cyan), Print("--- Select Release to Market ---\n"), ResetColor)?;
            let mut available_releases = Vec::new();
            let mut displayed_ids = std::collections::HashSet::new();
            for r_ref in game.just_released_music.iter().chain(game.band.singles_released.iter()).chain(game.band.albums_released.iter()) {
                if displayed_ids.insert(r_ref.id) {
                    available_releases.push(r_ref);
                }
            }
            available_releases.sort_by(|a,b| b.week_released.cmp(&a.week_released));
            if available_releases.is_empty() {
                execute!(self.stdout, Print("No releases available to market.\nPress any key to return..."))?;
                self.wait_for_key()?;
                return Ok(None);
            }
            for (idx, release_item) in available_releases.iter().enumerate() { 
                let type_str = if release_item.release_type == ReleaseType::Album { "Album" } else { "Single" };
                execute!(self.stdout, Print(&format!("[{}] {} ({}) - Q:{}/100\n", idx, release_item.name, type_str, release_item.release_quality)))?; 
            }
            execute!(self.stdout, Print("\nEnter number of release (or 'b' to go back): "))?;
            let choice = self.get_input("")?;
            if choice.eq_ignore_ascii_case("b") { return Ok(None); }
            match choice.parse::<usize>() {
                Ok(idx) if idx < available_releases.len() => return Ok(Some(available_releases[idx].id)),
                _ => self.show_error("Invalid selection.")?,
            }
        }
    }

    fn select_marketing_campaign_type(&mut self, _game: &Game, _release_id: u32) -> Result<Option<MarketingCampaignType>, Box<dyn std::error::Error>> {
        loop {
            self.clear_screen()?;
            execute!(self.stdout, SetForegroundColor(Color::Cyan), Print("--- Select Marketing Campaign Type ---\n"), ResetColor)?;
            let campaigns = [
                (MarketingCampaignType::BasicPress, "Basic Press", 100),
                (MarketingCampaignType::RadioPromotion, "Radio Promotion", 500),
                (MarketingCampaignType::MusicVideo, "Music Video", 2000),
                (MarketingCampaignType::SocialMediaBlitz, "Social Media Blitz", 750),
                (MarketingCampaignType::MagazineSpread, "Magazine Spread", 300),
            ];
            for (idx, (_, name, cost)) in campaigns.iter().enumerate() {
                execute!(self.stdout, Print(&format!("[{}] {} (${})\n", idx, name, cost)))?; 
            }
            execute!(self.stdout, Print("\nEnter campaign number (or 'b' to go back): "))?;
            let choice = self.get_input("")?;
            if choice.eq_ignore_ascii_case("b") { return Ok(None); }
            match choice.parse::<usize>() {
                Ok(idx) if idx < campaigns.len() => return Ok(Some(campaigns[idx].0.clone())),
                _ => self.show_error("Invalid selection.")?,
            }
        }
    }

    fn display_active_campaigns(&mut self, game: &Game) -> Result<(), Box<dyn std::error::Error>> {
        self.clear_screen()?;
        execute!(self.stdout, SetForegroundColor(Color::Yellow), Print("--- Active Marketing Campaigns ---\n"), ResetColor)?;
        let mut found_any = false;
        let all_releases_items = game.just_released_music.iter() 
            .chain(game.band.singles_released.iter())
            .chain(game.band.albums_released.iter());
        for release_item in all_releases_items { 
            if !release_item.active_marketing.is_empty() {
                found_any = true;
                execute!(self.stdout, Print(&format!("\nRelease: {} (Q:{}/100)\n", release_item.name, release_item.release_quality)))?; 
                for campaign in &release_item.active_marketing {
                    execute!(self.stdout, Print(&format!("  - {:?}: ends week {}, bonus +{}\n", 
                        campaign.campaign_type, campaign.end_week, campaign.effectiveness_bonus
                    )))?;
                }
            }
        }
        if !found_any {
            execute!(self.stdout, Print("No active marketing campaigns.\n"))?;
        }
        execute!(self.stdout, Print("\nPress any key to return..."))?;
        self.wait_for_key()?;
        Ok(())
    }

    pub fn display_release_report(&mut self, release_name: &str, quality: u8, marketing_level: u8, sales_score: u32, income: i32) -> Result<(), Box<dyn std::error::Error>> {
        self.clear_screen()?;
        execute!(self.stdout, SetForegroundColor(Color::Cyan), Print("--- Release Report ---\n"), ResetColor)?;
        execute!(self.stdout, Print(&format!("Release: '{}'\n", release_name)))?;
        execute!(self.stdout, Print(&format!("  Quality:           {}/100\n", quality)))?;
        execute!(self.stdout, Print(&format!("  Marketing Level:   {}/100\n", marketing_level)))?;
        execute!(self.stdout, Print(&format!("  Initial Sales Score: {}\n", sales_score)))?;
        execute!(self.stdout, Print(&format!("  Income This Period:  {}\n", format_money(income))))?; 
        execute!(self.stdout, Print("\nPress any key to continue..."))?;
        self.wait_for_key()?;
        Ok(())
    }
}
