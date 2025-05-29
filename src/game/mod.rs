pub mod band;
pub mod events;
pub mod music;
pub mod player;
pub mod timeline;
pub mod world;

use crate::data::constants;
use crate::data_loader::GameDataFiles;
use crate::game::music::*; // For Song, Release, ReleaseType, MarketingCampaignType, ActiveMarketingCampaign
use band::Band;
use events::EventManager;
use player::Player;
use rand::Rng;
use serde::{Deserialize, Serialize};
use std::fs::File;
use std::io::{Read, Write};
use timeline::MusicTimeline;
use world::{GameWorld, PotentialDealOffer, MusicGenre}; // Added MusicGenre

// Existing Royalty Constants (Kept for reference if any old logic uses them, but new model is primary)
const BASE_ALBUM_ROYALTY_PAYMENT: u32 = 500;
const BASE_SINGLE_ROYALTY_PAYMENT: u32 = 50;
const ROYALTY_FAME_MULTIPLIER: f32 = 10.0;

// New Quality Calculation Constants
const QUALITY_BASE_SONGWRITING: u8 = 30;
const QUALITY_SONGWRITING_MAX_BONUS_PLAYER_STATS: u8 = 25;
const QUALITY_SONGWRITING_RANDOM_VARIATION: u8 = 10;
const QUALITY_BASE_RECORDING: u8 = 30;
const QUALITY_RECORDING_MAX_BONUS_PLAYER_STATS: u8 = 20;
const QUALITY_RECORDING_RANDOM_VARIATION: u8 = 10;

// New Sales Model Constants
const INITIAL_SALES_WINDOW_WEEKS: u32 = 4;
const MARKETING_EFFECTIVENESS_DECAY_RATE: f32 = 0.90; // Not used in this simplified model yet
const SALES_SCORE_BASE: u32 = 50; // Not directly used in this specific formula, but good for reference
const SALES_QUALITY_WEIGHT: f32 = 2.5;
const SALES_MARKETING_WEIGHT: f32 = 1.8;
const SALES_FAME_WEIGHT: f32 = 1.2;
const SALES_MARKET_DEMAND_WEIGHT: f32 = 1.0; // Not directly used in this specific formula
const SALES_SATURATION_DIVISOR: f32 = 200.0; // Not directly used in this specific formula
const INDEPENDENT_INCOME_PER_SCORE_POINT: u32 = 20;
const LABEL_INCOME_PER_SCORE_POINT: u32 = 30;
const PLAYER_MARKET_IMPACT_THRESHOLD_SALES_SCORE: u32 = 600;
const PLAYER_MARKET_IMPACT_GENRE_MOD_BONUS: f32 = 0.05;
const PLAYER_MARKET_IMPACT_DEMAND_BONUS: u8 = 1;


#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum GameAction {
    LazeAround,
    WriteSongs,
    Practice,
    RecordSingle,
    RecordAlbum,
    Gig,
    GoOnTour,
    TakeBreak,
    VisitDoctor,
    SaveGame,
    LoadGame,
    ViewDealOffers,
    AcceptDeal(usize),
    RejectDeal(usize),
    StartMarketingCampaign(u32, MarketingCampaignType), // release_id, campaign_type
    Quit,
}

#[derive(Serialize, Deserialize)]
pub struct Game {
    pub player: Player,
    pub band: Band,
    pub world: GameWorld,
    pub events: EventManager,
    pub timeline: MusicTimeline,
    #[serde(skip)]
    pub data_files: GameDataFiles,
    pub pending_deal_offers: Vec<PotentialDealOffer>,
    pub week: u32,
    pub game_over: bool,
    pub next_song_id: u32,
    pub next_release_id: u32,
    pub just_released_music: Vec<Release>, // Stores releases for their initial sales window
}

impl Game {
    pub fn new() -> Result<Self, Box<dyn std::error::Error>> {
        GameDataFiles::validate_data_files()?;
        let data_files = GameDataFiles::load()?;
        Ok(Self {
            player: Player::default(),
            band: Band::default(),
            world: GameWorld::new(&data_files),
            events: EventManager::new(),
            timeline: MusicTimeline::new(&data_files),
            data_files,
            pending_deal_offers: Vec::new(),
            week: 1,
            game_over: false,
            next_song_id: 0,
            next_release_id: 0,
            just_released_music: Vec::new(),
        })
    }

