//! Player weekly actions (split by concern). Methods remain on `Game`.

use rand::Rng;

use super::super::constants::{self, *};
use super::super::*;

impl Game {
    pub(crate) fn calculate_songwriting_quality(&self, rng: &mut impl Rng) -> u8 {
        let mut quality = QUALITY_BASE_SONGWRITING as f32;

        // Creativity bonus (0–25 range at creativity 0–100)
        let creativity_bonus =
            (self.player.creativity as f32) / (SONGWRITING_CREATIVITY_DIVISOR as f32);

        // Band member skill bonus
        let skill_bonus = (self.band.average_member_skill() / 15) as f32;

        quality += creativity_bonus + skill_bonus;

        // Random variation (same RNG call as before to preserve determinism)
        let random_offset = rng.gen_range(0..=QUALITY_SONGWRITING_RANDOM_VARIATION) as i8
            - (QUALITY_SONGWRITING_RANDOM_VARIATION / 2) as i8;
        quality += random_offset as f32;

        // Apply happiness multiplier: 0.8 + (happiness / 500.0), clamped 0.8–1.0
        let happiness_multiplier = (HAPPINESS_QUALITY_MULTIPLIER_MIN
            + (self.player.happiness as f32) / HAPPINESS_QUALITY_MULTIPLIER_SCALE)
            .clamp(HAPPINESS_QUALITY_MULTIPLIER_MIN, 1.0);
        quality *= happiness_multiplier;

        quality.clamp(1.0, 100.0) as u8
    }

    fn get_selected_songs_for_release(
        &mut self,
        count: usize,
    ) -> Result<(Vec<music::Song>, u8), String> {
        if self.band.unreleased_songs.len() < count {
            return Err(format!(
                "Not enough unreleased songs. Need {}, have {}.",
                count,
                self.band.unreleased_songs.len()
            ));
        }

        let selected_songs: Vec<music::Song> = self
            .band
            .unreleased_songs
            .drain((self.band.unreleased_songs.len() - count)..)
            .collect();

        if selected_songs.is_empty() && count > 0 {
            return Err("No songs were selected, though count was > 0.".to_string());
        }
        if count == 0 {
            return Ok((Vec::new(), 0));
        }

        let total_quality: u32 = selected_songs
            .iter()
            .map(|s| s.songwriting_quality as u32)
            .sum();
        let avg_quality = (total_quality / selected_songs.len() as u32) as u8;

        Ok((selected_songs, avg_quality))
    }

    pub(crate) fn calculate_release_quality(&self, avg_song_quality: u8, rng: &mut impl Rng) -> u8 {
        let mut quality = (QUALITY_BASE_RECORDING as f32 + avg_song_quality as f32) / 2.0;

        // Band skill term
        quality += (self.band.skill / 10) as f32;

        // Condition penalty: −10 if stress > threshold
        if self.player.stress > RECORDING_STRESS_PENALTY_THRESHOLD {
            quality -= RECORDING_STRESS_PENALTY as f32;
        }

        // Random variation (same RNG call as before to preserve determinism)
        let random_offset = rng.gen_range(0..=QUALITY_RECORDING_RANDOM_VARIATION) as i8
            - (QUALITY_RECORDING_RANDOM_VARIATION / 2) as i8;
        quality += random_offset as f32;

        // Apply happiness multiplier: 0.8 + (happiness / 500.0), clamped 0.8–1.0
        let happiness_multiplier = (HAPPINESS_QUALITY_MULTIPLIER_MIN
            + (self.player.happiness as f32) / HAPPINESS_QUALITY_MULTIPLIER_SCALE)
            .clamp(HAPPINESS_QUALITY_MULTIPLIER_MIN, 1.0);
        quality *= happiness_multiplier;

        quality.clamp((avg_song_quality as f32 / 2.0).max(1.0), 100.0) as u8
    }

