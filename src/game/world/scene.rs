//! Scene bands: population, weekly lives, worldgen of the roster.

use crate::data_loader::GameDataFiles;
use crate::game::genre::MusicGenre;
use crate::game::timeline::MusicTimeline;
use rand::Rng;
use serde::{Deserialize, Serialize};

use super::GameWorld;
use super::regions::{self, ChartRegion};

/// The scene never empties out or grows without bound.
pub const SCENE_START_BANDS: usize = 180;
pub const SCENE_MIN_BANDS: usize = 120;
pub const SCENE_MAX_BANDS: usize = 260;

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

impl GameWorld {
    pub(super) fn update_scene_bands(
        &mut self,
        rng: &mut impl Rng,
        timeline: &MusicTimeline,
        data_files: &GameDataFiles,
        news: &mut Vec<String>,
    ) {
        let era_year = timeline.get_current_era().year;
        let trending = timeline.get_trending_genres();
        // Label + fame at release time travel with the score so the
        // post-loop submission pass can work out where each release
        // charts without re-borrowing `self.bands` (design §C).
        let mut releases: Vec<(usize, String, u32, Option<String>, u8)> = Vec::new();

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

            // Releases: signed bands put records out more often. Calmer
            // than the pre-regional-charts odds (design §C) — four Top-100
            // boards fill without the scene turning over weekly.
            let release_odds = if band.label.is_some() { 26 } else { 44 };
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
                releases.push((idx, title, score, band.label.clone(), band.fame));
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

        // Chart submissions happen after the borrow on bands ends. Every
        // scene band always competes on Local (design §C); unsigned acts
        // spill into the UK once famous enough, signed acts spread by
        // label tier — the `regions` module's presence API decides where.
        for (idx, title, score, label, fame) in releases {
            let band_name = self.bands[idx].name.clone();

            let mut regions_to_chart = vec![ChartRegion::Local];
            match label
                .as_deref()
                .and_then(|name| regions::label_tier_for(name, data_files))
            {
                Some(tier) => regions_to_chart.extend(regions::signed_spread(tier, rng)),
                None => regions_to_chart.extend(regions::unsigned_spillover(fame).iter().copied()),
            }

            let mut positions: Vec<(ChartRegion, usize)> = Vec::new();
            let mut local_position = None;
            for region in regions_to_chart {
                let position =
                    self.submit_chart_entry(region, title.clone(), band_name.clone(), false, score);
                if let Some(pos) = position {
                    if region == ChartRegion::Local {
                        local_position = Some(pos);
                    }
                    positions.push((region, pos));
                }
            }

            // Fame/momentum growth still tracks Local — the home board
            // every act enters, so the scene keeps living the way it
            // always did regardless of how far a release spreads abroad.
            // Bonus scales 5..1 across the full depth-100 board (the old
            // 5..1-over-10 curve, stretched to match).
            if let Some(pos) = local_position {
                let band = &mut self.bands[idx];
                let fame_bonus = ((101u32.saturating_sub(pos as u32)) / 20).max(1) as u8;
                band.fame = (band.fame + fame_bonus).min(100);
                band.momentum = (band.momentum + 1).min(3);
                band.peak_fame = band.peak_fame.max(band.fame);
                // A charting record crowds the market a little.
                self.music_market.saturation = (self.music_market.saturation + 1).min(95);
            }

            let current_fame = self.bands[idx].fame;
            for (region, pos) in positions {
                if pos <= 5 || current_fame >= 60 {
                    news.push(format!(
                        "📀 {}'s '{}' charts at #{} {}.",
                        band_name,
                        title,
                        pos,
                        region.label()
                    ));
                }
            }
        }

        self.fill_territories(rng, timeline, data_files);
    }

    /// Bands break up and new ones form: the scene has a life of its own.
    pub(super) fn update_scene_population(
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

    pub(super) fn generate_scene(data_files: &GameDataFiles, rng: &mut impl Rng) -> Vec<SceneBand> {
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
}
