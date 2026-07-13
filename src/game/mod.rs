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
use rand::{Rng, SeedableRng};
use rand::rngs::StdRng;
use serde::{Deserialize, Serialize};
use std::fs::File;
use std::io::{Read, Write};
use timeline::MusicTimeline;
use world::{GameWorld, PotentialDealOffer};

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

// Distribution model: how much of a release's potential audience you can
// actually reach. Labels bring their market_reach; independents are capped
// by their own fame and pay to press and ship records themselves.
const INDIE_REACH_FLOOR: f32 = 0.15;
const INDIE_DISTRIBUTION_BASE_SINGLE: f32 = 60.0;
const INDIE_DISTRIBUTION_BASE_ALBUM: f32 = 500.0;

// Support tours: bigger acts occasionally want you as their opener.
const SUPPORT_OFFER_MIN_FAME: u8 = 5;
const SUPPORT_OFFER_FAME_GAP: u8 = 10;
const SUPPORT_OFFER_CHANCE: f64 = 0.12;
const SUPPORT_OFFER_LIFETIME_WEEKS: u32 = 3;
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
    Gig(usize),
    GoOnTour(usize),
    TakeBreak,
    VisitDoctor,
    AcceptDeal(usize),
    RejectDeal(usize),
    AcceptSupportTour,
    DeclineSupportTour,
    StartMarketingCampaign(u32, MarketingCampaignType), // release_id, campaign_type
    Quit,
}

/// An invitation from a bigger act to open on their tour.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SupportTourOffer {
    pub host_band: String,
    pub host_fame: u8,
    pub weeks: u32,
    pub pay: u32,
    pub fame_gain: u8,
    pub expires_week: u32,
}

fn default_seed() -> u64 {
    rand::random::<u64>()
}

#[derive(Serialize, Deserialize)]
pub struct Game {
    #[serde(default = "default_seed")]
    pub world_seed: u64,
    pub player: Player,
    pub band: Band,
    pub world: GameWorld,
    pub events: EventManager,
    pub timeline: MusicTimeline,
    #[serde(skip)]
    pub data_files: GameDataFiles,
    pub pending_deal_offers: Vec<PotentialDealOffer>,
    #[serde(default)]
    pub pending_support_offer: Option<SupportTourOffer>,
    #[serde(default)]
    pub regional_fame: std::collections::HashMap<String, u8>,
    pub week: u32,
    pub game_over: bool,
    pub next_song_id: u32,
    pub next_release_id: u32,
    pub just_released_music: Vec<Release>, // Stores releases for their initial sales window
    /// Messages produced while processing the last turn, drained by the UI.
    #[serde(skip)]
    pub turn_log: Vec<String>,
}

impl Game {
    pub fn new() -> Result<Self, Box<dyn std::error::Error>> {
        GameDataFiles::validate_data_files()?;
        let data_files = GameDataFiles::load()?;

        let world_seed = std::env::var("ROCKER_SEED")
            .ok()
            .and_then(|s| s.parse::<u64>().ok())
            .unwrap_or_else(|| rand::random::<u64>());

        let mut init_rng = StdRng::seed_from_u64(world_seed);
        let world = GameWorld::new(&data_files, &mut init_rng);

        let mut turn_log = Vec::new();
        turn_log.push(format!("🌱 World Seed: {}", world_seed));

        Ok(Self {
            world_seed,
            player: Player::default(),
            band: Band::default(),
            world,
            events: EventManager::new(),
            timeline: MusicTimeline::new(&data_files),
            data_files,
            pending_deal_offers: Vec::new(),
            pending_support_offer: None,
            regional_fame: std::collections::HashMap::new(),
            week: 1,
            game_over: false,
            next_song_id: 0,
            next_release_id: 0,
            just_released_music: Vec::new(),
            turn_log,
        })
    }

    fn log(&mut self, message: impl Into<String>) {
        self.turn_log.push(message.into());
    }

