use crate::data_loader::{GameDataFiles, RecordLabel};
use crate::game::band::Band; // Added import for Band
use crate::game::timeline::MusicTimeline;
use rand::Rng;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PotentialDealOffer {
    pub label_name: String,
    pub label_tier: String,
    pub advance: u32,
    pub royalty_rate: f32,
    pub albums_required: u8,
    pub original_label_data: RecordLabel,
    /// The week the label withdraws the offer if it's ignored. `None`
    /// means it never expires — deliberately, because offers saved before
    /// expiry existed deserialize to `None`, and a bare numeric default
    /// (0) would kill every live offer the moment an old save loaded.
    #[serde(default)]
    pub expires_week: Option<u32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GameWorld {
    pub music_market: MusicMarket,
    /// Every act on the scene besides the player's. Not "rivals" — just the
    /// hundreds of bands a real scene is made of.
    #[serde(alias = "competing_bands")]
    pub bands: Vec<SceneBand>,
    pub venues: Vec<Venue>,
    pub current_trends: MusicTrend,
    #[serde(default)]
    pub dynamic_genre_modifiers: std::collections::HashMap<MusicGenre, f32>,
    #[serde(default)]
    pub charts: Vec<ChartEntry>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MusicMarket {
    pub demand: u8,     // 0-100, affects earnings
    pub saturation: u8, // 0-100, affects difficulty
    pub economic_state: EconomicState,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum EconomicState {
    Recession,
    Stagnant,
    Growing,
    Booming,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SceneBand {
    pub name: String,
    pub fame: u8,
    #[serde(default)]
    pub peak_fame: u8,
    pub latest_release: String,
    pub genre: MusicGenre,
    /// Which label the band records for, if any.
    #[serde(default)]
    pub label: Option<String>,
    /// Career trajectory: hits build it, flops and neglect erode it.
    #[serde(default)]
    pub momentum: i8,
}

/// One record on the weekly top-10.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChartEntry {
    pub title: String,
    pub band_name: String,
    pub is_player: bool,
    pub score: u32,
    pub weeks_on_chart: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Venue {
    pub name: String,
    pub capacity: u32,
    pub prestige: u8, // 0-100
    pub base_payment: u32,
    pub location: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum MusicTrend {
    Rock,
    Pop,
    Metal,
    Punk,
    Alternative,
    Electronic,
}

// Ensure MusicGenre can be used as a HashMap key
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum MusicGenre {
    #[default]
    Rock,
    Pop,
    Metal,
    Punk,
    Alternative,
    Electronic,
    Folk,
    Jazz,
}

impl MusicGenre {
    pub const ALL: [MusicGenre; 8] = [
        MusicGenre::Rock,
        MusicGenre::Pop,
        MusicGenre::Metal,
        MusicGenre::Punk,
        MusicGenre::Alternative,
        MusicGenre::Electronic,
        MusicGenre::Folk,
        MusicGenre::Jazz,
    ];

    pub fn name(&self) -> &'static str {
        match self {
            MusicGenre::Rock => "Rock",
            MusicGenre::Pop => "Pop",
            MusicGenre::Metal => "Metal",
            MusicGenre::Punk => "Punk",
            MusicGenre::Alternative => "Alternative",
            MusicGenre::Electronic => "Electronic",
            MusicGenre::Folk => "Folk",
            MusicGenre::Jazz => "Jazz",
        }
    }

    /// Keys this genre answers to in markets.json's genre_era_modifiers.
    pub fn aliases(&self) -> &'static [&'static str] {
        match self {
            MusicGenre::Rock => &["rock", "arena_rock"],
            MusicGenre::Pop => &["pop", "synth_pop", "brit_pop", "disco"],
            MusicGenre::Metal => &["metal", "hair_metal", "pop_metal"],
            MusicGenre::Punk => &["punk", "post_punk"],
            MusicGenre::Alternative => &["alternative", "grunge", "new_wave"],
            MusicGenre::Electronic => &["synth_pop", "house", "new_wave", "disco"],
            MusicGenre::Folk => &["folk"],
            MusicGenre::Jazz => &["jazz", "blues"],
        }
    }

    fn random(rng: &mut impl Rng) -> Self {
        MusicGenre::ALL[rng.gen_range(0..MusicGenre::ALL.len())].clone()
    }

    /// A genre that fits the current trends, if any does.
    fn random_trending(timeline: &MusicTimeline, rng: &mut impl Rng) -> Self {
        let trending = timeline.get_trending_genres();
        let matching: Vec<MusicGenre> = MusicGenre::ALL
            .iter()
            .filter(|genre| trending.iter().any(|t| t.contains(genre.name())))
            .cloned()
            .collect();
        if matching.is_empty() {
            Self::random(rng)
        } else {
            matching[rng.gen_range(0..matching.len())].clone()
        }
    }
}

/// The scene never empties out or grows without bound.
pub const SCENE_START_BANDS: usize = 180;
pub const SCENE_MIN_BANDS: usize = 120;
pub const SCENE_MAX_BANDS: usize = 260;
pub const CHART_SIZE: usize = 10;
const CHART_DECAY: f32 = 0.85;
const CHART_FLOOR_SCORE: u32 = 25;

// Deal-offer buzz: the scale the tier thresholds in record_labels.json
// measure (independent 10, boutique 20, major 30). Fame carries most of it;
// records prove you can deliver; a charting record adds short-lived heat.
const BUZZ_PER_SINGLE: u32 = 3;
const BUZZ_PER_ALBUM: u32 = 4;
const BUZZ_CATALOG_CAP: u32 = 10;
const BUZZ_CHART_BONUS: u32 = 2;

impl GameWorld {
    pub fn new(data_files: &GameDataFiles, rng: &mut impl Rng) -> Self {
        Self {
            music_market: MusicMarket {
                demand: 50,
                saturation: 55,
                economic_state: EconomicState::Growing,
            },
            bands: Self::generate_scene(data_files, rng),
            venues: Self::generate_venues(data_files, rng),
            current_trends: MusicTrend::Rock,
            dynamic_genre_modifiers: std::collections::HashMap::new(),
            charts: Vec::new(),
        }
    }

    /// Advance the world by one week. Returns newsworthy events.
    pub fn update_week(
        &mut self,
        timeline: &MusicTimeline,
        data_files: &GameDataFiles,
        rng: &mut impl Rng,
    ) -> Vec<String> {
        let mut news = Vec::new();

        self.update_market_with_timeline(rng, timeline, &mut news);
        self.decay_charts(&mut news);
        self.update_scene_bands(rng, timeline, data_files, &mut news);
        self.update_scene_population(rng, timeline, data_files, &mut news);
        self.update_trends_with_timeline(timeline, rng);

        // Decay dynamic genre modifiers
        let mut new_modifiers = std::collections::HashMap::new();
        for (genre, val) in self.dynamic_genre_modifiers.iter() {
            let decayed_val = (*val - 1.0) * 0.95 + 1.0;
            // Only keep it if it's significantly different from 1.0
            if (decayed_val - 1.0).abs() > 0.01 {
                new_modifiers.insert(genre.clone(), decayed_val);
            }
        }
        self.dynamic_genre_modifiers = new_modifiers;

        news
    }

    fn update_market_with_timeline(
        &mut self,
        rng: &mut impl Rng,
        timeline: &MusicTimeline,
        news: &mut Vec<String>,
    ) {
        let era = timeline.get_current_era();

        // Adjust market demand towards historical era demand
        let target_demand = era.market_conditions.overall_demand;
        if self.music_market.demand < target_demand {
            self.music_market.demand = (self.music_market.demand + 2).min(target_demand);
        } else if self.music_market.demand > target_demand {
            self.music_market.demand =
                (self.music_market.demand.saturating_sub(2)).max(target_demand);
        }

        // Update economic state based on era and some randomness
        if rng.gen_range(0..10) == 0 {
            let new_state = match era.market_conditions.record_sales_growth {
                x if x > 20.0 => EconomicState::Booming,
                x if x > 10.0 => EconomicState::Growing,
                x if x > 0.0 => EconomicState::Stagnant,
                _ => EconomicState::Recession,
            };
            if new_state != self.music_market.economic_state {
                news.push(match new_state {
                    EconomicState::Booming => {
                        "📈 Record sales are BOOMING — everyone's buying.".to_string()
                    }
                    EconomicState::Growing => {
                        "📈 The record industry is growing again.".to_string()
                    }
                    EconomicState::Stagnant => "📊 Industry sales have flattened out.".to_string(),
                    EconomicState::Recession => {
                        "📉 The industry has slid into a slump — money is tight.".to_string()
                    }
                });
                self.music_market.economic_state = new_state;
            }
        }

        // Saturation tracks how crowded the scene is; innovation opens space.
        let target_saturation = ((25 + self.bands.len() / 4) as u8).min(90);
        if self.music_market.saturation < target_saturation {
            self.music_market.saturation += 1;
        } else if self.music_market.saturation > target_saturation {
            self.music_market.saturation -= 1;
        }
        if era.market_conditions.innovation_openness > 80 {
            self.music_market.saturation = self.music_market.saturation.saturating_sub(1);
        }
    }

    fn update_trends_with_timeline(&mut self, timeline: &MusicTimeline, rng: &mut impl Rng) {
        let trending_genres = timeline.get_trending_genres();
        if !trending_genres.is_empty() && rng.gen_range(0..10) == 0 {
            // 10% chance to align with historical trends
            self.current_trends = match trending_genres[0].as_str() {
                "Rock" => MusicTrend::Rock,
                "Pop" => MusicTrend::Pop,
                "Metal" | "Hard Rock" | "Hair Metal" => MusicTrend::Metal,
                "Punk" | "Post-Punk" => MusicTrend::Punk,
                "Alternative" | "Grunge" | "New Wave" => MusicTrend::Alternative,
                _ => MusicTrend::Electronic,
            };
        }
    }

    /// Every band on the scene lives a little each week: fame drifts with
    /// momentum and the era's trends, records get released and chart (or
    /// don't), and rising unsigned acts get signed.
    fn update_scene_bands(
        &mut self,
        rng: &mut impl Rng,
        timeline: &MusicTimeline,
        data_files: &GameDataFiles,
        news: &mut Vec<String>,
    ) {
        let era_year = timeline.get_current_era().year;
        let trending = timeline.get_trending_genres();
        let mut releases: Vec<(usize, String, u32)> = Vec::new();

        for (idx, band) in self.bands.iter_mut().enumerate() {
            let on_trend = trending.iter().any(|t| t.contains(band.genre.name()));

            // Fame drift: momentum plus the era's pull.
            if rng.gen_range(0..4) == 0 {
                let delta =
                    band.momentum as i16 + if on_trend { 1 } else { 0 } + rng.gen_range(-1..=1) - 1; // slight gravity: staying famous takes work
                band.fame = (band.fame as i16 + delta).clamp(0, 100) as u8;
            }
            // Momentum cools toward zero.
            if band.momentum != 0 && rng.gen_range(0..6) == 0 {
                band.momentum -= band.momentum.signum();
            }
            band.peak_fame = band.peak_fame.max(band.fame);

            // Releases: signed bands put records out more often.
            let release_odds = if band.label.is_some() { 16 } else { 28 };
            if rng.gen_range(0..release_odds) == 0 {
                let title = data_files.generate_song_title(rng);
                let quality = rng.gen_range(25..=85) as f32;
                let genre_mod = data_files.era_genre_modifier(era_year, band.genre.aliases());
                let reach = if band.label.is_some() {
                    1.35
                } else {
                    0.8 + f32::from(band.fame) / 250.0
                };
                let score = ((f32::from(band.fame) * 1.2 + quality * 2.5)
                    * genre_mod
                    * reach
                    * rng.gen_range(0.7..1.4)) as u32;

                band.latest_release = title.clone();
                releases.push((idx, title, score));
            }

            // Signings: a rising unsigned act catches a label's ear.
            if band.label.is_none() && band.fame >= 25 && rng.gen_bool(0.02) {
                let label = Self::random_label_for_fame(data_files, band.fame, rng);
                if band.fame >= 45 {
                    news.push(format!("🖋️ {} sign with {}.", band.name, label));
                }
                band.label = Some(label);
                band.momentum = (band.momentum + 1).min(3);
            }
        }

        // Chart submissions happen after the borrow on bands ends.
        for (idx, title, score) in releases {
            let band_name = self.bands[idx].name.clone();
            let position = self.submit_chart_entry(title.clone(), band_name.clone(), false, score);
            if let Some(pos) = position {
                let band = &mut self.bands[idx];
                band.fame = (band.fame + ((11 - pos as u8) / 2).max(1)).min(100);
                band.momentum = (band.momentum + 1).min(3);
                band.peak_fame = band.peak_fame.max(band.fame);
                // A charting record crowds the market a little.
                self.music_market.saturation = (self.music_market.saturation + 1).min(95);
                if pos <= 5 || self.bands[idx].fame >= 60 {
                    news.push(format!(
                        "📀 {}'s '{}' charts at #{}.",
                        band_name, title, pos
                    ));
                }
            }
        }
    }

    /// Bands break up and new ones form: the scene has a life of its own.
    fn update_scene_population(
        &mut self,
        rng: &mut impl Rng,
        timeline: &MusicTimeline,
        data_files: &GameDataFiles,
        news: &mut Vec<String>,
    ) {
        // Struggling bands call it quits; only once-notable splits make news.
        let mut notable_splits = Vec::new();
        self.bands.retain(|band| {
            if band.fame < 8 && band.momentum <= 0 && rng.gen_bool(0.05) {
                if band.peak_fame >= 40 {
                    notable_splits.push(band.name.clone());
                }
                false
            } else {
                true
            }
        });
        for name in notable_splits {
            news.push(format!(
                "💔 {} — once a fixture of the scene — have called it quits.",
                name
            ));
        }

        // New blood: refill hard when thin, trickle otherwise.
        let mut newcomers = if self.bands.len() < SCENE_MIN_BANDS {
            SCENE_MIN_BANDS - self.bands.len()
        } else if self.bands.len() < SCENE_MAX_BANDS {
            rng.gen_range(0..=3)
        } else {
            0
        };
        newcomers = newcomers.min(SCENE_MAX_BANDS - self.bands.len());

        for _ in 0..newcomers {
            let name = self.unique_band_name(data_files, rng);
            // Newcomers mostly chase whatever is currently hot.
            let genre = if rng.gen_bool(0.4) {
                MusicGenre::random_trending(timeline, rng)
            } else {
                MusicGenre::random(rng)
            };
            let hyped = rng.gen_bool(0.03);
            let fame = if hyped {
                rng.gen_range(25..40)
            } else {
                rng.gen_range(3..=22)
            };
            if hyped {
                news.push(format!(
                    "🌱 {} arrive on the scene with serious buzz.",
                    name
                ));
            }
            self.bands.push(SceneBand {
                name,
                fame,
                peak_fame: fame,
                latest_release: data_files.generate_song_title(rng),
                genre,
                label: None,
                momentum: 1,
            });
        }
    }

    /// Weekly chart churn: every record fades, and what falls below the
    /// floor (or off the bottom) is gone.
    fn decay_charts(&mut self, news: &mut Vec<String>) {
        for entry in &mut self.charts {
            entry.score = (entry.score as f32 * CHART_DECAY) as u32;
            entry.weeks_on_chart += 1;
        }
        let dropped: Vec<&ChartEntry> = self
            .charts
            .iter()
            .filter(|e| e.is_player && e.score < CHART_FLOOR_SCORE)
            .collect();
        for entry in dropped {
            news.push(format!(
                "📉 '{}' slips off the charts after {} week{}.",
                entry.title,
                entry.weeks_on_chart,
                if entry.weeks_on_chart == 1 { "" } else { "s" }
            ));
        }
        self.charts.retain(|e| e.score >= CHART_FLOOR_SCORE);
    }

    /// Submit a release to the charts. Returns its position (1-based) if it
    /// makes the top 10.
    pub fn submit_chart_entry(
        &mut self,
        title: String,
        band_name: String,
        is_player: bool,
        score: u32,
    ) -> Option<usize> {
        self.charts.push(ChartEntry {
            title: title.clone(),
            band_name,
            is_player,
            score,
            weeks_on_chart: 0,
        });
        self.charts.sort_by_key(|e| std::cmp::Reverse(e.score));
        let position = self
            .charts
            .iter()
            .position(|e| e.weeks_on_chart == 0 && e.title == title)
            .map(|i| i + 1);
        self.charts.truncate(CHART_SIZE);
        position.filter(|&p| p <= CHART_SIZE)
    }

    /// The biggest unsigned act may grab a deal the player turned down.
    pub fn poach_rejected_deal(&mut self, label_name: &str, rng: &mut impl Rng) -> Option<String> {
        if !rng.gen_bool(0.6) {
            return None;
        }
        let idx = self
            .bands
            .iter()
            .enumerate()
            .filter(|(_, b)| b.label.is_none() && b.fame >= 20)
            .max_by_key(|(_, b)| b.fame)
            .map(|(i, _)| i)?;
        let band = &mut self.bands[idx];
        band.label = Some(label_name.to_string());
        band.momentum = (band.momentum + 1).min(3);
        Some(band.name.clone())
    }

    fn random_label_for_fame(data_files: &GameDataFiles, fame: u8, rng: &mut impl Rng) -> String {
        let labels = &data_files.get_record_labels_data();
        let pool = if fame >= 60 {
            &labels.major_labels
        } else if fame >= 35 {
            &labels.independent_labels
        } else {
            &labels.boutique_labels
        };
        if pool.is_empty() {
            return "an unknown label".to_string();
        }
        pool[rng.gen_range(0..pool.len())].name.clone()
    }

    fn unique_band_name(&self, data_files: &GameDataFiles, rng: &mut impl Rng) -> String {
        for _ in 0..10 {
            let candidate = data_files.generate_band_name(rng);
            if !self.bands.iter().any(|b| b.name == candidate) {
                return candidate;
            }
        }
        data_files.generate_band_name(rng)
    }

    fn generate_scene(data_files: &GameDataFiles, rng: &mut impl Rng) -> Vec<SceneBand> {
        let mut bands: Vec<SceneBand> = Vec::with_capacity(SCENE_START_BANDS);
        let mut taken = std::collections::HashSet::new();

        while bands.len() < SCENE_START_BANDS {
            let mut name = data_files.generate_band_name(rng);
            let mut tries = 0;
            while taken.contains(&name) && tries < 10 {
                name = data_files.generate_band_name(rng);
                tries += 1;
            }
            if taken.contains(&name) {
                continue;
            }
            taken.insert(name.clone());

            // A realistic pyramid: most bands are nobodies, a few are stars.
            let roll = rng.gen_range(0..100);
            let fame: u8 = if roll < 60 {
                rng.gen_range(3..18)
            } else if roll < 85 {
                rng.gen_range(18..45)
            } else if roll < 97 {
                rng.gen_range(45..70)
            } else {
                rng.gen_range(70..93)
            };

            // Stars are usually signed already; mid-tier acts sometimes are.
            let signed = (fame >= 45 && rng.gen_bool(0.7)) || (fame >= 25 && rng.gen_bool(0.3));
            let label = if signed {
                Some(Self::random_label_for_fame(data_files, fame, rng))
            } else {
                None
            };

            bands.push(SceneBand {
                name,
                fame,
                peak_fame: fame,
                latest_release: data_files.generate_song_title(rng),
                genre: MusicGenre::random(rng),
                label,
                momentum: rng.gen_range(-1..=1),
            });
        }

        bands
    }

    fn generate_venues(data_files: &GameDataFiles, rng: &mut impl Rng) -> Vec<Venue> {
        let districts = [
            "Downtown",
            "City Center",
            "Industrial District",
            "Uptown",
            "Sports Complex",
        ];
        let capacities = [50, 200, 500, 2000, 20000];
        let prestiges = [10, 25, 40, 70, 95];
        let payments = [100, 300, 800, 3000, 15000];

        let mut venues = Vec::new();
        for i in 0..5 {
            venues.push(Venue {
                name: data_files.venue_names[rng.gen_range(0..data_files.venue_names.len())]
                    .clone(),
                capacity: capacities[i],
                prestige: prestiges[i],
                base_payment: payments[i],
                location: format!(
                    "{}, {}",
                    districts[i],
                    data_files.city_names[rng.gen_range(0..data_files.city_names.len())].clone()
                ),
            });
        }

        venues
    }

    pub fn get_market_modifier(&self) -> f32 {
        let demand_mod = self.music_market.demand as f32 / 100.0;
        let saturation_penalty = 1.0 - (self.music_market.saturation as f32 / 200.0);
        let economic_mod = match self.music_market.economic_state {
            EconomicState::Recession => 0.7,
            EconomicState::Stagnant => 0.9,
            EconomicState::Growing => 1.1,
            EconomicState::Booming => 1.3,
        };

        demand_mod * saturation_penalty * economic_mod
    }

    /// Industry buzz: how loudly the trade papers talk about an act. This is
    /// the scale the label tier thresholds in record_labels.json measure
    /// (independent 10, boutique 20, major 30).
    ///
    /// Intent: tiers unlock along a real career arc, not on fame alone.
    /// Fame is the backbone (3 buzz per 10 fame), the catalog widens each
    /// window (a single is 3, an album 4 — proof you deliver records), and
    /// a record on this week's chart adds a little heat while it lasts.
    /// The catalog contribution is capped so a deep back-catalog can pull
    /// an act one tier up, but never substitutes for genuine stardom.
    /// Where that lands, with the JSON thresholds also applying:
    /// independents around fame ~25 with a first record out, boutiques in
    /// the ~45-60 mid-career, majors only for genuinely big acts (fame
    /// ~65+ cold, ~60 while a record is charting).
    fn band_buzz(&self, band: &Band) -> u8 {
        let fame_heat = u32::from(band.fame) * 3 / 10;
        let catalog_heat = (band.singles_released.len() as u32 * BUZZ_PER_SINGLE
            + band.albums_released.len() as u32 * BUZZ_PER_ALBUM)
            .min(BUZZ_CATALOG_CAP);
        let chart_heat = if self.charts.iter().any(|entry| entry.is_player) {
            BUZZ_CHART_BONUS
        } else {
            0
        };
        (fame_heat + catalog_heat + chart_heat).min(100) as u8
    }

    pub fn generate_deal_offers(
        &self,
        band: &Band,
        game_data: &GameDataFiles,
        rng: &mut impl Rng,
    ) -> Vec<PotentialDealOffer> {
        let mut offers = Vec::new();
        let labels_data = game_data.get_record_labels_data();
        let buzz = self.band_buzz(band);

        let label_tiers = [
            (
                "Major",
                &labels_data.major_labels,
                &labels_data
                    .label_requirements
                    .major_label_interest_threshold,
            ),
            (
                "Independent",
                &labels_data.independent_labels,
                &labels_data
                    .label_requirements
                    .independent_label_interest_threshold,
            ),
            (
                "Boutique",
                &labels_data.boutique_labels,
                &labels_data
                    .label_requirements
                    .boutique_label_interest_threshold,
            ),
        ];

        for (tier_name, labels_in_tier, threshold) in &label_tiers {
            for label in *labels_in_tier {
                // The `singles` column reads as "records out, of any kind":
                // an album on the shelf opens doors at least as well as a
                // 45, so an act that went straight to albums isn't
                // invisible to every A&R desk in town.
                if band.fame >= threshold.fame
                    && band.albums_released.len() >= threshold.albums as usize
                    && band.total_releases() >= threshold.singles as usize
                    && buzz >= threshold.buzz
                {
                    // Check if already signed with this label
                    if let Some(current_deal) = band.current_deal()
                        && current_deal.label_name == label.name
                    {
                        continue; // Already signed with this label
                    }

                    // Random chance to make an offer
                    let offer_chance = match *tier_name {
                        "Major" => {
                            if band.fame > 70 {
                                0.30
                            } else if band.fame > 50 {
                                0.20
                            } else {
                                0.10
                            }
                        }
                        "Independent" => {
                            if band.fame > 40 {
                                0.40
                            } else if band.fame > 20 {
                                0.25
                            } else {
                                0.15
                            }
                        }
                        "Boutique" => {
                            if band.fame > 10 {
                                0.50
                            } else {
                                0.20
                            }
                        }
                        _ => 0.10,
                    };

                    if rng.gen_bool(offer_chance) {
                        let advance_percentage = match band.fame {
                            0..=20 => rng.gen_range(0.0..0.4), // Lower end for low fame
                            21..=50 => rng.gen_range(0.3..0.7),
                            51..=100 => rng.gen_range(0.6..1.0), // Higher end for high fame
                            _ => 0.5,
                        };
                        let advance_range_span = label.advance_range[1] - label.advance_range[0];
                        let calculated_advance = label.advance_range[0]
                            + (advance_range_span as f32 * advance_percentage) as u32;

                        let advance = calculated_advance
                            .clamp(label.advance_range[0], label.advance_range[1]);

                        let royalty_rate = label.royalty_rate as f32 / 100.0;

                        let albums_required = match *tier_name {
                            "Major" => rng.gen_range(2..=4),
                            "Independent" => rng.gen_range(1..=3),
                            "Boutique" => rng.gen_range(1..=2),
                            _ => 2,
                        };

                        offers.push(PotentialDealOffer {
                            label_name: label.name.clone(),
                            label_tier: tier_name.to_string(),
                            advance,
                            royalty_rate,
                            albums_required,
                            original_label_data: label.clone(),
                            // The world has no clock; the game stamps the
                            // deadline when the offer lands on the table.
                            expires_week: None,
                        });
                    }
                }
            }
        }
        offers
    }
}

impl std::fmt::Display for MusicTrend {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            MusicTrend::Rock => write!(f, "Rock"),
            MusicTrend::Pop => write!(f, "Pop"),
            MusicTrend::Metal => write!(f, "Metal"),
            MusicTrend::Punk => write!(f, "Punk"),
            MusicTrend::Alternative => write!(f, "Alternative"),
            MusicTrend::Electronic => write!(f, "Electronic"),
        }
    }
}

impl std::fmt::Display for EconomicState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            EconomicState::Recession => write!(f, "Recession"),
            EconomicState::Stagnant => write!(f, "Stagnant"),
            EconomicState::Growing => write!(f, "Growing"),
            EconomicState::Booming => write!(f, "Booming"),
        }
    }
}