    pub fn initialize_player(&mut self, player_name: &str, band_name: &str) {
        self.player.name = player_name.to_string();
        self.band.name = band_name.to_string();
        self.player.money = 500; // Starting cash in 1970

        self.band.members = vec![
            band::BandMember {
                name: self.data_files.random_band_member_name(),
                instrument: band::Instrument::Guitar,
                skill: 25,
                loyalty: 75,
                drug_problem: false,
            },
            band::BandMember {
                name: self.data_files.random_band_member_name(),
                instrument: band::Instrument::Bass,
                skill: 20,
                loyalty: 80,
                drug_problem: false,
            },
            band::BandMember {
                name: self.data_files.random_band_member_name(),
                instrument: band::Instrument::Drums,
                skill: 30,
                loyalty: 70,
                drug_problem: false,
            },
        ];
    }

    // --- Song and Release Calculation Helper Methods (Step 4) ---
    fn calculate_songwriting_quality(&self) -> u8 {
        let mut quality = QUALITY_BASE_SONGWRITING as f32;
        let mut player_bonus = 0.0;

        // Player energy bonus
        if self.player.energy > 70 { player_bonus += 5.0; }
        else if self.player.energy > 40 { player_bonus += 2.0; }

        // Player stress bonus (low stress is good)
        if self.player.stress < 30 { player_bonus += 5.0; }
        else if self.player.stress < 60 { player_bonus += 2.0; }
        
        // Band member skill bonus
        player_bonus += (self.band.average_member_skill() / 15) as f32; 
        
        quality += player_bonus.min(QUALITY_SONGWRITING_MAX_BONUS_PLAYER_STATS as f32);

        // Random variation
        let mut rng = rand::thread_rng();
        let random_offset = rng.gen_range(0..=QUALITY_SONGWRITING_RANDOM_VARIATION) as i8 - (QUALITY_SONGWRITING_RANDOM_VARIATION / 2) as i8;
        quality += random_offset as f32;
        
        quality.clamp(1.0, 100.0) as u8
    }

    fn get_selected_songs_for_release(&mut self, count: usize) -> Result<(Vec<music::Song>, u8), String> {
        if self.band.unreleased_songs.len() < count {
            return Err(format!("Not enough unreleased songs. Need {}, have {}.", count, self.band.unreleased_songs.len()));
        }
        
        let selected_songs: Vec<music::Song> = self.band.unreleased_songs.drain((self.band.unreleased_songs.len() - count)..).collect();
        
        if selected_songs.is_empty() && count > 0 {
            return Err("No songs were selected, though count was > 0.".to_string());
        }
        if count == 0 { 
             return Ok((Vec::new(), 0));
        }

        let total_quality: u32 = selected_songs.iter().map(|s| s.songwriting_quality as u32).sum();
        let avg_quality = (total_quality / selected_songs.len() as u32) as u8;
        
        Ok((selected_songs, avg_quality))
    }

    fn calculate_release_quality(&self, avg_song_quality: u8) -> u8 {
        let mut quality = (QUALITY_BASE_RECORDING as f32 + avg_song_quality as f32) / 2.0; 
        
        quality += (self.band.skill / 10) as f32; 

        let mut player_bonus: f32 = 0.0;
        if self.player.energy > 70 { player_bonus += 3.0; }
        else if self.player.energy > 40 { player_bonus += 1.0; }
        if self.player.stress < 30 { player_bonus += 3.0; }
        else if self.player.stress < 60 { player_bonus += 1.0; }
        quality += player_bonus.min(QUALITY_RECORDING_MAX_BONUS_PLAYER_STATS as f32);

        let mut rng = rand::thread_rng();
        let random_offset = rng.gen_range(0..=QUALITY_RECORDING_RANDOM_VARIATION) as i8 - (QUALITY_RECORDING_RANDOM_VARIATION / 2) as i8;
        quality += random_offset as f32;
        
        quality.clamp( (avg_song_quality as f32 / 2.0).max(1.0) , 100.0) as u8
    }
    