    pub fn take_turn_log(&mut self) -> Vec<String> {
        std::mem::take(&mut self.turn_log)
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

    /// How much of a release's potential audience the band can actually reach.
    /// A label brings its distribution network; an independent act is capped
    /// by its own fame — a nobody pressing records sells them locally at best.
    fn distribution_multiplier(&self) -> f32 {
        match self.band.current_deal() {
            Some(deal) => 0.5 + f32::from(deal.market_reach) / 100.0,
            None => {
                INDIE_REACH_FLOOR + (f32::from(self.band.fame) / 100.0) * (1.0 - INDIE_REACH_FLOOR)
            }
        }
    }

    /// What it costs to press and ship a release yourself. Zero when signed —
    /// the label handles it. Scales with fame: a big act presses a big run.
    pub fn independent_distribution_cost(&self, release_type: &ReleaseType) -> i32 {
        if self.band.current_deal().is_some() {
            return 0;
        }
        let base = match release_type {
            ReleaseType::Single => INDIE_DISTRIBUTION_BASE_SINGLE,
            ReleaseType::Album => INDIE_DISTRIBUTION_BASE_ALBUM,
        };
        let fame_scale = 1.0 + f32::from(self.band.fame) / 20.0;
        (base * fame_scale * self.timeline.get_recording_cost_modifier()) as i32
    }

    /// Total up-front cost of a release: studio time plus, when unsigned,
    /// pressing and shipping it yourself.
    pub fn release_cost(&self, release_type: &ReleaseType) -> i32 {
        let base = match release_type {
            ReleaseType::Single => constants::SINGLE_RECORDING_COST,
            ReleaseType::Album => constants::ALBUM_RECORDING_BASE_COST,
        };
        let recording = (base as f32 * self.timeline.get_recording_cost_modifier()) as i32;
        recording + self.independent_distribution_cost(release_type)
    }

    fn calculate_income_from_sales_score(&self, sales_score: u32, _release_type: &ReleaseType) -> u32 {
        let base_income_per_point = if self.band.current_deal().is_some() {
            LABEL_INCOME_PER_SCORE_POINT
        } else {
            INDEPENDENT_INCOME_PER_SCORE_POINT
        };

        let gross =
            (sales_score as f32 * base_income_per_point as f32 * self.distribution_multiplier()) as u32;

        if let Some(deal) = self.band.current_deal() {
            (gross as f32 * deal.royalty_rate) as u32
        } else {
            gross
        }
    }

    // --- Action Helper Methods (Step 5) ---
    fn action_laze_around(&mut self) -> Result<(), String> {
        self.player.energy = (self.player.energy + 20).min(constants::MAX_ENERGY);
        self.player.stress = self.player.stress.saturating_sub(10);
        self.log("😴 You took it easy this week — energy up, stress down.");
        Ok(())
    }

    fn action_write_songs(&mut self) -> Result<(), String> {
        if self.player.energy < 20 {
            return Err("You're too tired to write songs!".to_string());
        }
        self.player.energy -= 20;

        let num_songs_to_write = rand::thread_rng().gen_range(1..=3);
        let mut titles = Vec::new();
        for _ in 0..num_songs_to_write {
            let quality = self.calculate_songwriting_quality();
            let song_name = self.data_files.random_song_title();
            titles.push(format!("\"{}\"", song_name));
            self.band.unreleased_songs.push(music::Song {
                id: self.next_song_id,
                name: song_name,
                songwriting_quality: quality,
            });
            self.next_song_id += 1;
        }
        self.log(format!(
            "🎼 Wrote {} new song{}: {}",
            num_songs_to_write,
            if num_songs_to_write == 1 { "" } else { "s" },
            titles.join(", ")
        ));
        Ok(())
    }

    fn action_practice(&mut self) -> Result<(), String> {
        if self.player.energy < 15 {
            return Err("You're too tired to practice!".to_string());
        }
        self.player.energy -= 15;
        self.band.skill = (self.band.skill + 2).min(constants::MAX_SKILL);
        let skill = self.band.skill;
        self.log(format!("🥁 A week in the rehearsal room — band skill is now {}%.", skill));
        Ok(())
    }

    fn action_record_single(&mut self) -> Result<(), String> {
        if !self.band.can_record_single() {
             return Err("You need to write at least one song first!".to_string());
        }

        let distribution_cost = self.independent_distribution_cost(&music::ReleaseType::Single);
        let cost = self.release_cost(&music::ReleaseType::Single);
        if !self.player.can_afford(cost) {
            if distribution_cost > 0 {
                return Err(format!(
                    "An independent single costs ${} — ${} studio time plus ${} to press and ship it yourself!",
                    cost,
                    cost - distribution_cost,
                    distribution_cost
                ));
            }
            return Err(format!("You need at least ${} to record a single!", cost));
        }

        let (selected_songs, avg_song_quality) = self.get_selected_songs_for_release(1)?;
        if selected_songs.is_empty() {
            return Err("Failed to select a song for the single.".to_string());
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
            genre: selected_songs.first().map(|_s| world::MusicGenre::Rock), // Placeholder
        };
        let name = new_release.name.clone();
        self.just_released_music.push(new_release);
        self.next_release_id += 1;
        if distribution_cost > 0 {
            self.log(format!(
                "🎙️ Recorded '{}' for ${} and paid ${} to press and ship it yourself — out in {} weeks.",
                name,
                cost - distribution_cost,
                distribution_cost,
                INITIAL_SALES_WINDOW_WEEKS
            ));
        } else {
            self.log(format!(
                "🎙️ Recorded '{}' for ${} — the label ships it in {} weeks. Market it while it's hot!",
                name, cost, INITIAL_SALES_WINDOW_WEEKS
            ));
        }
        Ok(())
    }

    fn action_record_album(&mut self) -> Result<(), String> {
        if !self.band.can_record_album() {
            return Err(format!("You need at least {} unreleased songs to record an album!", constants::MIN_ALBUM_SONGS));
        }

        let distribution_cost = self.independent_distribution_cost(&music::ReleaseType::Album);
        let cost = self.release_cost(&music::ReleaseType::Album);
        if !self.player.can_afford(cost) {
            if distribution_cost > 0 {
                return Err(format!(
                    "An independent album costs ${} — ${} studio time plus ${} to press and ship it yourself!",
                    cost,
                    cost - distribution_cost,
                    distribution_cost
                ));
            }
            return Err(format!("You need at least ${} to record an album!", cost));
        }

        let (selected_songs, avg_song_quality) = self.get_selected_songs_for_release(constants::MIN_ALBUM_SONGS as usize)?;
        if selected_songs.len() < constants::MIN_ALBUM_SONGS as usize {
             return Err("Not enough songs selected for an album.".to_string());
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
            genre: selected_songs.first().map(|_s| world::MusicGenre::Rock), // Placeholder
        };
        let name = new_release.name.clone();
        self.just_released_music.push(new_release);
        self.next_release_id += 1;
        if distribution_cost > 0 {
            self.log(format!(
                "🎙️ Recorded the album '{}' for ${}, plus ${} to press and ship it yourself — out in {} weeks.",
                name,
                cost - distribution_cost,
                distribution_cost,
                INITIAL_SALES_WINDOW_WEEKS
            ));
        } else {
            self.log(format!(
                "🎙️ Recorded the album '{}' for ${} — the label ships it in {} weeks.",
                name, cost, INITIAL_SALES_WINDOW_WEEKS
            ));
        }

        if self.timeline.is_album_era() {
            self.band.fame = (self.band.fame + 3).min(constants::MAX_FAME);
            self.log("📈 It's an album-oriented era — the announcement alone earns you buzz (+3 fame).");
        }
        Ok(())
    }
    
    fn action_start_marketing_campaign(&mut self, release_id: u32, campaign_type: MarketingCampaignType) -> Result<(), String> {
        let spec = campaign_type.spec();
        if !self.player.can_afford(spec.cost) {
            return Err(format!("Not enough money for a {} campaign. Need ${}.", spec.name, spec.cost));
        }

        let current_week = self.week;
        // Find in just_released_music first, then in already released music
        let release = self.just_released_music.iter_mut()
            .find(|r| r.id == release_id)
            .or_else(|| self.band.singles_released.iter_mut().find(|r| r.id == release_id))
            .or_else(|| self.band.albums_released.iter_mut().find(|r| r.id == release_id))
            .ok_or_else(|| format!("Release with ID {} not found to start marketing campaign.", release_id))?;

        release.active_marketing.push(ActiveMarketingCampaign {
            campaign_type,
            start_week: current_week,
            end_week: current_week + spec.duration_weeks,
            effectiveness_bonus: spec.effectiveness_bonus,
        });

        release.marketing_level_achieved = release.active_marketing.iter()
            .map(|c| c.effectiveness_bonus as u32)
            .sum::<u32>()
            .min(100) as u8;
        let release_name = release.name.clone();

        self.player.spend_money(spec.cost);
        self.log(format!(
            "📣 {} campaign launched for '{}' — ${}, runs {} weeks, +{} buzz.",
            spec.name, release_name, spec.cost, spec.duration_weeks, spec.effectiveness_bonus
        ));
        Ok(())
    }

    pub fn get_sorted_regions(&self) -> Vec<(String, String, String, u32, u8, u8)> {
        let mut result = Vec::new();
        let markets_data = &self.data_files.markets_data;
        
        let mut countries: Vec<String> = markets_data.markets.keys().cloned().collect();
        countries.sort();
        
        for country in countries {
            if let Some(c_market) = markets_data.markets.get(&country) {
                let mut regions: Vec<String> = c_market.regions.keys().cloned().collect();
                regions.sort();
                for r_key in regions {
                    if let Some(r_market) = c_market.regions.get(&r_key) {
                        let fame_req = if r_market.population < 3_000_000 {
                            25
                        } else if r_market.population < 7_000_000 {
                            35
                        } else if r_market.population < 10_000_000 {
                            45
                        } else if r_market.population < 15_000_000 {
                            55
                        } else {
                            70
                        };
                        result.push((
                            country.clone(),
                            r_key.clone(),
                            r_market.name.clone(),
                            r_market.population,
                            r_market.economic_strength,
                            fame_req,
                        ));
                    }
                }
            }
        }
        result
    }

    fn action_play_gig(&mut self, venue_index: usize) -> Result<(), String> {
        if self.player.energy < 30 {
            return Err("You're too tired to perform!".to_string());
        }
        if venue_index >= self.world.venues.len() {
            return Err("Invalid venue selected.".to_string());
        }
        let venue = &self.world.venues[venue_index];
        if venue.prestige > self.band.fame.saturating_add(20) {
            return Err(format!("'{}' is out of your league! Get more famous first.", venue.name));
        }

        self.player.energy -= 30;

        let era_modifier = self.timeline.get_gig_pay_modifier();
        let market_modifier = self.world.get_market_modifier();

        let attendance_ratio = ((self.band.fame as f32 + 10.0) / (venue.prestige as f32 + 10.0)).min(1.0);
        let attendance = (venue.capacity as f32 * attendance_ratio) as u32;

        let earnings = (venue.base_payment as f32 * attendance_ratio * market_modifier * era_modifier) as u32;

        let base_fame_gain = if venue.capacity <= 50 {
            1
        } else if venue.capacity <= 200 {
            2
        } else if venue.capacity <= 500 {
            3
        } else if venue.capacity <= 2000 {
            4
        } else {
            6
        };
        let fame_gain = if attendance_ratio < 0.5 {
            (base_fame_gain / 2).max(1)
        } else {
            base_fame_gain
        };

        self.player.earn_money(earnings);
        self.band.fame = (self.band.fame + fame_gain).min(constants::MAX_FAME);
        self.log(format!(
            "🎤 Played at '{}' — sold {}/{} tickets, earned ${}, fame +{}.",
            venue.name, attendance, venue.capacity, earnings, fame_gain
        ));
        Ok(())
    }

    fn action_go_on_tour(&mut self, region_index: usize) -> Result<(), String> {
        if self.player.energy < 40 {
            return Err("You're too tired to go on tour!".to_string());
        }
        
        let sorted_regions = self.get_sorted_regions();
        if region_index >= sorted_regions.len() {
            return Err("Invalid region selected.".to_string());
        }
        
        let (country_key, region_key, region_name, population, economic_strength, fame_req) = &sorted_regions[region_index];
        
        if self.band.fame < *fame_req {
            return Err(format!("Your band needs at least {} fame to tour '{}'.", fame_req, region_name));
        }

        let tier_name = if self.band.fame < 35 {
            "local"
        } else if self.band.fame < 60 {
            "regional"
        } else if self.band.fame < 80 {
            "national"
        } else {
            "international"
        };

        let touring_costs = self.data_files.markets_data.market_modifiers.touring_costs.get(tier_name)
            .ok_or_else(|| "Touring cost tier not found.".to_string())?;

        let country_travel_mult = match country_key.as_str() {
            "united_states" => 1.5,
            "united_kingdom" => 0.8,
            "europe" => 1.2,
            "japan" => 1.0,
            "australia" => 1.4,
            _ => 1.0,
        };

        let tour_cost = (touring_costs.base_cost_per_show as f32 * country_travel_mult) as i32;

        if !self.player.can_afford(tour_cost) {
            return Err(format!("You need at least ${} to finance this tour!", tour_cost));
        }

        let (tour_weeks, fame_gain) = if self.band.fame >= 80 {
            (4, 10)
        } else if self.band.fame >= 60 {
            (3, 6)
        } else if self.band.fame >= 35 {
            (2, 4)
        } else {
            (2, 3)
        };

        let regional_fame_key = format!("{}:{}", country_key, region_key);
        let regional_fame = *self.regional_fame.get(&regional_fame_key).unwrap_or(&0);

        let audience = (self.band.fame as f32 / 3.0) + (regional_fame as f32);
        let base_gross = (*population as f32).sqrt() * (*economic_strength as f32 / 100.0) * audience * 0.06;

        let era_modifier = self.timeline.get_gig_pay_modifier();
        let market_modifier = self.world.get_market_modifier();
        let final_earnings = (base_gross * era_modifier * market_modifier) as i32;

        self.player.spend_money(tour_cost);
        self.player.earn_money(final_earnings as u32);
        self.player.energy -= 40;
        self.player.stress = (self.player.stress + 30).min(constants::MAX_STRESS);
        self.band.fame = (self.band.fame + fame_gain).min(constants::MAX_FAME);
        
        let mut rng = rand::thread_rng();
        let regional_fame_gain = 10 + rng.gen_range(0..=5);
        let new_regional_fame = (regional_fame as u16 + regional_fame_gain as u16).min(100) as u8;
        self.regional_fame.insert(regional_fame_key.clone(), new_regional_fame);

        self.week += tour_weeks;
        self.log(format!(
            "🚌 Tour of {} ({}): grossed ${} against ${} in costs, fame +{}, regional fame {}% (+{}).",
            region_name, country_key.replace("_", " "), final_earnings, tour_cost, fame_gain, new_regional_fame, regional_fame_gain
        ));

        if rng.gen_bool(0.3) {
            self.band.fame = (self.band.fame + 2).min(constants::MAX_FAME);
            self.log("🗣️ Word of your live show spreads — extra fame on the way home.");
        } else if rng.gen_bool(0.15) {
            self.player.health = self.player.health.saturating_sub(10);
            self.log("🤒 The road took its toll — you came home run down.");
        }
        Ok(())
    }

    fn action_take_break(&mut self) -> Result<(), String> {
        self.player.energy = constants::MAX_ENERGY;
        self.player.stress = 0;
        self.player.health = (self.player.health + 10).min(constants::MAX_HEALTH);
        self.log("🏖️ A proper week off — fully recharged.");
        Ok(())
    }

    fn action_visit_doctor(&mut self) -> Result<(), String> {
        if !self.player.can_afford(constants::DOCTOR_VISIT_COST) {
            return Err(format!("You need ${} to visit the doctor!", constants::DOCTOR_VISIT_COST));
        }
        self.player.spend_money(constants::DOCTOR_VISIT_COST);
        self.player.health = (self.player.health + 20).min(constants::MAX_HEALTH);
        self.log(format!("🩺 The doctor patched you up (+20 health, -${}).", constants::DOCTOR_VISIT_COST));
        Ok(())
    }

    fn action_accept_deal(&mut self, offer_index: usize) -> Result<(), String> {
        if offer_index >= self.pending_deal_offers.len() {
            return Err("Invalid deal offer selected.".to_string());
        }
        let offer = self.pending_deal_offers.remove(offer_index);
        let label_name = offer.label_name.clone();
        let advance = offer.advance;
        let albums_required = offer.albums_required;
        let new_deal = band::RecordDeal {
            label_name: offer.label_name,
            label_tier: offer.label_tier,
            advance: offer.advance,
            royalty_rate: offer.royalty_rate,
            albums_required: offer.albums_required,
            albums_delivered: 0,
            market_reach: offer.original_label_data.market_reach,
        };
        self.band.sign_deal(new_deal);
        self.player.earn_money(advance);
        self.pending_deal_offers.clear();
        self.log(format!(
            "✍️ Signed with {}! ${} advance in the bank — you owe them {} album{}.",
            label_name,
            advance,
            albums_required,
            if albums_required == 1 { "" } else { "s" }
        ));
        Ok(())
    }

    fn action_reject_deal(&mut self, offer_index: usize) -> Result<(), String> {
        if offer_index >= self.pending_deal_offers.len() {
            return Err("Invalid deal offer selected.".to_string());
        }
        let offer = self.pending_deal_offers.remove(offer_index);
        self.log(format!("🚫 Turned down {}'s offer.", offer.label_name));
        
        let mut rng = rand::thread_rng();
        if let Some(poaching_band) = self.world.poach_rejected_deal(&offer.label_name, &mut rng) {
            self.log(format!("📰 NEWS: {} signed with {} after you turned them down!", poaching_band, offer.label_name));
        }
        Ok(())
    }

    fn action_accept_support_tour(&mut self) -> Result<(), String> {
        let Some(offer) = self.pending_support_offer.clone() else {
            return Err("Nobody has offered you a support slot.".to_string());
        };
        if self.player.energy < 30 {
            return Err("You're too exhausted to head out on the road!".to_string());
        }
        self.pending_support_offer = None;

        self.player.earn_money(offer.pay);
        self.player.energy = self.player.energy.saturating_sub(35);
        self.player.stress = (self.player.stress + 20).min(constants::MAX_STRESS);
        self.band.fame = (self.band.fame + offer.fame_gain).min(constants::MAX_FAME);
        self.week += offer.weeks;
        self.log(format!(
            "🎟️ Opened for {} for {} weeks — ${} and a taste of the big stage (fame +{}).",
            offer.host_band, offer.weeks, offer.pay, offer.fame_gain
        ));

        let mut rng = rand::thread_rng();
        if rng.gen_bool(0.25) {
            self.band.fame = (self.band.fame + 2).min(constants::MAX_FAME);
            self.log("🔥 Their crowd adopted you — encores every night (+2 fame).");
        }
        Ok(())
    }

    fn action_decline_support_tour(&mut self) -> Result<(), String> {
        let Some(offer) = self.pending_support_offer.take() else {
            return Err("Nobody has offered you a support slot.".to_string());
        };
        self.log(format!("🚫 Passed on {}'s support slot.", offer.host_band));
        Ok(())
    }

    /// Expire a stale support offer, or roll for a new one from a rival act
    /// famous enough to headline over you.
    fn update_support_tour_offer(&mut self) {
        if let Some(offer) = &self.pending_support_offer {
            if self.week >= offer.expires_week {
                let host = offer.host_band.clone();
                self.pending_support_offer = None;
                self.log(format!(
                    "🎟️ {}'s support slot went to another band — you sat on it too long.",
                    host
                ));
            }
            return;
        }

        if self.band.fame < SUPPORT_OFFER_MIN_FAME {
            return;
        }
        let mut rng = rand::thread_rng();
        if !rng.gen_bool(SUPPORT_OFFER_CHANCE) {
            return;
        }

        let candidates: Vec<(String, u8)> = self
            .world
            .bands
            .iter()
            .filter(|b| b.fame >= self.band.fame.saturating_add(SUPPORT_OFFER_FAME_GAP))
            .map(|b| (b.name.clone(), b.fame))
            .collect();
        if candidates.is_empty() {
            return;
        }
        let (host_band, host_fame) = candidates[rng.gen_range(0..candidates.len())].clone();

        let weeks = rng.gen_range(2..=4u32);
        let base_pay = weeks * (50 + u32::from(host_fame) * 5);
        let pay = (base_pay as f32 * self.timeline.get_gig_pay_modifier()) as u32;
        let gap = host_fame.saturating_sub(self.band.fame);
        let fame_gain = (2 + gap / 8).clamp(3, 12);

        self.pending_support_offer = Some(SupportTourOffer {
            host_band: host_band.clone(),
            host_fame,
            weeks,
            pay,
            fame_gain,
            expires_week: self.week + SUPPORT_OFFER_LIFETIME_WEEKS,
        });
        self.log(format!(
            "🎟️ {} want '{}' opening their {}-week tour — ${} and real exposure. Press T to respond.",
            host_band, self.band.name, weeks, pay
        ));
    }
    
    // --- Main execute_action ---
    fn execute_action(&mut self, action: GameAction) -> Result<(), String> {
        match action {
            GameAction::LazeAround => self.action_laze_around(),
            GameAction::WriteSongs => self.action_write_songs(),
            GameAction::Practice => self.action_practice(),
            GameAction::RecordSingle => self.action_record_single(),
            GameAction::RecordAlbum => self.action_record_album(),
            GameAction::Gig(venue_index) => self.action_play_gig(venue_index),
            GameAction::GoOnTour(region_index) => self.action_go_on_tour(region_index),
            GameAction::TakeBreak => self.action_take_break(),
            GameAction::VisitDoctor => self.action_visit_doctor(),
            GameAction::AcceptDeal(index) => self.action_accept_deal(index),
            GameAction::RejectDeal(index) => self.action_reject_deal(index),
            GameAction::AcceptSupportTour => self.action_accept_support_tour(),
            GameAction::DeclineSupportTour => self.action_decline_support_tour(),
            GameAction::StartMarketingCampaign(release_id, campaign_type) => self.action_start_marketing_campaign(release_id, campaign_type),
            GameAction::Quit => {
                self.game_over = true;
                Ok(())
            }
        }
    }
    
    // --- Turn Processing Helper Methods (Step 6) ---
    fn process_music_releases_and_marketing(&mut self) {
        let current_week = self.week; 

        let mut still_pending_release = Vec::new();
        for mut release in std::mem::take(&mut self.just_released_music) {
            if current_week >= release.week_released + INITIAL_SALES_WINDOW_WEEKS {
                let sales_score = self.calculate_release_sales_score(&release);
                release.initial_sales_score = sales_score;

                let income = self.calculate_income_from_sales_score(sales_score, &release.release_type);
                release.total_income_generated += income;
                self.player.earn_money(income);

                let verdict = match sales_score {
                    0..=99 => "flopped",
                    100..=299 => "sold modestly",
                    300..=599 => "sold well",
                    _ => "is a SMASH HIT",
                };
                // A label's distribution spreads your name further than a
                // self-pressed run ever could.
                let fame_gain = if self.band.current_deal().is_some() {
                    (sales_score / 250).min(4) as u8
                } else {
                    (sales_score / 400).min(2) as u8
                };
                self.band.fame = (self.band.fame + fame_gain).min(constants::MAX_FAME);
                if fame_gain > 0 {
                    self.log(format!(
                        "💿 '{}' {} — first-run earnings: ${}, fame +{}.",
                        release.name, verdict, income, fame_gain
                    ));
                } else {
                    self.log(format!(
                        "💿 '{}' {} — first-run earnings: ${}.",
                        release.name, verdict, income
                    ));
                }

                let release_genre = release.genre.clone();
                if release.release_type == music::ReleaseType::Album {
                    if self.band.current_deal().is_some() && self.band.fulfill_album_obligation() {
                        self.log("🤝 That album completes your record deal — you're a free agent again!");
                    }
                    self.band.albums_released.push(release);
                } else {
                    self.band.singles_released.push(release);
                }

                if sales_score > PLAYER_MARKET_IMPACT_THRESHOLD_SALES_SCORE {
                    if let Some(genre_to_boost) = release_genre {
                        *self.world.dynamic_genre_modifiers.entry(genre_to_boost).or_insert(1.0) += PLAYER_MARKET_IMPACT_GENRE_MOD_BONUS;
                    }
                    self.world.music_market.demand = (self.world.music_market.demand + PLAYER_MARKET_IMPACT_DEMAND_BONUS).min(100);
                }

            } else {
                still_pending_release.push(release);
            }
        }
        self.just_released_music = still_pending_release;

        // Deal terms are captured up front: the catalogue loop below holds
        // mutable borrows into self.band, so it cannot call &self methods.
        let income_per_point = if self.band.current_deal().is_some() {
            LABEL_INCOME_PER_SCORE_POINT
        } else {
            INDEPENDENT_INCOME_PER_SCORE_POINT
        };
        let royalty_rate = self.band.current_deal().map(|deal| deal.royalty_rate);
        let distribution = self.distribution_multiplier();
        let mut catalog_income_this_week: u32 = 0;

        for release_list in [&mut self.band.albums_released, &mut self.band.singles_released] {
            for release in release_list.iter_mut() {
                release.active_marketing.retain(|campaign| current_week < campaign.end_week);
                release.marketing_level_achieved = release.active_marketing.iter()
                    .map(|c| c.effectiveness_bonus as u32)
                    .sum::<u32>()
                    .min(100) as u8;

                if release.initial_sales_score > 0 && current_week > release.week_released + INITIAL_SALES_WINDOW_WEEKS {
                     let weeks_since_initial_window_end = current_week - (release.week_released + INITIAL_SALES_WINDOW_WEEKS -1);
                     let ongoing_sales_score_divisor = 1 + weeks_since_initial_window_end;
                     let ongoing_sales_score = release.initial_sales_score / ongoing_sales_score_divisor;

                     if ongoing_sales_score > 10 {
                        let gross_income =
                            (ongoing_sales_score as f32 * income_per_point as f32 * distribution) as u32;
                        let ongoing_income = match royalty_rate {
                            Some(rate) => (gross_income as f32 * rate) as u32,
                            None => gross_income,
                        } / 5;
                        release.total_income_generated += ongoing_income;
                        self.player.earn_money(ongoing_income);
                        catalog_income_this_week += ongoing_income;
                     }
                }
            }
        }

        if catalog_income_this_week > 0 {
            self.log(format!("💵 Catalog royalties trickle in: ${}.", catalog_income_this_week));
        }
    }

    fn advance_week_events(&mut self) -> Result<(), String> {
        // Sync the timeline with the current week. Tours can jump several weeks
        // at once, so catch up year by year instead of testing a single boundary.
        let expected_year =
            constants::STARTING_YEAR + (self.week.saturating_sub(1)) / constants::WEEKS_PER_YEAR;
        while self.timeline.get_current_year() < expected_year {
            self.timeline.advance_year();
            let year = self.timeline.get_current_year();
            let era_name = self.timeline.get_current_era().era_name.clone();
            self.log(format!("🗓️ It's now {} — the era of {}.", year, era_name));
        }

        if let Some(event) = self.events.try_trigger_event(self.week) {
            self.apply_random_event(event)?;
        }

        self.player.weekly_health_decay();

        // Derive a weekly StdRng using splitmix64 key derivation from world_seed + week
        let mut key = self.world_seed.wrapping_add(self.week as u64).wrapping_mul(0x9E3779B97F4A7C15);
        key = (key ^ (key >> 30)).wrapping_mul(0xBF58476D1CE4E5B8);
        key = (key ^ (key >> 27)).wrapping_mul(0x94D049BB133111EB);
        key ^= key >> 31;
        let mut wk_rng = StdRng::seed_from_u64(key);

        if let Some(historical_event) = self.timeline.take_historical_event(&mut wk_rng) {
            self.apply_historical_event(&historical_event)?;
            self.log(format!("📰 MUSIC NEWS: {}", historical_event));
        }

        let scene_news = self.world.update_week(&self.timeline, &self.data_files, &mut wk_rng);
        for item in scene_news {
            self.log(item);
        }

        self.update_support_tour_offer();
        Ok(())
    }

    fn check_and_generate_deal_offers(&mut self) {
        if self.pending_deal_offers.is_empty() && self.week.is_multiple_of(4) && self.band.record_deal.is_none() {
            let mut rng = rand::thread_rng();
            let new_offers = self.world.generate_deal_offers(&self.band, &self.data_files, &mut rng);
            if !new_offers.is_empty() {
                let n = new_offers.len();
                self.pending_deal_offers = new_offers;
                self.log(format!(
                    "📬 {} record label{} sent you an offer — press V to review.",
                    n,
                    if n == 1 { "" } else { "s" }
                ));
            }
        }
    }
    
    pub fn process_turn(&mut self, action: GameAction) -> Result<bool, String> {
        if self.game_over {
            return Ok(false);
        }

        let is_turn_consuming_action = !matches!(
            action,
            GameAction::AcceptDeal(_)
                | GameAction::RejectDeal(_)
                | GameAction::DeclineSupportTour
                | GameAction::StartMarketingCampaign(_, _)
                | GameAction::Quit
        );

        self.execute_action(action.clone())?; // Execute action first

        if is_turn_consuming_action {
            self.week += 1; // Advance week only for turn-consuming actions
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


    fn check_game_over(&mut self) {
        if self.player.health == 0 {
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
        if self.player.health == 0 {
            "You died from poor health!".to_string()
        } else if self.player.money < 0 && self.band.fame < 10 {
            "You went broke and nobody knows who you are!".to_string()
        } else if self.band.fame >= constants::ROCKSTAR_FAME_THRESHOLD
            && self.band.albums_released.len() >= constants::ROCKSTAR_ALBUM_THRESHOLD as usize
        {
            "Congratulations! You're now a ROCKSTAR!".to_string()
        } else if self.game_over {
            "You walked away from the rock life on your own terms.".to_string()
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
                    self.log("🍾 You partied with the wrong crowd — you're wired, but at what cost…");
                } else {
                    self.log("🚫 Someone offered you 'a little help' backstage. You passed.");
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
                            self.log(format!("🔧 Your amp blew mid-set — ${} in repairs.", repair_cost));
                        } else {
                            self.band.skill = self.band.skill.saturating_sub(5);
                            self.log("🔧 Your amp blew and you can't afford repairs — the band sounds rougher.");
                        }
                    }
                    1 => {
                        self.band.skill = (self.band.skill + 5).min(constants::MAX_SKILL);
                        self.log("🎸 A pawn-shop find! New gear tightens up your sound (+5 skill).");
                    }
                    _ => {
                        let loss = rng.gen_range(100..500);
                        if self.player.can_afford(loss) {
                            self.player.spend_money(loss);
                            self.log(format!("🚨 Gear stolen from the van — ${} to replace it.", loss));
                        } else {
                            self.player.money = 0;
                            self.log("🚨 Gear stolen from the van — it cleaned you out.");
                        }
                        self.band.skill = self.band.skill.saturating_sub(3);
                    }
                }
            }
            RandomEvent::BandMemberIssue => {
                if !self.band.members.is_empty() {
                    let member_idx = rng.gen_range(0..self.band.members.len());
                    let roll = rng.gen_range(0..4);
                    let develops_problem = roll == 1 && rng.gen_bool(0.3);
                    let demand = rng.gen_range(100..300);

                    let member = &mut self.band.members[member_idx];
                    let name = member.name.clone();
                    match roll {
                        0 => {
                            member.skill = (member.skill + 5).min(100);
                            member.loyalty = (member.loyalty + 10).min(100);
                            self.log(format!("🌟 {} has been woodshedding — sharper than ever.", name));
                        }
                        1 => {
                            member.loyalty = member.loyalty.saturating_sub(15);
                            if develops_problem {
                                member.drug_problem = true;
                                self.log(format!(
                                    "😠 {} is unhappy with the band's direction — and partying way too hard.",
                                    name
                                ));
                            } else {
                                self.log(format!("😠 {} is unhappy with the band's direction.", name));
                            }
                        }
                        2 => {
                            if member.loyalty < 30 {
                                member.loyalty = 0;
                                self.log(format!("🚪 {} is threatening to quit!", name));
                            }
                        }
                        _ => {
                            self.player.money -= demand;
                            self.log(format!("💸 {} demands a bigger cut — ${} to keep the peace.", name, demand));
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
                    self.log("📰 A glowing review in the music press — your profile rises.");
                }
                1 => {
                    self.band.fame = self.band.fame.saturating_sub(rng.gen_range(2..6));
                    self.band.reputation.media_presence =
                        self.band.reputation.media_presence.saturating_sub(8);
                    self.log("📰 A critic tears your latest show apart. Ouch.");
                }
                _ => {
                    self.band.fame = self.band.fame.saturating_sub(rng.gen_range(5..15));
                    self.player.stress = (self.player.stress + 20).min(constants::MAX_STRESS);
                    self.log("🔥 SCANDAL! The tabloids are all over you — fame takes a hit.");
                }
            },
            RandomEvent::HealthEvent => match rng.gen_range(0..3) {
                0 => {
                    self.player.health = self.player.health.saturating_sub(rng.gen_range(10..25));
                    self.player.energy = self.player.energy.saturating_sub(30);
                    self.log("🤒 You've caught something nasty — health and energy suffer.");
                }
                1 => {
                    self.player.health = self.player.health.saturating_sub(rng.gen_range(5..15));
                    self.band.skill = self.band.skill.saturating_sub(5);
                    self.log("🤕 Stage dive gone wrong — you're hurt, and rehearsals suffer.");
                }
                _ => {
                    self.player.stress =
                        (self.player.stress + rng.gen_range(15..30)).min(constants::MAX_STRESS);
                    self.player.energy = self.player.energy.saturating_sub(20);
                    self.log("😰 The pressure is getting to you — stress climbs.");
                }
            },
            RandomEvent::MoneyEvent => {
                match rng.gen_range(0..4) {
                    0 => {
                        let amount = rng.gen_range(200..1000);
                        self.player.earn_money(amount as u32);
                        self.log(format!("💰 Unexpected windfall: ${}!", amount));
                    }
                    1 => {
                        let amount = rng.gen_range(100..500);
                        if self.player.can_afford(amount) {
                            self.player.spend_money(amount);
                        } else {
                            self.player.money = 0;
                        }
                        self.log(format!("💸 A surprise bill lands on the doormat: ${}.", amount));
                    }
                    2 => {
                        // Simplified: Royalty for *all* past releases, not just current one.
                        let total_releases_count = self.band.albums_released.len() + self.band.singles_released.len();
                        let royalties = (total_releases_count as i32) * rng.gen_range(10..50);
                        self.player.earn_money(royalties as u32);
                        if royalties > 0 {
                            self.log(format!("💵 A royalty check arrives: ${}.", royalties));
                        }
                    }
                    _ => {
                        let cost = rng.gen_range(500..2000);
                        if self.player.can_afford(cost) {
                            self.player.spend_money(cost);
                        } else {
                            self.player.money = 0;
                        }
                        self.band.fame = self.band.fame.saturating_sub(5);
                        self.log(format!("⚖️ Legal trouble costs you ${} and some reputation.", cost));
                    }
                }
            }
            RandomEvent::IndustryEvent => match rng.gen_range(0..3) {
                0 if !self.band.has_record_deal() && self.band.fame > 30 => {
                    self.band.fame = (self.band.fame + 5).min(constants::MAX_FAME);
                    self.log("👀 A&R scouts were spotted at your show — industry buzz grows.");
                }
                1 if self.band.fame > 20 => {
                    let payment = rng.gen_range(500..2000);
                    self.player.earn_money(payment as u32);
                    self.band.fame = (self.band.fame + 3).min(constants::MAX_FAME);
                    self.log(format!("🎪 A festival slot opens up — ${} and more fans.", payment));
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

#[cfg(test)]
mod tests {
    use super::*;

    fn test_game() -> Game {
        Game::new().expect("data files present")
    }

    fn test_deal(market_reach: u8, royalty_rate: f32) -> band::RecordDeal {
        band::RecordDeal {
            label_name: "Test Records".to_string(),
            label_tier: "Major".to_string(),
            advance: 0,
            royalty_rate,
            albums_required: 2,
            albums_delivered: 0,
            market_reach,
        }
    }

    #[test]
    fn unknown_indie_acts_reach_almost_nobody() {
        let mut game = test_game();
        game.band.record_deal = None;

        game.band.fame = 5;
        let unknown = game.calculate_income_from_sales_score(300, &ReleaseType::Single);
        game.band.fame = 95;
        let famous = game.calculate_income_from_sales_score(300, &ReleaseType::Single);

        assert!(
            famous > unknown * 3,
            "a famous indie act should reach a far larger audience: {unknown} vs {famous}"
        );
    }

    #[test]
    fn label_out_earns_indie_at_low_fame_but_not_at_high_fame() {
        let mut game = test_game();
        game.band.fame = 10;

        game.band.record_deal = None;
        let indie_low = game.calculate_income_from_sales_score(300, &ReleaseType::Single);
        game.band.record_deal = Some(test_deal(90, 0.12));
        let label_low = game.calculate_income_from_sales_score(300, &ReleaseType::Single);
        assert!(
            label_low > indie_low,
            "an unknown band should earn more through a label: label {label_low} vs indie {indie_low}"
        );

        game.band.fame = 95;
        game.band.record_deal = None;
        let indie_high = game.calculate_income_from_sales_score(300, &ReleaseType::Single);
        assert!(
            indie_high > label_low * 2,
            "a superstar keeping everything should out-earn a royalty slice: indie {indie_high} vs label {label_low}"
        );
    }

    #[test]
    fn indie_distribution_cost_scales_with_fame_and_vanishes_when_signed() {
        let mut game = test_game();
        game.band.record_deal = None;

        game.band.fame = 0;
        let garage_band = game.independent_distribution_cost(&ReleaseType::Album);
        game.band.fame = 100;
        let superstar = game.independent_distribution_cost(&ReleaseType::Album);

        assert!(garage_band > 0);
        assert!(
            superstar >= garage_band * 5,
            "a famous act presses a much bigger run: {garage_band} vs {superstar}"
        );

        game.band.record_deal = Some(test_deal(70, 0.10));
        assert_eq!(
            game.independent_distribution_cost(&ReleaseType::Album),
            0,
            "the label handles distribution when signed"
        );
    }

    #[test]
    fn failed_recording_does_not_eat_songs() {
        let mut game = test_game();
        game.band.unreleased_songs.push(music::Song {
            id: 0,
            name: "Keeper".to_string(),
            songwriting_quality: 50,
        });
        game.player.money = 0;

        assert!(game.action_record_single().is_err());
        assert_eq!(
            game.band.unreleased_songs.len(),
            1,
            "songs must survive a recording attempt the player cannot afford"
        );
    }

    #[test]
    fn accepting_a_support_tour_pays_and_advances_time() {
        let mut game = test_game();
        game.band.fame = 10;
        game.player.money = 500;
        game.player.energy = 100;
        game.pending_support_offer = Some(SupportTourOffer {
            host_band: "Big Stars".to_string(),
            host_fame: 60,
            weeks: 3,
            pay: 1000,
            fame_gain: 6,
            expires_week: 10,
        });
        let week_before = game.week;

        game.action_accept_support_tour().expect("offer should be acceptable");

        assert!(game.pending_support_offer.is_none());
        assert_eq!(game.player.money, 1500);
        assert_eq!(game.week, week_before + 3);
        assert!(game.band.fame >= 16, "fame should include the offered gain");
        assert_eq!(game.player.energy, 65);
    }

    #[test]
    fn declining_a_support_tour_clears_it() {
        let mut game = test_game();
        game.pending_support_offer = Some(SupportTourOffer {
            host_band: "Big Stars".to_string(),
            host_fame: 60,
            weeks: 2,
            pay: 500,
            fame_gain: 4,
            expires_week: 10,
        });

        game.action_decline_support_tour().expect("decline should succeed");
        assert!(game.pending_support_offer.is_none());
        assert!(game.action_decline_support_tour().is_err(), "no offer left to decline");
    }

    #[test]
    fn support_offers_arrive_when_bigger_acts_exist() {
        let mut game = test_game();
        game.band.fame = 20;
        // Guarantee at least one act big enough to headline over the player.
        game.world.bands[0].fame = 80;

        let mut offered = false;
        for week in 1..=200 {
            game.week = week;
            game.update_support_tour_offer();
            if game.pending_support_offer.is_some() {
                offered = true;
                break;
            }
        }
        assert!(offered, "200 weeks alongside a big act should produce at least one offer");

        let offer = game.pending_support_offer.as_ref().unwrap();
        assert!(offer.host_fame >= game.band.fame + SUPPORT_OFFER_FAME_GAP);
        assert!(offer.pay > 0);
    }

    #[test]
    fn stale_support_offers_expire() {
        let mut game = test_game();
        game.pending_support_offer = Some(SupportTourOffer {
            host_band: "Big Stars".to_string(),
            host_fame: 60,
            weeks: 2,
            pay: 500,
            fame_gain: 4,
            expires_week: 3,
        });
        game.week = 5;

        game.update_support_tour_offer();
        assert!(game.pending_support_offer.is_none(), "offers should expire");
        assert!(
            game.turn_log.iter().any(|m| m.contains("went to another band")),
            "expiry should be reported"
        );
    }
}
