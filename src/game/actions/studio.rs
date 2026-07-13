//! Player weekly actions (split by concern). Methods remain on `Game`.

use super::super::constants::{self, *};
use super::super::*;

impl Game {
    fn calculate_songwriting_quality(&self, rng: &mut impl Rng) -> u8 {
        let mut quality = QUALITY_BASE_SONGWRITING as f32;
        let mut player_bonus = 0.0;

        // Player energy bonus
        if self.player.energy > 70 {
            player_bonus += 5.0;
        } else if self.player.energy > 40 {
            player_bonus += 2.0;
        }

        // Player stress bonus (low stress is good)
        if self.player.stress < 30 {
            player_bonus += 5.0;
        } else if self.player.stress < 60 {
            player_bonus += 2.0;
        }

        // Band member skill bonus
        player_bonus += (self.band.average_member_skill() / 15) as f32;

        quality += player_bonus.min(QUALITY_SONGWRITING_MAX_BONUS_PLAYER_STATS as f32);

        // Random variation
        let random_offset = rng.gen_range(0..=QUALITY_SONGWRITING_RANDOM_VARIATION) as i8
            - (QUALITY_SONGWRITING_RANDOM_VARIATION / 2) as i8;
        quality += random_offset as f32;

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

    fn calculate_release_quality(&self, avg_song_quality: u8, rng: &mut impl Rng) -> u8 {
        let mut quality = (QUALITY_BASE_RECORDING as f32 + avg_song_quality as f32) / 2.0;

        quality += (self.band.skill / 10) as f32;

        let mut player_bonus: f32 = 0.0;
        if self.player.energy > 70 {
            player_bonus += 3.0;
        } else if self.player.energy > 40 {
            player_bonus += 1.0;
        }
        if self.player.stress < 30 {
            player_bonus += 3.0;
        } else if self.player.stress < 60 {
            player_bonus += 1.0;
        }
        quality += player_bonus.min(QUALITY_RECORDING_MAX_BONUS_PLAYER_STATS as f32);

        let random_offset = rng.gen_range(0..=QUALITY_RECORDING_RANDOM_VARIATION) as i8
            - (QUALITY_RECORDING_RANDOM_VARIATION / 2) as i8;
        quality += random_offset as f32;

        quality.clamp((avg_song_quality as f32 / 2.0).max(1.0), 100.0) as u8
    }

    pub(in crate::game) fn action_write_songs(&mut self, rng: &mut impl Rng) -> Result<(), String> {
        if self.player.energy < 20 {
            return Err("You're too tired to write songs!".to_string());
        }
        self.player.energy -= 20;

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
        Ok(())
    }

    pub(in crate::game) fn action_practice(&mut self) -> Result<(), String> {
        if self.player.energy < 15 {
            return Err("You're too tired to practice!".to_string());
        }
        self.player.energy -= 15;
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
        Ok(())
    }

    pub(in crate::game) fn action_record_album(
        &mut self,
        pressing: Option<usize>,
        rng: &mut impl Rng,
    ) -> Result<(), String> {
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
        Ok(())
    }
}
