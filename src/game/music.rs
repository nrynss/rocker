use crate::game::constants::{
    DISTRIBUTION_CHANNEL_FAME_GATE, DISTRIBUTION_CHANNEL_FEE, DISTRIBUTION_CHANNEL_REACH_FLOOR,
};
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

/// Indie distribution channels (design §E-3, M6): releasing while unsigned
/// buys reach through one of these, fee due at release. A label deal
/// ignores channels entirely — its own `market_reach` always wins
/// (`economy::distribution_multiplier`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum DistributionChannel {
    MailOrder,
    Regional,
    National,
}

impl Default for DistributionChannel {
    /// Mail order & gigs — the pre-M6 indie formula with no purchasable
    /// floor, so an old save (or a signed release, which never sets this)
    /// reads exactly as it always did.
    fn default() -> Self {
        DistributionChannel::MailOrder
    }
}

impl DistributionChannel {
    /// In picker order, smallest reach to biggest.
    pub const ALL: [DistributionChannel; 3] = [
        DistributionChannel::MailOrder,
        DistributionChannel::Regional,
        DistributionChannel::National,
    ];

    /// Index into the `DISTRIBUTION_CHANNEL_*` const tables — also this
    /// channel's position in `ALL`, for picker navigation.
    pub fn ordinal(self) -> usize {
        match self {
            DistributionChannel::MailOrder => 0,
            DistributionChannel::Regional => 1,
            DistributionChannel::National => 2,
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            DistributionChannel::MailOrder => "Mail order & gigs",
            DistributionChannel::Regional => "Regional distributor",
            DistributionChannel::National => "National distributor",
        }
    }

    /// Fame required to select this channel at all (design §E-3 table).
    pub fn fame_gate(self) -> u8 {
        DISTRIBUTION_CHANNEL_FAME_GATE[self.ordinal()]
    }

    pub fn is_available(self, fame: u8) -> bool {
        fame >= self.fame_gate()
    }

    /// Fee due at each release under this channel (design §E-3 table).
    pub fn fee(self) -> i32 {
        DISTRIBUTION_CHANNEL_FEE[self.ordinal()]
    }

    /// Reach floor: effective indie reach is `max(floor, current fame
    /// formula)` (design §E-3 table).
    pub fn reach_floor(self) -> f32 {
        DISTRIBUTION_CHANNEL_REACH_FLOOR[self.ordinal()]
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
    /// Best chart position this release ever reached (1 = #1); `None` means it
    /// never charted. A release is a "hit" iff this is `Some` (fame gravity,
    /// design §C). Serde default `None` so pre-0.6 saves load.
    #[serde(default)]
    pub peak_chart_position: Option<u8>,
    /// Number of singles cut from this album by the label (design §C).
    /// Meaningful for albums; stays 0 on singles.
    #[serde(default)]
    pub singles_cut: u32,
    /// Certification level (§D): 0 = none, 1 = silver, 2 = gold, 3 = platinum, 4+ = multi-platinum count.
    /// Derived from cumulative copies_sold at certification thresholds.
    #[serde(default)]
    pub certified: u8,
    /// Indie distribution channel this release went out under (design §E-3,
    /// M6), frozen at record time. `None` for label releases (channel-blind;
    /// reach is `market_reach`) and for pre-M6 saves — both read as
    /// `DistributionChannel::default()` (Mail order & gigs) wherever reach is
    /// computed, so old catalog behaves exactly as before. Kept per-release
    /// (not a single "current" setting) so a later channel upgrade never
    /// retroactively changes an old release's tail sales.
    #[serde(default)]
    pub distribution_channel: Option<DistributionChannel>,
    /// The signing label's `market_reach`, frozen on the release at record
    /// time. `Some(reach)` for a label release, `None` for indie releases and
    /// pre-M6 saves. Reach is otherwise read from the band's *current* deal,
    /// so without this a signed record's catalog tail would collapse to indie
    /// reach the moment the act leaves the label — the record keeps the
    /// distribution footprint it was actually released with. `None` falls back
    /// to the live deal (so indie/legacy behaviour is unchanged), which is why
    /// this disambiguates label releases from legacy ones that also carry
    /// `distribution_channel: None`.
    #[serde(default)]
    pub label_market_reach: Option<u8>,
}
