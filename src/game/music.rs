use crate::game::genre::MusicGenre;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Song {
    pub id: u32,
    pub name: String,
    pub songwriting_quality: u8, // 0-100
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum ReleaseType {
    Single,
    Album,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum MarketingCampaignType {
    BasicPress,       // Low cost, low impact
    RadioPromotion,   // Medium cost, medium impact
    MusicVideo,       // High cost, high impact
    SocialMediaBlitz, // Modern era, variable cost/impact
    MagazineSpread,   // Older eras, medium cost/impact
}

pub struct CampaignSpec {
    pub name: &'static str,
    pub cost: i32,
    pub duration_weeks: u32,
    pub effectiveness_bonus: u8,
}

impl MarketingCampaignType {
    pub const ALL: [MarketingCampaignType; 5] = [
        MarketingCampaignType::BasicPress,
        MarketingCampaignType::MagazineSpread,
        MarketingCampaignType::RadioPromotion,
        MarketingCampaignType::SocialMediaBlitz,
        MarketingCampaignType::MusicVideo,
    ];

    pub fn spec(&self) -> CampaignSpec {
        match self {
            MarketingCampaignType::BasicPress => CampaignSpec {
                name: "Basic Press",
                cost: 100,
                duration_weeks: 4,
                effectiveness_bonus: 5,
            },
            MarketingCampaignType::MagazineSpread => CampaignSpec {
                name: "Magazine Spread",
                cost: 300,
                duration_weeks: 4,
                effectiveness_bonus: 10,
            },
            MarketingCampaignType::RadioPromotion => CampaignSpec {
                name: "Radio Promotion",
                cost: 500,
                duration_weeks: 6,
                effectiveness_bonus: 15,
            },
            MarketingCampaignType::SocialMediaBlitz => CampaignSpec {
                name: "Street Team Blitz",
                cost: 750,
                duration_weeks: 4,
                effectiveness_bonus: 20,
            },
            MarketingCampaignType::MusicVideo => CampaignSpec {
                name: "Promo Film",
                cost: 2000,
                duration_weeks: 8,
                effectiveness_bonus: 30,
            },
        }
    }
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
    pub week_released: u32,  // Game week when it becomes available for sales calculation
    pub songs_involved_quality_avg: u8, // Average songwriting_quality of songs in it
    pub active_marketing: Vec<ActiveMarketingCampaign>,
    pub marketing_level_achieved: u8, // 0-100, sum of effectiveness_bonus from active campaigns
    pub initial_sales_score: u32,     // Calculated after an initial sales window
    pub total_income_generated: u32,
    pub genre: Option<MusicGenre>,
    /// How many copies exist. Sales can never exceed this; 0 means uncapped
    /// (legacy saves from before pressing runs existed).
    #[serde(default)]
    pub copies_pressed: u32,
    /// Copies sold so far, across the first run and the catalog long tail.
    #[serde(default)]
    pub copies_sold: u32,
}
