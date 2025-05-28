use serde::{Deserialize, Serialize};
// MusicGenre will be referenced via super::world::MusicGenre
// as it's defined in world.rs

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Song {
    pub id: u32,
    pub name: String,
    pub songwriting_quality: u8, // 0-100
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum ReleaseType {
    Single,
    Album,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum MarketingCampaignType {
    BasicPress,      // Low cost, low impact
    RadioPromotion,  // Medium cost, medium impact
    MusicVideo,      // High cost, high impact
    SocialMediaBlitz, // Modern era, variable cost/impact
    MagazineSpread,   // Older eras, medium cost/impact
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActiveMarketingCampaign {
    pub campaign_type: MarketingCampaignType,
    pub start_week: u32,
    pub end_week: u32,
    pub effectiveness_bonus: u8, // e.g., 5-25 points to marketing_level_achieved
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Release {
    pub id: u32,
    pub name: String,
    pub release_type: ReleaseType,
    pub release_quality: u8, // 0-100, overall quality of the recording/production
    pub week_released: u32, // Game week when it becomes available for sales calculation
    pub songs_involved_quality_avg: u8, // Average songwriting_quality of songs in it
    pub active_marketing: Vec<ActiveMarketingCampaign>,
    pub marketing_level_achieved: u8, // 0-100, sum of effectiveness_bonus from active campaigns
    pub initial_sales_score: u32, // Calculated after an initial sales window
    pub total_income_generated: u32,
    pub genre: Option<super::world::MusicGenre>, // Assuming MusicGenre is in world.rs
}