    pub(in crate::game) fn action_write_songs(&mut self, rng: &mut impl Rng) -> Result<(), String> {
        // Guard: stress blocks writing (§A)
        if self.player.stress >= STUDIO_STRESS_BLOCK {
            return Err("You're too stressed to write coherent music!".to_string());
        }

        // Increment writing streak at the start
        self.writing_streak += 1;

        let num_songs_to_write = rng.gen_range(1..=3);
        let mut titles = Vec::new();
        for _ in 0..num_songs_to_write {
            let quality = self.calculate_songwriting_quality(rng);
            let song_name = self.data_files.generate_song_title(rng);
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

        // Apply stress cost and creativity consumption rules (§A)
        self.player.stress = (self.player.stress + WRITE_STRESS_COST).min(constants::MAX_STRESS);

        // Creativity consumption: only when forced
        // 1. Writing too much: 3rd and every subsequent consecutive writing week
        if self.writing_streak >= WRITING_STREAK_FATIGUE {
            self.player.creativity = self
                .player
                .creativity
                .saturating_sub(WRITING_FATIGUE_CREATIVITY_COST);
        }

        // 2. Writing under stress: stress > 50 at the moment of writing
        if self.player.stress > WRITING_STRESS_CREATIVITY_THRESHOLD {
            let stress_over_threshold = self.player.stress - WRITING_STRESS_CREATIVITY_THRESHOLD;
            let creativity_cost = (stress_over_threshold as f32
                / WRITING_STRESS_CREATIVITY_DIVISOR as f32)
                .ceil() as u8;
            self.player.creativity = self.player.creativity.saturating_sub(creativity_cost);
        }

        Ok(())
    }

    pub(in crate::game) fn action_practice(&mut self) -> Result<(), String> {
        if self.player.stress >= STUDIO_STRESS_BLOCK {
            return Err("You're too stressed to focus on rehearsal!".to_string());
        }
        self.player.stress = (self.player.stress + PRACTICE_STRESS_COST).min(constants::MAX_STRESS);
        self.band.skill = (self.band.skill + 2).min(constants::MAX_SKILL);
        let skill = self.band.skill;
        self.log(format!(
            "🥁 A week in the rehearsal room — band skill is now {}%.",
            skill
        ));
        Ok(())
    }

    pub(in crate::game) fn action_record_single(
        &mut self,
        pressing: Option<usize>,
        rng: &mut impl Rng,
    ) -> Result<(), String> {
        // Guard: stress blocks recording (§A)
        if self.player.stress >= STUDIO_STRESS_BLOCK {
            return Err("You're too stressed to record quality music!".to_string());
        }

        if !self.band.can_record_single() {
            return Err("You need to write at least one song first!".to_string());
        }

        let recording_cost = self.recording_cost(&music::ReleaseType::Single);
        let (copies, pressing_cost) = self.plan_pressing(&music::ReleaseType::Single, pressing)?;
        let cost = recording_cost + pressing_cost;
        if !self.player.can_afford(cost) {
            if pressing_cost > 0 {
                return Err(format!(
                    "An independent single costs ${} — ${} studio time plus ${} to press {} copies!",
                    cost, recording_cost, pressing_cost, copies
                ));
            }
            return Err(format!("You need at least ${} to record a single!", cost));
        }

        let (selected_songs, avg_song_quality) = self.get_selected_songs_for_release(1)?;
        if selected_songs.is_empty() {
            return Err("Failed to select a song for the single.".to_string());
        }
        self.player.spend_money(cost);

        let release_quality = self.calculate_release_quality(avg_song_quality, rng);
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
            genre: Some(self.band.genre.clone()),
            copies_pressed: copies,
            copies_sold: 0,
            peak_chart_position: None,
            singles_cut: 0,
        };
        let name = new_release.name.clone();
        self.just_released_music.push(new_release);
        self.next_release_id += 1;
        if pressing_cost > 0 {
            self.log(format!(
                "🎙️ Recorded '{}' for ${} and pressed {} copies for ${} — out in {} weeks.",
                name, recording_cost, copies, pressing_cost, INITIAL_SALES_WINDOW_WEEKS
            ));
        } else {
            self.log(format!(
                "🎙️ Recorded '{}' for ${} — the label presses {} copies, out in {} weeks.",
                name, recording_cost, copies, INITIAL_SALES_WINDOW_WEEKS
            ));
        }
        self.apply_label_promo();

        // Apply stress cost (§A)
        self.player.stress = (self.player.stress + RECORD_STRESS_COST).min(constants::MAX_STRESS);

        Ok(())
    }

    pub(in crate::game) fn action_record_album(
        &mut self,
        pressing: Option<usize>,
        rng: &mut impl Rng,
    ) -> Result<(), String> {
        // Guard: stress blocks recording (§A)
        if self.player.stress >= STUDIO_STRESS_BLOCK {
            return Err("You're too stressed to record quality music!".to_string());
        }

        if !self.band.can_record_album() {
            return Err(format!(
                "You need at least {} unreleased songs to record an album!",
                constants::MIN_ALBUM_SONGS
            ));
        }

        let recording_cost = self.recording_cost(&music::ReleaseType::Album);
        let (copies, pressing_cost) = self.plan_pressing(&music::ReleaseType::Album, pressing)?;
        let cost = recording_cost + pressing_cost;
        if !self.player.can_afford(cost) {
            if pressing_cost > 0 {
                return Err(format!(
                    "An independent album costs ${} — ${} studio time plus ${} to press {} copies!",
                    cost, recording_cost, pressing_cost, copies
                ));
            }
            return Err(format!("You need at least ${} to record an album!", cost));
        }

        let (selected_songs, avg_song_quality) =
            self.get_selected_songs_for_release(constants::MIN_ALBUM_SONGS as usize)?;
        if selected_songs.len() < constants::MIN_ALBUM_SONGS as usize {
            return Err("Not enough songs selected for an album.".to_string());
        }
        self.player.spend_money(cost);

        let release_quality = self.calculate_release_quality(avg_song_quality, rng);
        let release_name = self.data_files.random_album_title(rng);

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
            genre: Some(self.band.genre.clone()),
            copies_pressed: copies,
            copies_sold: 0,
            peak_chart_position: None,
            singles_cut: 0,
        };
        let name = new_release.name.clone();
        self.just_released_music.push(new_release);
        self.next_release_id += 1;
        if pressing_cost > 0 {
            self.log(format!(
                "🎙️ Recorded the album '{}' for ${} and pressed {} copies for ${} — out in {} weeks.",
                name, recording_cost, copies, pressing_cost, INITIAL_SALES_WINDOW_WEEKS
            ));
        } else {
            self.log(format!(
                "🎙️ Recorded the album '{}' for ${} — the label presses {} copies, out in {} weeks.",
                name, recording_cost, copies, INITIAL_SALES_WINDOW_WEEKS
            ));
        }
        self.apply_label_promo();

        if self.timeline.is_album_era() {
            self.band.fame = (self.band.fame + 3).min(constants::MAX_FAME);
            self.log(
                "📈 It's an album-oriented era — the announcement alone earns you buzz (+3 fame).",
            );
        }

        // Apply stress cost (§A)
        self.player.stress = (self.player.stress + RECORD_STRESS_COST).min(constants::MAX_STRESS);

        Ok(())
    }
}
