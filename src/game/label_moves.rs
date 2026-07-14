//! Label initiatives: single-cuts and other label-driven actions
//! (design §C — Label single-cuts).

use rand::Rng;

use super::constants::{
    LABEL_CUT_CHANCE, LABEL_CUT_IDLE_WEEKS, LABEL_CUT_MAX_PER_ALBUM,
    LABEL_CUT_RELEASE_COOLDOWN_WEEKS,
};
use super::music::ReleaseType;
use super::*;

impl Game {
    /// Check if the label should cut a single from an unreleased album.
    ///
    /// Fires only when ALL conditions hold (design §C):
    /// - signed to a label
    /// - some album has fewer than 2 singles already cut from it
    /// - the band has been quiet for ≥ 3 weeks
    /// - no release in the last 6 weeks
    /// - a 10% weekly roll succeeds
    ///
    /// When it fires: picks the eligible album (most recent first), increments
    /// its cut counter, and creates a new SINGLE `Release` into
    /// `just_released_music` with label pressing and promo. The single enters
    /// the launch window like any release and counts toward activity rules.
    pub(super) fn label_single_cut_check(&mut self, rng: &mut impl Rng) {
        // Condition 1: must be signed.
        let Some(deal) = self.band.current_deal() else {
            return;
        };
        let label_name = deal.label_name.clone();
        let deal_market_reach = deal.market_reach;

        // Condition 2: find an eligible album (un-singled tracks, most recent first).
        // We need to check if there's an eligible album first.
        let has_eligible = self
            .band
            .albums_released
            .iter()
            .rev()
            .any(|album| album.singles_cut < LABEL_CUT_MAX_PER_ALBUM);

        if !has_eligible {
            return;
        }

        // Condition 3: must be quiet (idle_streak ≥ 3).
        if self.idle_streak < LABEL_CUT_IDLE_WEEKS {
            return;
        }

        // Condition 4: no release in the last 6 weeks.
        if self.has_recent_release_for_cut() {
            return;
        }

        // Condition 5: roll on the rng (10% chance). This is the last check so
        // weeks where earlier conditions fail draw nothing from the stream.
        if !rng.gen_bool(LABEL_CUT_CHANCE) {
            return;
        }

        // All conditions passed — find and cut the single.
        // Now that we've passed all guards, we can mutate.
        let album = self
            .band
            .albums_released
            .iter_mut()
            .rev()
            .find(|album| album.singles_cut < LABEL_CUT_MAX_PER_ALBUM)
            .expect("has_eligible guarantees this");

        let album_name = album.name.clone();
        let album_quality = album.release_quality;
        let album_genre = album.genre.clone();

        // Increment the album's cut counter.
        album.singles_cut += 1;

        // Create a single cut from the album.
        let single_name = format!("{} (single)", album_name);

        // Label pressing size: the label's network plus your name (duplicated from economy.rs).
        let label_pressing = u32::from(deal_market_reach) * constants::LABEL_PRESSING_PER_REACH
            + u32::from(self.band.fame) * constants::LABEL_PRESSING_PER_FAME;

        let new_release = music::Release {
            id: self.next_release_id,
            name: single_name.clone(),
            release_type: ReleaseType::Single,
            release_quality: album_quality,
            week_released: self.week,
            songs_involved_quality_avg: album.songs_involved_quality_avg,
            active_marketing: Vec::new(),
            marketing_level_achieved: 0,
            initial_sales_score: 0,
            total_income_generated: 0,
            genre: album_genre,
            copies_pressed: label_pressing,
            copies_sold: 0,
            peak_chart_position: None,
            singles_cut: 0,
        };

        self.just_released_music.push(new_release);
        self.next_release_id += 1;

        // Log the cut in the design's voice.
        self.log(format!(
            "📀 Without asking, {} pulls '{}' off the album as a single.",
            label_name, single_name
        ));

        // Apply the label's promo machine (same as any label release).
        self.apply_label_promo();
    }

    /// Check if any release (album, single, or launch-window) was released
    /// within the cooldown window. Used by label_single_cut_check.
    fn has_recent_release_for_cut(&self) -> bool {
        let cutoff_week = self.week.saturating_sub(LABEL_CUT_RELEASE_COOLDOWN_WEEKS);
        self.band
            .albums_released
            .iter()
            .chain(self.band.singles_released.iter())
            .chain(self.just_released_music.iter())
            .any(|release| release.week_released > cutoff_week)
    }
}
