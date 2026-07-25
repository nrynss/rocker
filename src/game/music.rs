use crate::game::constants::{
    DISTRIBUTION_CHANNEL_FAME_GATE, DISTRIBUTION_CHANNEL_FEE,
    DISTRIBUTION_CHANNEL_INTERNATIONAL_FLOOR, DISTRIBUTION_CHANNEL_REACH_FLOOR,
};
use crate::game::genre::MusicGenre;
use crate::game::studios::RecordingStudio;
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
        // L-SCALE (v0.8 §B): campaign `cost` values ×3 to the one honest
        // era-real money scale (was 100 / 300 / 500 / 750 / 2000), in lockstep
        // with the ×3 record income a self-releasing act markets against.
        // `duration_weeks` and `effectiveness_bonus` are not dollars — unchanged.
        match self {
            MarketingCampaignType::BasicPress => CampaignSpec {
                name: "Basic Press",
                cost: 300,
                duration_weeks: 4,
                effectiveness_bonus: 5,
            },
            MarketingCampaignType::MagazineSpread => CampaignSpec {
                name: "Magazine Spread",
                cost: 900,
                duration_weeks: 4,
                effectiveness_bonus: 10,
            },
            MarketingCampaignType::RadioPromotion => CampaignSpec {
                name: "Radio Promotion",
                cost: 1500,
                duration_weeks: 6,
                effectiveness_bonus: 15,
            },
            MarketingCampaignType::SocialMediaBlitz => CampaignSpec {
                name: "Street Team Blitz",
                cost: 2250,
                duration_weeks: 4,
                effectiveness_bonus: 20,
            },
            MarketingCampaignType::MusicVideo => CampaignSpec {
                name: "Promo Film",
                cost: 6000,
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

    /// How far this channel reaches into the three foreign sales territories
    /// on distribution alone, before touring (the indie half of international
    /// presence). `reach_floor` above governs how well the act sells where it
    /// is present; this governs whether it is present abroad at all. Touring's
    /// regional fame adds on top (`economy::territory_presence`).
    pub fn international_floor(self) -> f32 {
        DISTRIBUTION_CHANNEL_INTERNATIONAL_FLOOR[self.ordinal()]
    }
}

/// A finished record the band owns and has not yet put out (design §D).
///
/// Recording is not pressing. A session (design §B) yields one of these into
/// `Band::masters`; a separate Release action buys the pressing run and the
/// distribution channel and turns it into a `Release`. **Sitting on a master
/// is legal** and sometimes right — finish the record, tour behind the last
/// one, put this one out into a better market.
///
/// Everything the record *is* was decided in the studio, and all of it is
/// frozen here: the tracklisting, the format, the quality that came out of
/// the room, and which room it was. Nothing that happens between the last day
/// of the session and release day can change any of it — which is the point
/// of a master. What release day still decides is how many copies exist and
/// how far they travel, and those live on the `Release`, not here.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Master {
    /// The record's title, chosen in the studio — an album's from the title
    /// generator, a single's from its A-side.
    pub name: String,
    pub release_type: ReleaseType,
    /// What came out of the room: `calculate_release_quality` rolled once, at
    /// booking, against the band's condition then (design §D).
    pub release_quality: u8,
    /// Average songwriting quality of the songs actually cut.
    pub songs_involved_quality_avg: u8,
    /// The tracklisting, taken out of the notebook when the session was booked.
    pub songs: Vec<Song>,
    /// The room it was cut in (design §C) — kept for the vault to name.
    pub studio: RecordingStudio,
    /// The week the session's last week resolved.
    pub week_finished: u32,
    /// The band's genre at the time of recording, stamped onto the `Release`.
    pub genre: Option<MusicGenre>,
}

/// What it costs to put a master out (design §D): the pressing run and the
/// distribution channel, and nothing else — the studio was paid for weeks
/// ago, at booking. Produced by `Game::quote_release`, which the vault UI
/// quotes from and `action_release_master` charges from, so the two can never
/// disagree — the same single-source-of-truth shape `RecordingQuote` has for
/// a booking and `TourQuote` has for a tour.
#[derive(Debug, Clone, Copy)]
pub struct ReleaseQuote {
    /// Copies that will exist: the chosen tier's run, or the label's own.
    pub copies: u32,
    /// `0` when signed — the label presses on its own dime, as it always has.
    pub pressing_cost: i32,
    /// `0` when signed — a label deal ignores indie channels entirely.
    pub distribution_fee: i32,
    /// The channel the record would go out under, frozen onto the `Release`.
    pub channel: DistributionChannel,
}

impl ReleaseQuote {
    /// The whole bill due on release day.
    pub fn total(&self) -> i32 {
        self.pressing_cost + self.distribution_fee
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