    fn calculate_release_sales_score(&self, release: &Release) -> u32 {
        let quality_score = release.release_quality as f32 * SALES_QUALITY_WEIGHT;
        let marketing_score = release.marketing_level_achieved as f32 * SALES_MARKETING_WEIGHT;
        let fame_score = self.band.fame as f32 * SALES_FAME_WEIGHT;

        let era_sales_modifier = self.timeline.get_current_era().market_conditions.record_sales_growth / 100.0 + 1.0;
        
        let genre_modifier = release.genre.as_ref()
            .and_then(|g| self.world.dynamic_genre_modifiers.get(g).copied())
            .unwrap_or(1.0);

        let base_score = quality_score + marketing_score + fame_score;
        (base_score * era_sales_modifier * genre_modifier).max(0.0) as u32
    }

    fn calculate_income_from_sales_score(&self, sales_score: u32, _release_type: &ReleaseType) -> u32 {
        let base_income_per_point = if self.band.current_deal().is_some() {
            LABEL_INCOME_PER_SCORE_POINT 
        } else {
            INDEPENDENT_INCOME_PER_SCORE_POINT
        };
        
        let total_label_income = sales_score * base_income_per_point;

        if let Some(deal) = self.band.current_deal() {
            (total_label_income as f32 * deal.royalty_rate) as u32 
        } else {
            total_label_income 
        }
    }

    // --- Action Helper Methods (Step 5) ---
    fn action_laze_around(&mut self) -> Result<(), String> {
        self.player.energy = (self.player.energy + 20).min(constants::MAX_ENERGY);
        self.player.stress = (self.player.stress.saturating_sub(10)).max(0);
        Ok(())
    }

    fn action_write_songs(&mut self) -> Result<(), String> {
        if self.player.energy < 20 {
            return Err("You're too tired to write songs!".to_string());
        }
        self.player.energy -= 20;
        
        let num_songs_to_write = rand::thread_rng().gen_range(1..=3);
        for _ in 0..num_songs_to_write {
            let quality = self.calculate_songwriting_quality();
            let song_name = self.data_files.random_song_title(); 
            self.band.unreleased_songs.push(music::Song { 
                id: self.next_song_id,
                name: song_name,
                songwriting_quality: quality,
            });
            self.next_song_id += 1;
        }
        Ok(())
    }

    fn action_practice(&mut self) -> Result<(), String> {
        if self.player.energy < 15 {
            return Err("You're too tired to practice!".to_string());
        }
        self.player.energy -= 15;
        self.band.skill = (self.band.skill + 2).min(constants::MAX_SKILL);
        Ok(())
    }

    fn action_record_single(&mut self) -> Result<(), String> {
        if !self.band.can_record_single() {
             return Err("You need to write at least one song first!".to_string());
        }
        let (selected_songs, avg_song_quality) = self.get_selected_songs_for_release(1)?;
         if selected_songs.is_empty() {
            return Err("Failed to select a song for the single.".to_string());
        }

        let cost = ((constants::SINGLE_RECORDING_COST as f32)
            * self.timeline.get_recording_cost_modifier()) as i32;
        if !self.player.can_afford(cost) {
            return Err(format!("You need at least ${} to record a single!", cost));
        }
        self.player.spend_money(cost);

        let release_quality = self.calculate_release_quality(avg_song_quality);
        let release_name = format!("Single: {}", selected_songs[0].name);
        
        let new_release = music::Release {
            id: self.next_release_id,
            name: release_name,
            release_type: music::ReleaseType::Single,
            release_quality,
            week_released: self.week,
            songs_involved_quality_avg: avg_song_quality,
            active_marketing: Vec::new(),
            marketing_level_achieved: 0,
            initial_sales_score: 0,
            total_income_generated: 0,
            genre: selected_songs.first().and_then(|_s| Some(world::MusicGenre::Rock)), // Placeholder
        };
        self.just_released_music.push(new_release);
        self.next_release_id += 1;
        Ok(())
    }