impl std::fmt::Display for MusicGenre {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.name())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::game::timeline::MusicTimeline;
    use rand::SeedableRng;
    use rand::rngs::StdRng;

    #[test]
    fn worldgen_is_reproducible_per_seed() {
        let data = GameDataFiles::load().expect("data files present");
        let world_a = GameWorld::new(&data, &mut StdRng::seed_from_u64(99));
        let world_b = GameWorld::new(&data, &mut StdRng::seed_from_u64(99));
        let world_c = GameWorld::new(&data, &mut StdRng::seed_from_u64(100));

        let names = |w: &GameWorld| w.bands.iter().map(|b| b.name.clone()).collect::<Vec<_>>();
        assert_eq!(names(&world_a), names(&world_b), "same seed, same scene");
        assert_ne!(
            names(&world_a),
            names(&world_c),
            "different seed, different scene"
        );
        assert_eq!(world_a.bands.len(), SCENE_START_BANDS);
    }

    #[test]
    fn scene_has_hundreds_of_mostly_distinct_bands() {
        let data = GameDataFiles::load().expect("data files present");
        let world = GameWorld::new(&data, &mut StdRng::seed_from_u64(7));

        assert!(
            world.bands.len() >= 150,
            "the scene should start in the hundreds"
        );
        let distinct: std::collections::HashSet<_> =
            world.bands.iter().map(|b| b.name.as_str()).collect();
        assert!(
            distinct.len() as f32 >= world.bands.len() as f32 * 0.95,
            "band names should be nearly all distinct: {} of {}",
            distinct.len(),
            world.bands.len()
        );
    }

    #[test]
    fn scene_population_stays_bounded_and_charts_fill() {
        let data = GameDataFiles::load().expect("data files present");
        let timeline = MusicTimeline::new(&data);
        let mut world = GameWorld::new(&data, &mut StdRng::seed_from_u64(11));
        let mut rng = StdRng::seed_from_u64(12);
        let mut news_seen = 0;

        for _ in 0..300 {
            news_seen += world.update_week(&timeline, &data, &mut rng).len();
            let n = world.bands.len();
            assert!(
                (SCENE_MIN_BANDS..=SCENE_MAX_BANDS).contains(&n),
                "scene size {n} out of bounds"
            );
            assert!(world.charts.len() <= CHART_SIZE);
        }

        assert!(
            !world.charts.is_empty(),
            "a living scene keeps the charts full"
        );
        assert!(news_seen > 0, "300 weeks should produce scene news");
    }

    #[test]
    fn chart_submission_ranks_and_evicts() {
        let data = GameDataFiles::load().expect("data files present");
        let mut world = GameWorld::new(&data, &mut StdRng::seed_from_u64(5));

        for i in 0..CHART_SIZE {
            world.submit_chart_entry(
                format!("Filler {i}"),
                "Someone".into(),
                false,
                100 + i as u32,
            );
        }
        let pos = world.submit_chart_entry("Big Hit".into(), "You".into(), true, 5000);
        assert_eq!(pos, Some(1), "a huge score should enter at #1");
        let flop = world.submit_chart_entry("Flop".into(), "You".into(), true, 1);
        assert_eq!(flop, None, "a tiny score should miss a full chart");
        assert_eq!(world.charts.len(), CHART_SIZE);
    }

    #[test]
    fn rejected_deals_get_poached_by_the_biggest_unsigned_act() {
        let data = GameDataFiles::load().expect("data files present");
        let mut world = GameWorld::new(&data, &mut StdRng::seed_from_u64(21));
        for band in &mut world.bands {
            band.label = None;
        }
        let biggest = world
            .bands
            .iter()
            .max_by_key(|b| b.fame)
            .map(|b| b.name.clone())
            .unwrap();

        let mut rng = StdRng::seed_from_u64(3);
        let mut poached = None;
        for _ in 0..20 {
            poached = world.poach_rejected_deal("Apex Records", &mut rng);
            if poached.is_some() {
                break;
            }
        }
        assert_eq!(poached.as_deref(), Some(biggest.as_str()));
        let signed = world.bands.iter().find(|b| b.name == biggest).unwrap();
        assert_eq!(signed.label.as_deref(), Some("Apex Records"));
    }

    // --- Deal scouting: label tiers unlock along a real career arc ---

    use crate::game::music::{Release, ReleaseType};

    fn record(release_type: ReleaseType) -> Release {
        Release {
            id: 0,
            name: "Test Record".to_string(),
            release_type,
            release_quality: 50,
            week_released: 0,
            songs_involved_quality_avg: 50,
            active_marketing: Vec::new(),
            marketing_level_achieved: 0,
            initial_sales_score: 0,
            total_income_generated: 0,
            genre: None,
            copies_pressed: 0,
            copies_sold: 0,
        }
    }

    /// A player band with the given fame and catalog.
    fn act(fame: u8, singles: usize, albums: usize) -> Band {
        Band {
            fame,
            singles_released: (0..singles).map(|_| record(ReleaseType::Single)).collect(),
            albums_released: (0..albums).map(|_| record(ReleaseType::Album)).collect(),
            ..Band::default()
        }
    }

    /// Which tiers ever bite across plenty of offer rolls. The threshold
    /// gates are deterministic — only the per-label coin flip is random —
    /// so enough rolls make an unlocked tier impossible to miss, while a
    /// locked tier stays silent on every single roll.
    fn tiers_scouting(
        world: &GameWorld,
        data: &GameDataFiles,
        band: &Band,
    ) -> std::collections::HashSet<String> {
        let mut rng = StdRng::seed_from_u64(77);
        let mut tiers = std::collections::HashSet::new();
        for _ in 0..40 {
            for offer in world.generate_deal_offers(band, data, &mut rng) {
                tiers.insert(offer.label_tier);
            }
        }
        tiers
    }

    #[test]
    fn independent_labels_scout_a_working_act_early() {
        let data = GameDataFiles::load().expect("data files present");
        let world = GameWorld::new(&data, &mut StdRng::seed_from_u64(31));

        // Local fame and a first single out: indies bite, nobody bigger.
        let tiers = tiers_scouting(&world, &data, &act(25, 1, 0));
        assert!(
            tiers.contains("Independent"),
            "a fame-25 act with a single out should draw indie interest, got {tiers:?}"
        );
        assert!(
            !tiers.contains("Boutique"),
            "boutiques should wait for the mid-career"
        );
        assert!(
            !tiers.contains("Major"),
            "majors do not scout the small clubs"
        );

        // The same noise with nothing on the shelves draws nobody at all.
        let all_talk = tiers_scouting(&world, &data, &act(30, 0, 0));
        assert!(
            all_talk.is_empty(),
            "no catalog, no offers, got {all_talk:?}"
        );

        // An act that went straight to an album is a record out all the
        // same — not invisible for lacking a 45.
        let album_first = tiers_scouting(&world, &data, &act(25, 0, 1));
        assert!(
            album_first.contains("Independent"),
            "an album counts as a record out, got {album_first:?}"
        );
    }

    #[test]
    fn boutique_labels_court_the_mid_career_act() {
        let data = GameDataFiles::load().expect("data files present");
        let world = GameWorld::new(&data, &mut StdRng::seed_from_u64(31));

        let tiers = tiers_scouting(&world, &data, &act(50, 2, 1));
        assert!(
            tiers.contains("Boutique"),
            "a fame-50 act with a small catalog should draw boutique interest, got {tiers:?}"
        );
        assert!(
            !tiers.contains("Major"),
            "majors should still be out of reach at fame 50"
        );
    }

    #[test]
    fn major_labels_only_sign_genuinely_big_acts() {
        let data = GameDataFiles::load().expect("data files present");
        let world = GameWorld::new(&data, &mut StdRng::seed_from_u64(31));

        // An album and three singles out, but fame 60: the majors keep watching.
        let almost = tiers_scouting(&world, &data, &act(60, 3, 1));
        assert!(
            !almost.contains("Major"),
            "fame 60 should not yet be enough for the majors, got {almost:?}"
        );
        assert!(
            almost.contains("Boutique"),
            "the mid tiers should be all over them though"
        );

        // Ten more points of fame and the phone rings. (Under the old
        // placeholder — buzz = fame/5 against a threshold of 30 — no major
        // could call even at fame 100.)
        let star = tiers_scouting(&world, &data, &act(70, 3, 1));
        assert!(
            star.contains("Major"),
            "a fame-70 act with a real catalog should finally draw a major, got {star:?}"
        );

        // But no album, no major — singles alone don't prove you can
        // deliver the LPs a major contract is made of.
        let no_album = tiers_scouting(&world, &data, &act(70, 4, 0));
        assert!(
            !no_album.contains("Major"),
            "majors require an album in the catalog, got {no_album:?}"
        );
    }

    #[test]
    fn buzz_weighs_the_catalog_and_a_charting_record() {
        let data = GameDataFiles::load().expect("data files present");
        let mut world = GameWorld::new(&data, &mut StdRng::seed_from_u64(31));

        // Same fame, deeper catalog: more buzz...
        assert!(world.band_buzz(&act(50, 2, 1)) > world.band_buzz(&act(50, 0, 0)));
        // ...but a shelf of records alone never adds up to stardom.
        assert_eq!(
            world.band_buzz(&act(0, 10, 10)),
            BUZZ_CATALOG_CAP as u8,
            "the catalog contribution is capped"
        );

        // A record on this week's chart adds heat while it lasts.
        let star = act(60, 3, 1);
        let cold = world.band_buzz(&star);
        world.submit_chart_entry("The Hit".into(), "The Test Pattern".into(), true, 5_000);
        assert_eq!(world.band_buzz(&star), cold + BUZZ_CHART_BONUS as u8);
    }
}
