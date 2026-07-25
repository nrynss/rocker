//! Player weekly actions (split by concern). Methods remain on `Game`.
//!
//! Recording is a project, not a purchase (design §A/§C): the player picks a
//! format, a song count inside that format's range, and one of the five
//! studios, and is quoted the cost, the weeks, and the quality band before
//! anything is charged.

use rand::Rng;

use crate::game::music::ReleaseType;
use crate::game::studios::{self, RecordingQuote, RecordingStudio};

use super::super::constants::{self, *};
use super::super::*;

/// The deterministic (no-rng) core of a booking: everything the quote and the
/// actual recording must agree on. Shared by `quote_recording` (read-only, for
/// the picker) and `action_record_single`/`action_record_album` — the single
/// source of truth that keeps the quote honest, exactly as `TourPot` does for
/// tours (design §C — "the quote comes first").
struct RecordingPlan {
    songs: u8,
    studio: RecordingStudio,
    cost: i32,
    weeks: u8,
    avg_song_quality: u8,
}

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

    /// The positions of the best `count` unreleased songs, returned in the
    /// order they were written (design §A).
    ///
    /// This is the fix for the old tail-drain: `get_selected_songs_for_release`
    /// used to take the *last* `count` songs in the vector, silently spending
    /// whatever was newest regardless of quality. Picking the best N is what
    /// makes "write more, then pick" a real strategy instead of a superstition.
    /// Ties break toward the older song (the sort is stable over ascending
    /// indices), so the pick is total and replays identically.
    fn best_song_indices(&self, count: usize) -> Vec<usize> {
        let songs = &self.band.unreleased_songs;
        let mut order: Vec<usize> = (0..songs.len()).collect();
        order.sort_by_key(|&i| std::cmp::Reverse(songs[i].songwriting_quality));
        order.truncate(count);
        // Back to written order: a record's tracklisting is not a ranking, and
        // the caller removes by index, which needs them sorted anyway.
        order.sort_unstable();
        order
    }

    fn average_song_quality(&self, indices: &[usize]) -> u8 {
        if indices.is_empty() {
            return 0;
        }
        let total: u32 = indices
            .iter()
            .map(|&i| self.band.unreleased_songs[i].songwriting_quality as u32)
            .sum();
        (total / indices.len() as u32) as u8
    }

    /// Take the best `count` songs out of the notebook, returning them and
    /// their average songwriting quality. Removes **exactly** the chosen songs
    /// and leaves every other one where it was.
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
        if count == 0 {
            return Ok((Vec::new(), 0));
        }

        let chosen = self.best_song_indices(count);
        let avg_quality = self.average_song_quality(&chosen);
        // Remove back-to-front so each remaining index stays valid.
        let mut selected_songs: Vec<music::Song> = chosen
            .iter()
            .rev()
            .map(|&i| self.band.unreleased_songs.remove(i))
            .collect();
        selected_songs.reverse();

        Ok((selected_songs, avg_quality))
    }

    /// The whole of the release-quality formula except the roll, so the
    /// picker's projected band (design §C) and the recording that follows can
    /// never disagree about what a room is worth.
    fn release_quality_with_offset(
        &self,
        avg_song_quality: u8,
        studio: RecordingStudio,
        random_offset: i8,
    ) -> u8 {
        let mut quality = (QUALITY_BASE_RECORDING as f32 + avg_song_quality as f32) / 2.0;

        // Band skill term
        quality += (self.band.skill / 10) as f32;

        // Condition penalty: −10 if stress > threshold
        if self.player.stress > RECORDING_STRESS_PENALTY_THRESHOLD {
            quality -= RECORDING_STRESS_PENALTY as f32;
        }

        quality += random_offset as f32;

        // Apply happiness multiplier: 0.8 + (happiness / 500.0), clamped 0.8–1.0
        let happiness_multiplier = (HAPPINESS_QUALITY_MULTIPLIER_MIN
            + (self.player.happiness as f32) / HAPPINESS_QUALITY_MULTIPLIER_SCALE)
            .clamp(HAPPINESS_QUALITY_MULTIPLIER_MIN, 1.0);
        quality *= happiness_multiplier;

        // The room (design §C). Added after the multiplier and before the
        // clamp: a studio shifts the distribution by a fixed number of points
        // rather than scaling it, so it is worth the same to a miserable band
        // as to a happy one, and the world-class room cannot turn weak songs
        // into a hit on its own.
        quality += f32::from(studio.quality_bonus());

        quality.clamp((avg_song_quality as f32 / 2.0).max(1.0), 100.0) as u8
    }

    pub(crate) fn calculate_release_quality(
        &self,
        avg_song_quality: u8,
        studio: RecordingStudio,
        rng: &mut impl Rng,
    ) -> u8 {
        // Random variation (same RNG call as before to preserve determinism)
        let random_offset = rng.gen_range(0..=QUALITY_RECORDING_RANDOM_VARIATION) as i8
            - (QUALITY_RECORDING_RANDOM_VARIATION / 2) as i8;
        self.release_quality_with_offset(avg_song_quality, studio, random_offset)
    }

    /// Fame gate for a studio: fame never re-prices a booking, it only decides
    /// which rooms are on offer (design §C).
    pub fn studio_is_available(&self, studio: RecordingStudio) -> bool {
        self.band.fame >= studio.fame_gate()
    }

    /// Weeks in the room: `ceil(songs × weeks_per_song)`, clamped to
    /// `[1, RECORDING_MAX_WEEKS]` (design §B). Quoted here and *spent* by
    /// S-TIME (§F) — nothing in this task consumes them.
    fn recording_weeks(songs: u8, studio: RecordingStudio) -> u8 {
        let raw = (f32::from(songs) * studio.weeks_per_song()).ceil();
        (raw.clamp(1.0, f32::from(RECORDING_MAX_WEEKS))) as u8
    }

    /// The deterministic core shared by `quote_recording` and the two record
    /// actions (design §A/§C): validates the song count against the format's
    /// range, the studio against its fame gate, and the notebook against the
    /// count — naming the shortfall when it falls short — then prices the
    /// studio time and counts the weeks. No rng, no mutation.
    fn recording_plan(
        &self,
        release_type: &ReleaseType,
        songs: u8,
        studio: RecordingStudio,
    ) -> Result<RecordingPlan, String> {
        let format = studios::format_label(release_type);
        let (min_songs, max_songs) = studios::song_count_bounds(release_type);
        if !(min_songs..=max_songs).contains(&songs) {
            return Err(format!(
                "A {} is {}-{} songs — {} isn't one.",
                format, min_songs, max_songs, songs
            ));
        }
        if !self.studio_is_available(studio) {
            return Err(format!(
                "'{}' needs at least {} fame.",
                studio.label(),
                studio.fame_gate()
            ));
        }

        let available = self.band.unreleased_songs.len();
        if available < songs as usize {
            let shortfall = songs as usize - available;
            return Err(format!(
                "A {}-song {} needs {} more song{} — you have {} unreleased.",
                songs,
                format,
                shortfall,
                if shortfall == 1 { "" } else { "s" },
                available
            ));
        }

        let chosen = self.best_song_indices(songs as usize);
        Ok(RecordingPlan {
            songs,
            studio,
            cost: self.recording_cost(studio, songs),
            weeks: Self::recording_weeks(songs, studio),
            avg_song_quality: self.average_song_quality(&chosen),
        })
    }

    /// The up-front quote (design §C): the record picker must show this before
    /// booking. Pure — no rng, no mutation — so the UI can recompute it live as
    /// the player changes format, song count, or room.
    pub fn quote_recording(
        &self,
        release_type: ReleaseType,
        songs: u8,
        studio: RecordingStudio,
    ) -> Result<RecordingQuote, String> {
        let plan = self.recording_plan(&release_type, songs, studio)?;

        // The projected band is the same formula the recording runs, at the two
        // ends of its one roll — so the record that comes out cannot land
        // outside the band the player was shown.
        let spread = (QUALITY_RECORDING_RANDOM_VARIATION / 2) as i8;
        let quality_low = self.release_quality_with_offset(plan.avg_song_quality, studio, -spread);
        let quality_high = self.release_quality_with_offset(
            plan.avg_song_quality,
            studio,
            QUALITY_RECORDING_RANDOM_VARIATION as i8 - spread,
        );

        Ok(RecordingQuote {
            release_type,
            studio,
            songs: plan.songs,
            cost: plan.cost,
            weeks: plan.weeks,
            avg_song_quality: plan.avg_song_quality,
            quality_low,
            quality_high,
        })
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
        // L12: rehearsal is also where individual musicianship grows —
        // `average_member_skill()` feeds live reception (design §B) and,
        // unlike `band.skill` above, was never written after band creation.
        for member in &mut self.band.members {
            member.skill = (member.skill + PRACTICE_MEMBER_SKILL_GAIN).min(constants::MAX_SKILL);
        }
        let skill = self.band.skill;
        self.log(format!(
            "🥁 A week in the rehearsal room — band skill is now {}%.",
            skill
        ));
        Ok(())
    }

    pub(in crate::game) fn action_record_single(
        &mut self,
        songs: u8,
        studio: RecordingStudio,
        pressing: Option<usize>,
        rng: &mut impl Rng,
    ) -> Result<(), String> {
        // Guard: stress blocks recording (§A)
        if self.player.stress >= STUDIO_STRESS_BLOCK {
            return Err("You're too stressed to record quality music!".to_string());
        }

        // S-RECORD (§A/§C): the song count, the room, and the bill are all
        // decided here, by the same core that quoted them to the picker.
        let plan = self.recording_plan(&music::ReleaseType::Single, songs, studio)?;
        let recording_cost = plan.cost;
        let (copies, pressing_cost) = self.plan_pressing(&music::ReleaseType::Single, pressing)?;
        // M6 (§E-3): an unsigned release also buys reach through a
        // distribution channel — fee due now, alongside recording/pressing.
        // `plan_distribution` is `Ok(0)` while signed (label deals ignore
        // channels entirely).
        let distribution_channel = self.current_distribution_channel;
        let distribution_fee = self.plan_distribution(distribution_channel)?;
        let cost = recording_cost + pressing_cost + distribution_fee;
        if !self.player.can_afford(cost) {
            if distribution_fee > 0 {
                return Err(format!(
                    "An independent single costs ${} — ${} studio time, ${} to press {} copies, and ${} for {}!",
                    cost,
                    recording_cost,
                    pressing_cost,
                    copies,
                    distribution_fee,
                    distribution_channel.label()
                ));
            }
            if pressing_cost > 0 {
                return Err(format!(
                    "An independent single costs ${} — ${} studio time plus ${} to press {} copies!",
                    cost, recording_cost, pressing_cost, copies
                ));
            }
            return Err(format!("You need at least ${} to record a single!", cost));
        }

        let (selected_songs, avg_song_quality) =
            self.get_selected_songs_for_release(plan.songs as usize)?;
        if selected_songs.is_empty() {
            return Err("Failed to select a song for the single.".to_string());
        }
        self.player.spend_money(cost);

        let release_quality = self.calculate_release_quality(avg_song_quality, plan.studio, rng);
        // A single is an A-side plus B-sides (design §A), and the A-side is the
        // strongest thing on it — so the record is named for the best of the
        // chosen songs, not merely the first one written.
        let a_side = selected_songs
            .iter()
            .max_by_key(|song| song.songwriting_quality)
            .expect("a single always selects at least MIN_SINGLE_SONGS songs");
        let release_name = format!("Single: {}", a_side.name);
        let signed = self.band.current_deal().is_some();

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
            certified: 0,
            distribution_channel: if signed {
                None
            } else {
                Some(distribution_channel)
            },
            // L-REACH (§C): freeze the label's *live* reach on the release (not
            // the value frozen in the deal at signing) so a record cut after the
            // label rose or fell carries its reach *now*; that frozen value then
            // drives this release's catalogue tail for good (§E-3). `None` for an
            // indie release; the deal's signing reach for an unseeded label.
            label_market_reach: self.new_release_frozen_reach(),
        };
        let name = new_release.name.clone();
        self.just_released_music.push(new_release);
        self.next_release_id += 1;
        // The room and the session length are part of the story now (§B/§C),
        // so the log names both — `plan.weeks` is the number S-TIME will spend.
        if pressing_cost > 0 {
            self.log(format!(
                "🎙️ Recorded '{}' — {} songs at the {} for ${} ({} week session) — and pressed {} copies for ${}, out in {} weeks.",
                name,
                plan.songs,
                plan.studio.label(),
                recording_cost,
                plan.weeks,
                copies,
                pressing_cost,
                INITIAL_SALES_WINDOW_WEEKS
            ));
        } else {
            self.log(format!(
                "🎙️ Recorded '{}' — {} songs at the {} for ${} ({} week session). The label presses {} copies, out in {} weeks.",
                name,
                plan.songs,
                plan.studio.label(),
                recording_cost,
                plan.weeks,
                copies,
                INITIAL_SALES_WINDOW_WEEKS
            ));
        }
        if distribution_fee > 0 {
            self.log(format!(
                "🚚 {} takes ${} to carry '{}'.",
                distribution_channel.label(),
                distribution_fee,
                name
            ));
        }
        self.apply_label_promo();

        // Apply stress cost (§A)
        self.player.stress = (self.player.stress + RECORD_STRESS_COST).min(constants::MAX_STRESS);

        Ok(())
    }

    pub(in crate::game) fn action_record_album(
        &mut self,
        songs: u8,
        studio: RecordingStudio,
        pressing: Option<usize>,
        rng: &mut impl Rng,
    ) -> Result<(), String> {
        // Guard: stress blocks recording (§A)
        if self.player.stress >= STUDIO_STRESS_BLOCK {
            return Err("You're too stressed to record quality music!".to_string());
        }

        // S-RECORD (§A/§C): an album is 8-14 songs, the player's call, priced
        // per song at the chosen room's rate — see `recording_plan`.
        let plan = self.recording_plan(&music::ReleaseType::Album, songs, studio)?;
        let recording_cost = plan.cost;
        let (copies, pressing_cost) = self.plan_pressing(&music::ReleaseType::Album, pressing)?;
        // M6 (§E-3): an unsigned release also buys reach through a
        // distribution channel — fee due now, alongside recording/pressing.
        // `plan_distribution` is `Ok(0)` while signed (label deals ignore
        // channels entirely).
        let distribution_channel = self.current_distribution_channel;
        let distribution_fee = self.plan_distribution(distribution_channel)?;
        let cost = recording_cost + pressing_cost + distribution_fee;
        if !self.player.can_afford(cost) {
            if distribution_fee > 0 {
                return Err(format!(
                    "An independent album costs ${} — ${} studio time, ${} to press {} copies, and ${} for {}!",
                    cost,
                    recording_cost,
                    pressing_cost,
                    copies,
                    distribution_fee,
                    distribution_channel.label()
                ));
            }
            if pressing_cost > 0 {
                return Err(format!(
                    "An independent album costs ${} — ${} studio time plus ${} to press {} copies!",
                    cost, recording_cost, pressing_cost, copies
                ));
            }
            return Err(format!("You need at least ${} to record an album!", cost));
        }

        let (selected_songs, avg_song_quality) =
            self.get_selected_songs_for_release(plan.songs as usize)?;
        if selected_songs.len() < plan.songs as usize {
            return Err("Not enough songs selected for an album.".to_string());
        }
        self.player.spend_money(cost);

        let release_quality = self.calculate_release_quality(avg_song_quality, plan.studio, rng);
        let release_name = self.data_files.random_album_title(rng);
        let signed = self.band.current_deal().is_some();

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
            certified: 0,
            distribution_channel: if signed {
                None
            } else {
                Some(distribution_channel)
            },
            // L-REACH (§C): freeze the label's *live* reach on the release (not
            // the value frozen in the deal at signing) so a record cut after the
            // label rose or fell carries its reach *now*; that frozen value then
            // drives this release's catalogue tail for good (§E-3). `None` for an
            // indie release; the deal's signing reach for an unseeded label.
            label_market_reach: self.new_release_frozen_reach(),
        };
        let name = new_release.name.clone();
        self.just_released_music.push(new_release);
        self.next_release_id += 1;
        if pressing_cost > 0 {
            self.log(format!(
                "🎙️ Recorded the album '{}' — {} songs at the {} for ${} ({} week session) — and pressed {} copies for ${}, out in {} weeks.",
                name,
                plan.songs,
                plan.studio.label(),
                recording_cost,
                plan.weeks,
                copies,
                pressing_cost,
                INITIAL_SALES_WINDOW_WEEKS
            ));
        } else {
            self.log(format!(
                "🎙️ Recorded the album '{}' — {} songs at the {} for ${} ({} week session). The label presses {} copies, out in {} weeks.",
                name,
                plan.songs,
                plan.studio.label(),
                recording_cost,
                plan.weeks,
                copies,
                INITIAL_SALES_WINDOW_WEEKS
            ));
        }
        if distribution_fee > 0 {
            self.log(format!(
                "🚚 {} takes ${} to carry '{}'.",
                distribution_channel.label(),
                distribution_fee,
                name
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