    fn action_record_album(&mut self) -> Result<(), String> {
        if !self.band.can_record_album() {
            return Err(format!("You need at least {} unreleased songs to record an album!", constants::MIN_ALBUM_SONGS));
        }
        let (selected_songs, avg_song_quality) = self.get_selected_songs_for_release(constants::MIN_ALBUM_SONGS as usize)?;
        if selected_songs.len() < constants::MIN_ALBUM_SONGS as usize {
             return Err("Not enough songs selected for an album.".to_string());
        }

        let cost = ((constants::ALBUM_RECORDING_BASE_COST as f32)
            * self.timeline.get_recording_cost_modifier()) as i32;
        if !self.player.can_afford(cost) {
            return Err(format!("You need at least ${} to record an album!", cost));
        }
        self.player.spend_money(cost);

        let release_quality = self.calculate_release_quality(avg_song_quality);
        let release_name = self.data_files.random_album_title(); 
        
        let new_release = music::Release {
            id: self.next_release_id,
            name: release_name,
            release_type: music::ReleaseType::Album,
            release_quality,
            week_released: self.week,
            songs_involved_quality_avg: avg_song_quality,
            active_marketing: Vec::new(),
            marketing_level_achieved: 0,
            initial_sales_score: 0,
            total_income_generated: 0,
            genre: selected_songs.first().and_then(|_s| Some(world::MusicGenre::Rock)), // Placeholder
        };
        self.just_released_music.push(new_release);
        self.next_release_id += 1;

        if self.timeline.is_album_era() {
            self.band.fame = (self.band.fame + 3).min(constants::MAX_FAME);
        }
        Ok(())
    }
    
    fn action_start_marketing_campaign(&mut self, release_id: u32, campaign_type: MarketingCampaignType) -> Result<(), String> {
        // Find in just_released_music first, then in already released music
        let release_to_market = self.just_released_music.iter_mut()
            .find(|r| r.id == release_id)
            .or_else(|| self.band.singles_released.iter_mut().find(|r| r.id == release_id))
            .or_else(|| self.band.albums_released.iter_mut().find(|r| r.id == release_id));


        if let Some(release) = release_to_market {
            let (cost, duration_weeks, effectiveness_bonus) = match campaign_type {
                MarketingCampaignType::BasicPress => (100, 4, 5),
                MarketingCampaignType::RadioPromotion => (500, 6, 15),
                MarketingCampaignType::MusicVideo => (2000, 8, 30),
                MarketingCampaignType::SocialMediaBlitz => (750, 4, 20), 
                MarketingCampaignType::MagazineSpread => (300, 4, 10),
            };

            if !self.player.can_afford(cost) {
                return Err(format!("Not enough money for {:?} campaign. Need ${}.", campaign_type, cost));
            }
            self.player.spend_money(cost);

            release.active_marketing.push(ActiveMarketingCampaign {
                campaign_type,
                start_week: self.week,
                end_week: self.week + duration_weeks,
                effectiveness_bonus,
            });
            
            release.marketing_level_achieved = release.active_marketing.iter()
                .map(|c| c.effectiveness_bonus)
                .sum();
            
            Ok(())
        } else {
            Err(format!("Release with ID {} not found to start marketing campaign.", release_id))
        }
    }

    fn action_play_gig(&mut self) -> Result<(), String> {
        if self.player.energy < 30 {
            return Err("You're too tired to perform!".to_string());
        }
        self.player.energy -= 30;
        let earnings = self.calculate_gig_earnings();
        self.player.earn_money(earnings);
        self.band.fame = (self.band.fame + 1).min(constants::MAX_FAME);
        Ok(())
    }

    fn action_go_on_tour(&mut self) -> Result<(), String> {
        if self.player.energy < 40 {
            return Err("You're too tired to go on tour!".to_string());
        }
        if self.band.fame < 25 {
            return Err("You need more fame before promoters will book a tour!".to_string());
        }
        if !self.player.can_afford(2000) {
            return Err("You need at least $2000 to finance a tour!".to_string());
        }

        let (tour_cost, tour_earnings, tour_weeks, fame_gain) = if self.band.fame >= 70 {
            (5000, 15000 + (self.band.fame as i32 * 200), 4, 8)
        } else if self.band.fame >= 50 {
            (3000, 8000 + (self.band.fame as i32 * 100), 3, 5)
        } else {
            (2000, 4000 + (self.band.fame as i32 * 50), 2, 3)
        };

        let era_modifier = self.timeline.get_gig_pay_modifier();
        let market_modifier = self.world.get_market_modifier();
        let final_earnings = ((tour_earnings as f32) * era_modifier * market_modifier) as i32;

        self.player.spend_money(tour_cost);
        self.player.earn_money(final_earnings as u32);
        self.player.energy -= 40;
        self.player.stress = (self.player.stress + 30).min(constants::MAX_STRESS);
        self.band.fame = (self.band.fame + fame_gain).min(constants::MAX_FAME);
        self.week += tour_weeks;

        let mut rng = rand::thread_rng();
        if rng.gen_bool(0.3) {
            self.band.fame = (self.band.fame + 2).min(constants::MAX_FAME);
        } else if rng.gen_bool(0.15) {
            self.player.health = self.player.health.saturating_sub(10);
        }
        Ok(())
    }

    fn action_take_break(&mut self) -> Result<(), String> {
        self.player.energy = constants::MAX_ENERGY;
        self.player.stress = 0;
        self.player.health = (self.player.health + 10).min(constants::MAX_HEALTH);
        Ok(())
    }

    fn action_visit_doctor(&mut self) -> Result<(), String> {
        if !self.player.can_afford(constants::DOCTOR_VISIT_COST) {
            return Err(format!("You need ${} to visit the doctor!", constants::DOCTOR_VISIT_COST));
        }
        self.player.spend_money(constants::DOCTOR_VISIT_COST);
        self.player.health = (self.player.health + 20).min(constants::MAX_HEALTH);
        Ok(())
    }

    fn action_accept_deal(&mut self, offer_index: usize) -> Result<(), String> {
        if offer_index >= self.pending_deal_offers.len() {
            return Err("Invalid deal offer selected.".to_string());
        }
        let offer = self.pending_deal_offers.remove(offer_index);
        let new_deal = band::RecordDeal {
            label_name: offer.label_name,
            label_tier: offer.label_tier,
            advance: offer.advance,
            royalty_rate: offer.royalty_rate,
            albums_required: offer.albums_required,
            albums_delivered: 0,
        };
        self.band.sign_deal(new_deal);
        self.player.earn_money(offer.advance);
        self.pending_deal_offers.clear();
        Ok(())
    }

    fn action_reject_deal(&mut self, offer_index: usize) -> Result<(), String> {
        if offer_index >= self.pending_deal_offers.len() {
            return Err("Invalid deal offer selected.".to_string());
        }
        self.pending_deal_offers.remove(offer_index);
        Ok(())
    }
    
    // --- Main execute_action ---
    fn execute_action(&mut self, action: GameAction) -> Result<(), String> {
        match action {
            GameAction::LazeAround => self.action_laze_around(),
            GameAction::WriteSongs => self.action_write_songs(),
            GameAction::Practice => self.action_practice(),
            GameAction::RecordSingle => self.action_record_single(),
            GameAction::RecordAlbum => self.action_record_album(),
            GameAction::Gig => self.action_play_gig(),
            GameAction::GoOnTour => self.action_go_on_tour(),
            GameAction::TakeBreak => self.action_take_break(),
            GameAction::VisitDoctor => self.action_visit_doctor(),
            GameAction::AcceptDeal(index) => self.action_accept_deal(index),
            GameAction::RejectDeal(index) => self.action_reject_deal(index),
            GameAction::StartMarketingCampaign(release_id, campaign_type) => self.action_start_marketing_campaign(release_id, campaign_type),
            GameAction::Quit => {
                self.game_over = true;
                Ok(())
            }
            GameAction::SaveGame | GameAction::LoadGame | GameAction::ViewDealOffers => {
                Ok(()) 
            }
        }
    }
    
    // --- Turn Processing Helper Methods (Step 6) ---
    fn process_music_releases_and_marketing(&mut self) {
        let current_week = self.week; 

        let mut still_pending_release = Vec::new();
        for mut release in self.just_released_music.drain(..) {
            if current_week >= release.week_released + INITIAL_SALES_WINDOW_WEEKS { 
                let sales_score = self.calculate_release_sales_score(&release);
                release.initial_sales_score = sales_score;
                
                let income = self.calculate_income_from_sales_score(sales_score, &release.release_type);
                release.total_income_generated += income;
                self.player.earn_money(income);

                if release.release_type == music::ReleaseType::Album {
                    if self.band.current_deal().is_some() {
                        if self.band.fulfill_album_obligation() {
                            // TODO: Consider a message for deal fulfillment.
                        }
                    }
                    self.band.albums_released.push(release);
                } else {
                    self.band.singles_released.push(release);
                }

                if sales_score > PLAYER_MARKET_IMPACT_THRESHOLD_SALES_SCORE {
                    if let Some(genre_to_boost) = self.band.albums_released.last().or_else(|| self.band.singles_released.last()).and_then(|r| r.genre.clone()) {
                        *self.world.dynamic_genre_modifiers.entry(genre_to_boost).or_insert(1.0) += PLAYER_MARKET_IMPACT_GENRE_MOD_BONUS;
                    }
                    self.world.music_market.demand = (self.world.music_market.demand + PLAYER_MARKET_IMPACT_DEMAND_BONUS).min(100);
                }

            } else {
                still_pending_release.push(release);
            }
        }
        self.just_released_music = still_pending_release;

        for release_list in [&mut self.band.albums_released, &mut self.band.singles_released] {
            for release in release_list.iter_mut() {
                release.active_marketing.retain(|campaign| current_week < campaign.end_week);
                release.marketing_level_achieved = release.active_marketing.iter()
                    .map(|c| c.effectiveness_bonus)
                    .sum();
                
                if release.initial_sales_score > 0 && current_week > release.week_released + INITIAL_SALES_WINDOW_WEEKS {
                     let weeks_since_initial_window_end = current_week - (release.week_released + INITIAL_SALES_WINDOW_WEEKS -1);
                     let ongoing_sales_score_divisor = 1 + weeks_since_initial_window_end; 
                     let ongoing_sales_score = release.initial_sales_score / ongoing_sales_score_divisor; 

                     if ongoing_sales_score > 10 { 
                        let ongoing_income = self.calculate_income_from_sales_score(ongoing_sales_score, &release.release_type) / 5; 
                        release.total_income_generated += ongoing_income;
                        self.player.earn_money(ongoing_income);
                     }
                }
            }
        }
    }

    fn advance_week_events(&mut self) -> Result<(), String> {
        if self.week % constants::WEEKS_PER_YEAR == 0 {
            self.timeline.advance_year();
        }

        if let Some(event) = self.events.try_trigger_event(self.week) {
            self.apply_random_event(event)?;
        }

        self.player.weekly_health_decay();

        if let Some(historical_event) = self.timeline.should_trigger_historical_event() {
            self.apply_historical_event(&historical_event)?;
        }

        self.world.update_week(&self.timeline);
        Ok(())
    }

    fn check_and_generate_deal_offers(&mut self) {
        if self.pending_deal_offers.is_empty() && self.week % 4 == 0 && self.band.record_deal.is_none() {
            let mut rng = rand::thread_rng();
            let new_offers = self.world.generate_deal_offers(&self.band, &self.data_files, &mut rng);
            if !new_offers.is_empty() {
                self.pending_deal_offers = new_offers;
            }
        }
    }
    
    pub fn process_turn(&mut self, action: GameAction) -> Result<bool, String> {
        if self.game_over {
            return Ok(false);
        }

        let is_turn_consuming_action = match action {
            GameAction::SaveGame | GameAction::LoadGame | GameAction::ViewDealOffers | 
            GameAction::AcceptDeal(_) | GameAction::RejectDeal(_) | GameAction::StartMarketingCampaign(_,_) => false,
            _ => true,
        };

        self.execute_action(action.clone())?; // Execute action first

        if is_turn_consuming_action {
            self.week += 1; // Advance week only for turn-consuming actions
            if self.week % constants::WEEKS_PER_YEAR == 0 {
                self.timeline.advance_year();
            }
            self.advance_week_events()?; // Process standard weekly events
        }
        
        // These happen after every action resolution, regardless of turn consumption
        self.process_music_releases_and_marketing();
        self.check_and_generate_deal_offers();
        self.check_game_over();

        Ok(!self.game_over)
    }

    // --- Original methods (ensure they are present and correct) ---
    // calculate_royalties is removed as income is now handled by calculate_income_from_sales_score

    fn calculate_gig_earnings(&self) -> u32 {
        let base_pay = 50u32;
        let fame_bonus = (self.band.fame as u32) / 10;
        let skill_bonus = (self.band.skill as u32) / 20;
        base_pay + fame_bonus + skill_bonus
    }

    fn check_game_over(&mut self) {
        if self.player.health <= 0 {
            self.game_over = true;
        }
        if self.player.money < 0 && self.band.fame < 10 {
            self.game_over = true;
        }
        if self.band.fame >= constants::ROCKSTAR_FAME_THRESHOLD
            && self.band.albums_released.len() >= constants::ROCKSTAR_ALBUM_THRESHOLD as usize // Updated to check Vec length
        {
            self.game_over = true;
        }
    }

    pub fn is_game_over(&self) -> bool {
        self.game_over
    }

    pub fn get_status_message(&self) -> String {
        if self.player.health <= 0 {
            "You died from poor health!".to_string()
        } else if self.player.money < 0 && self.band.fame < 10 {
            "You went broke and nobody knows who you are!".to_string()
        } else if self.band.fame >= constants::ROCKSTAR_FAME_THRESHOLD
            && self.band.albums_released.len() >= constants::ROCKSTAR_ALBUM_THRESHOLD as usize
        {
            "Congratulations! You're now a ROCKSTAR!".to_string()
        } else {
            "Game continues...".to_string()
        }
    }

    fn apply_random_event(&mut self, event: events::RandomEvent) -> Result<(), String> {
        use events::RandomEvent;
        let mut rng = rand::thread_rng();

        match event {
            RandomEvent::DrugOffer => {
                if rng.gen_bool(0.3) {
                    self.player.energy = (self.player.energy + 20).min(constants::MAX_ENERGY);
                    self.player.drug_addiction =
                        (self.player.drug_addiction + 10).min(constants::MAX_STRESS);
                    self.player.health = self.player.health.saturating_sub(5);
                }
            }
            RandomEvent::EquipmentIssue => {
                match rng.gen_range(0..3) {
                    0 => {
                        let repair_cost = rng.gen_range(
                            constants::EQUIPMENT_REPAIR_COST_RANGE.0
                                ..=constants::EQUIPMENT_REPAIR_COST_RANGE.1,
                        );
                        if self.player.can_afford(repair_cost) {
                            self.player.spend_money(repair_cost);
                        } else {
                            self.band.skill = self.band.skill.saturating_sub(5);
                        }
                    }
                    1 => {
                        self.band.skill = (self.band.skill + 5).min(constants::MAX_SKILL);
                    }
                    _ => {
                        let loss = rng.gen_range(100..500);
                        if self.player.can_afford(loss) {
                            self.player.spend_money(loss);
                        } else {
                            self.player.money = 0;
                        }
                        self.band.skill = self.band.skill.saturating_sub(3);
                    }
                }
            }
            RandomEvent::BandMemberIssue => {
                if !self.band.members.is_empty() {
                    let member_idx = rng.gen_range(0..self.band.members.len());
                    let member = &mut self.band.members[member_idx];

                    match rng.gen_range(0..4) {
                        0 => {
                            member.skill = (member.skill + 5).min(100);
                            member.loyalty = (member.loyalty + 10).min(100);
                        }
                        1 => {
                            member.loyalty = member.loyalty.saturating_sub(15);
                            if rng.gen_bool(0.3) {
                                member.drug_problem = true;
                            }
                        }
                        2 => {
                            if member.loyalty < 30 {
                                member.loyalty = 0; 
                            }
                        }
                        _ => {
                            let demand = rng.gen_range(100..300);
                            self.player.money -= demand;
                        }
                    }
                }
            }
            RandomEvent::MediaEvent => match rng.gen_range(0..3) {
                0 => {
                    self.band.fame =
                        (self.band.fame + rng.gen_range(3..8)).min(constants::MAX_FAME);
                    self.band.reputation.media_presence =
                        (self.band.reputation.media_presence + 5).min(100);
                }
                1 => {
                    self.band.fame = self.band.fame.saturating_sub(rng.gen_range(2..6));
                    self.band.reputation.media_presence =
                        self.band.reputation.media_presence.saturating_sub(8);
                }
                _ => {
                    self.band.fame = self.band.fame.saturating_sub(rng.gen_range(5..15));
                    self.player.stress = (self.player.stress + 20).min(constants::MAX_STRESS);
                }
            },
            RandomEvent::HealthEvent => match rng.gen_range(0..3) {
                0 => {
                    self.player.health = self.player.health.saturating_sub(rng.gen_range(10..25));
                    self.player.energy = self.player.energy.saturating_sub(30);
                }
                1 => {
                    self.player.health = self.player.health.saturating_sub(rng.gen_range(5..15));
                    self.band.skill = self.band.skill.saturating_sub(5);
                }
                _ => {
                    self.player.stress =
                        (self.player.stress + rng.gen_range(15..30)).min(constants::MAX_STRESS);
                    self.player.energy = self.player.energy.saturating_sub(20);
                }
            },
            RandomEvent::MoneyEvent => {
                match rng.gen_range(0..4) {
                    0 => {
                        let amount = rng.gen_range(200..1000);
                        self.player.earn_money(amount as u32);
                    }
                    1 => {
                        let amount = rng.gen_range(100..500);
                        if self.player.can_afford(amount) {
                            self.player.spend_money(amount);
                        } else {
                            self.player.money = 0; 
                        }
                    }
                    2 => {
                        // Simplified: Royalty for *all* past releases, not just current one.
                        let total_releases_count = self.band.albums_released.len() + self.band.singles_released.len();
                        let royalties = (total_releases_count as i32) * rng.gen_range(10..50);
                        self.player.earn_money(royalties as u32);
                    }
                    _ => {
                        let cost = rng.gen_range(500..2000);
                        if self.player.can_afford(cost) {
                            self.player.spend_money(cost);
                        } else {
                            self.player.money = 0; 
                        }
                        self.band.fame = self.band.fame.saturating_sub(5);
                    }
                }
            }
            RandomEvent::IndustryEvent => match rng.gen_range(0..3) {
                0 => {
                    if !self.band.has_record_deal() && self.band.fame > 30 {
                        self.band.fame = (self.band.fame + 5).min(constants::MAX_FAME);
                    }
                }
                1 => {
                    if self.band.fame > 20 {
                        let payment = rng.gen_range(500..2000);
                        self.player.earn_money(payment as u32);
                        self.band.fame = (self.band.fame + 3).min(constants::MAX_FAME);
                    }
                }
                _ => {}
            },
        }

        Ok(())
    }

    fn apply_historical_event(&mut self, event: &str) -> Result<(), String> {
        let mut rng = rand::thread_rng();

        match event {
            event if event.contains("Beatles") => {
                if self.band.dominant_genres_match(&["Rock", "Folk Rock"]) {
                    self.band.fame = (self.band.fame + 5).min(constants::MAX_FAME);
                    self.player.money += 200;
                }
            }
            event if event.contains("MTV") => {
                if self.timeline.get_image_importance() > 80 {
                    if self.band.reputation.media_presence > 60 {
                        self.band.fame = (self.band.fame + 10).min(constants::MAX_FAME);
                        let earnings = rng.gen_range(1000..3000);
                        self.player.money += earnings;
                    } else {
                        self.band.fame = self.band.fame.saturating_sub(5);
                    }
                }
            }
            event if event.contains("Grunge emerges") => {
                if self.band.dominant_genres_match(&["Grunge", "Alternative"]) {
                    self.band.fame = (self.band.fame + 12).min(constants::MAX_FAME);
                    let major_earnings = rng.gen_range(2000..5000);
                    self.player.money += major_earnings;
                } else if self
                    .band
                    .dominant_genres_match(&["Hair Metal", "Pop Metal"])
                {
                    self.band.fame = self.band.fame.saturating_sub(8);
                }
            }
            _ => {
                match rng.gen_range(0..3) {
                    0 => self.band.fame = (self.band.fame + 1).min(constants::MAX_FAME),
                    1 => self.player.money += rng.gen_range(50..200),
                    _ => {
                        self.band.reputation.critical_acclaim =
                            (self.band.reputation.critical_acclaim + 1).min(100)
                    }
                }
            }
        }

        Ok(())
    }

    pub fn get_last_historical_event(&self) -> Option<String> {
        self.timeline.should_trigger_historical_event()
    }

    pub fn save_game(&self, file_path: &str) -> Result<(), Box<dyn std::error::Error>> {
        let json_string = serde_json::to_string_pretty(self)?;
        let mut file = File::create(file_path)?;
        file.write_all(json_string.as_bytes())?;
        Ok(())
    }

    pub fn load_game(file_path: &str) -> Result<Self, Box<dyn std::error::Error>> {
        let mut file = File::open(file_path)?;
        let mut json_string = String::new();
        file.read_to_string(&mut json_string)?;

        let mut loaded_game: Game = serde_json::from_str(&json_string)?;
        
        loaded_game.data_files = GameDataFiles::load()?;

        Ok(loaded_game)
    }
}
