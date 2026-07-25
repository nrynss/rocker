//! Player weekly actions (split by concern). Methods remain on `Game`.
//!
//! Recording is a project, not a purchase (design §A/§C): the player picks a
//! format, a song count inside that format's range, and one of the five
//! studios, and is quoted the cost, the weeks, and the quality band before
//! anything is charged.
//!
//! And recording is not pressing (design §D). The pipeline is three stages,
//! not one keypress:
//!
//! ```text
//! book a session  ->  a Master in the vault  ->  a Release
//!   weeks of studio time    finished, owned,      pressing +
//!   + the room + the songs  not yet out          distribution
//! ```
//!
//! `action_record_*` books the session and charges the studio bill;
//! `resolve_recording_week` plays one week of it (design §F);
//! `settle_recording` deposits the master; `action_release_master` is the
//! separate decision that buys a pressing run and a distribution channel and
//! finally puts the record in shops.

use rand::Rng;

use crate::game::music::{Master, ReleaseType};
use crate::game::studios::{self, RecordingQuote, RecordingStudio};

use super::super::constants::{self, *};
use super::super::*;

/// The activity-stream step a session's own rolls draw from (`rng.rs`).
///
/// A session makes every roll it will ever make at booking — the room's
/// quality roll and the title — so one step is enough, and step 0 cannot
/// collide with any other commitment's: the stream is keyed by the booking
/// week, and only one commitment can be booked in a given week (booking is
/// refused while `ongoing_activity` is `Some`, and it consumes the week).
const RECORDING_SESSION_RNG_STEP: u32 = 0;

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
    /// Read by `quote_recording` alone: the booking itself re-derives the
    /// average from the songs it actually pulls out of the notebook, so the
    /// two can never disagree about a notebook that changed in between.
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

    /// Book a studio session (design §B + §D + §F). The bill is due here, in
    /// full — you cannot start a record you can't finish — and *everything the
    /// record will be* is decided here too: the songs come out of the
    /// notebook, the room's quality roll is made, the title is drawn. The
    /// weeks are then played one per turn by `turn.rs`, and the last of them
    /// deposits a `Master` in the vault. This method never touches
    /// `Game::week`.
    ///
    /// Shared by both formats — a single and an album differ only in their
    /// song-count bounds (§A), their sales ceiling (`SINGLE_SALES_FACTOR`),
    /// and how the record gets its name.
    fn book_recording(
        &mut self,
        release_type: ReleaseType,
        songs: u8,
        studio: RecordingStudio,
    ) -> Result<(), String> {
        if let Some(activity) = &self.ongoing_activity {
            return Err(format!(
                "You're already committed — {} ({}).",
                activity.headline(),
                activity.progress()
            ));
        }
        // Guard: stress blocks recording (§A)
        if self.player.stress >= STUDIO_STRESS_BLOCK {
            return Err("You're too stressed to record quality music!".to_string());
        }

        // S-RECORD (§A/§C): the song count, the room, the bill and the length
        // all come from the same core that quoted them to the picker.
        // S-MASTER (§D): that bill is now *studio time only* — pressing and
        // distribution are `action_release_master`'s to charge.
        let plan = self.recording_plan(&release_type, songs, studio)?;
        if !self.player.can_afford(plan.cost) {
            return Err(format!(
                "A {}-song session at the {} costs ${} up front — {} week{} of studio time.",
                plan.songs,
                plan.studio.label(),
                plan.cost,
                plan.weeks,
                if plan.weeks == 1 { "" } else { "s" }
            ));
        }

        let (selected_songs, avg_song_quality) =
            self.get_selected_songs_for_release(plan.songs as usize)?;
        if selected_songs.len() < plan.songs as usize {
            return Err(format!(
                "Not enough songs selected for the {}.",
                studios::format_label(&release_type)
            ));
        }
        // §B: money is due at booking, in full.
        self.player.spend_money(plan.cost);

        // The record itself is settled now and frozen onto the activity (see
        // `RecordingInProgress`): both rolls come off the activity stream,
        // keyed by the booking week, so what comes out of the room cannot be
        // moved by the weeks that pass while the band is in it — nor by a save
        // taken halfway through. Draw order (quality, then title) is the order
        // the pre-§D action used; do not reorder it.
        let mut rng = self.activity_rng(self.week, RECORDING_SESSION_RNG_STEP);
        let release_quality =
            self.calculate_release_quality(avg_song_quality, plan.studio, &mut rng);
        let title = match release_type {
            // A single is an A-side plus B-sides (design §A), and the A-side is
            // the strongest thing on it — so the record is named for the best
            // of the chosen songs, not merely the first one written.
            ReleaseType::Single => format!(
                "Single: {}",
                selected_songs
                    .iter()
                    .max_by_key(|song| song.songwriting_quality)
                    .expect("a single always selects at least MIN_SINGLE_SONGS songs")
                    .name
            ),
            ReleaseType::Album => self.data_files.random_album_title(&mut rng),
        };

        // Apply stress cost (§A). Charged once, at the gate, rather than per
        // week: §B prices a session in money and *time*, and those are the two
        // currencies the design asks a room to cost. Wear accumulated inside
        // the studio is what §G's in-session events are for.
        self.player.stress = (self.player.stress + RECORD_STRESS_COST).min(constants::MAX_STRESS);

        self.log(format!(
            "🎙️ Booked the {} for '{}' — {} songs, {} week{}, ${} up front.",
            plan.studio.label(),
            title,
            plan.songs,
            plan.weeks,
            if plan.weeks == 1 { "" } else { "s" },
            plan.cost
        ));

        let session = RecordingInProgress {
            release_type,
            studio: plan.studio,
            weeks_total: plan.weeks,
            weeks_done: 0,
            booking_week: self.week,
            cost: plan.cost,
            title,
            songs: selected_songs,
            avg_song_quality,
            release_quality,
        };

        // The first week plays now; `turn.rs` drives the rest.
        self.resolve_recording_week(session);
        Ok(())
    }

    pub(in crate::game) fn action_record_single(
        &mut self,
        songs: u8,
        studio: RecordingStudio,
    ) -> Result<(), String> {
        self.book_recording(ReleaseType::Single, songs, studio)
    }

    pub(in crate::game) fn action_record_album(
        &mut self,
        songs: u8,
        studio: RecordingStudio,
    ) -> Result<(), String> {
        self.book_recording(ReleaseType::Album, songs, studio)
    }

    /// Play one week of a session in progress (design §F). Nothing is decided
    /// here — the record was settled at booking — so a week in the studio is
    /// simply a week the band is unavailable while the world keeps turning
    /// around it, which is exactly what §B asks a session to cost.
    pub(in crate::game) fn resolve_recording_week(&mut self, mut session: RecordingInProgress) {
        session.weeks_done += 1;
        if session.weeks_done >= session.weeks_total {
            self.settle_recording(session, true);
            return;
        }

        self.log(format!(
            "🎙️ In the {} cutting '{}', week {} of {}.",
            session.studio.label(),
            session.title,
            session.weeks_done,
            session.weeks_total
        ));
        self.ongoing_activity = Some(OngoingActivity::Recording(session));
    }

    /// Close out a session and clear it from the calendar (design §D + §F). A
    /// finished one deposits its `Master` in the vault, where it may sit as
    /// long as the player likes; one walked out of leaves nothing — the songs
    /// went into it and the studio bill was paid at the gate.
    pub(in crate::game) fn settle_recording(
        &mut self,
        session: RecordingInProgress,
        completed: bool,
    ) {
        self.ongoing_activity = None;

        if !completed {
            self.log(format!(
                "🛑 Walked out of the '{}' sessions after {} of {} weeks — the tapes are unusable, and the {} songs and the ${} they cost went with them.",
                session.title,
                session.weeks_done,
                session.weeks_total,
                session.songs.len(),
                session.cost
            ));
            return;
        }

        let master = Master {
            name: session.title,
            release_type: session.release_type,
            release_quality: session.release_quality,
            songs_involved_quality_avg: session.avg_song_quality,
            songs: session.songs,
            studio: session.studio,
            week_finished: self.week,
            genre: Some(self.band.genre.clone()),
        };
        self.log(format!(
            "🎙️ Recorded '{}' — {} songs cut at the {}, quality {}. It's in the vault until you put it out.",
            master.name,
            master.songs.len(),
            master.studio.label(),
            master.release_quality
        ));
        self.band.masters.push(master);
    }

    /// The vault: finished masters awaiting release, oldest first (design
    /// §D/§H). Indices into this slice are what `GameAction::ReleaseMaster`
    /// names.
    pub fn vault(&self) -> &[Master] {
        &self.band.masters
    }

    /// What it costs to put the master at `master_index` out right now
    /// (design §D). Pure — no rng, no mutation — so the vault UI can quote it
    /// live as the player changes pressing tier or channel, and it is the same
    /// call `action_release_master` charges from.
    pub fn quote_release(
        &self,
        master_index: usize,
        pressing: Option<usize>,
    ) -> Result<music::ReleaseQuote, String> {
        let master = self
            .band
            .masters
            .get(master_index)
            .ok_or_else(|| "There's no such record in the vault.".to_string())?;
        let (copies, pressing_cost) = self.plan_pressing(&master.release_type, pressing)?;
        let channel = self.current_distribution_channel;
        // `plan_distribution` is `Ok(0)` while signed (label deals ignore
        // channels entirely) and `Err` when fame has slipped below the chosen
        // channel's gate — the release is refused by name rather than silently
        // going out on a channel the band no longer qualifies for.
        let distribution_fee = self.plan_distribution(channel)?;
        Ok(music::ReleaseQuote {
            copies,
            pressing_cost,
            distribution_fee,
            channel,
        })
    }

    /// Put a finished master out (design §D — "a master, then a release").
    ///
    /// This is where `plan_pressing` and `plan_distribution` moved to, and the
    /// only place a `Release` is now born. A signed act's label still presses
    /// on release, as it always did (`plan_pressing` checks the deal first);
    /// an unsigned one buys its own run and its own reach here — and *only*
    /// here, because booking the session charged neither.
    ///
    /// Instant: no week is consumed (`turn.rs`).
    pub(in crate::game) fn action_release_master(
        &mut self,
        master_index: usize,
        pressing: Option<usize>,
    ) -> Result<(), String> {
        let quote = self.quote_release(master_index, pressing)?;
        if !self.player.can_afford(quote.total()) {
            if quote.distribution_fee > 0 {
                return Err(format!(
                    "Putting it out costs ${} — ${} to press {} copies and ${} for {}!",
                    quote.total(),
                    quote.pressing_cost,
                    quote.copies,
                    quote.distribution_fee,
                    quote.channel.label()
                ));
            }
            return Err(format!(
                "Pressing {} copies costs ${} — you can't cover it.",
                quote.copies, quote.pressing_cost
            ));
        }
        self.player.spend_money(quote.total());

        let master = self.band.masters.remove(master_index);
        let release_type = master.release_type;
        let signed = self.band.current_deal().is_some();

        let new_release = music::Release {
            id: self.next_release_id,
            name: master.name,
            release_type,
            release_quality: master.release_quality,
            // §D: the week that matters commercially is the week it came out,
            // not the week the tapes were finished. A master shelved for a
            // year charts, decays and ages from *here*.
            week_released: self.week,
            songs_involved_quality_avg: master.songs_involved_quality_avg,
            active_marketing: Vec::new(),
            marketing_level_achieved: 0,
            initial_sales_score: 0,
            total_income_generated: 0,
            genre: master.genre,
            copies_pressed: quote.copies,
            copies_sold: 0,
            peak_chart_position: None,
            singles_cut: 0,
            certified: 0,
            distribution_channel: if signed { None } else { Some(quote.channel) },
            // L-REACH (§C): freeze the label's *live* reach on the release (not
            // the value frozen in the deal at signing) so a record put out after
            // the label rose or fell carries its reach *now*; that frozen value
            // then drives this release's catalogue tail for good (§E-3). `None`
            // for an indie release; the deal's signing reach for an unseeded
            // label. Read here rather than at recording precisely because §D
            // makes release day a separate decision: shelve a master through a
            // label's collapse and the record ships with the reach it actually
            // ships with.
            label_market_reach: self.new_release_frozen_reach(),
        };
        let name = new_release.name.clone();
        self.just_released_music.push(new_release);
        self.next_release_id += 1;

        if quote.pressing_cost > 0 {
            self.log(format!(
                "📀 Put '{}' out — {} copies pressed for ${}, in the shops for {} weeks.",
                name, quote.copies, quote.pressing_cost, INITIAL_SALES_WINDOW_WEEKS
            ));
        } else {
            self.log(format!(
                "📀 Put '{}' out — the label presses {} copies, in the shops for {} weeks.",
                name, quote.copies, INITIAL_SALES_WINDOW_WEEKS
            ));
        }
        if quote.distribution_fee > 0 {
            self.log(format!(
                "🚚 {} takes ${} to carry '{}'.",
                quote.channel.label(),
                quote.distribution_fee,
                name
            ));
        }
        self.apply_label_promo();

        // §D moved this from recording to release, where it always belonged:
        // the buzz is the *announcement* of a new album, and since the split
        // there is nothing to announce until the record actually comes out.
        if release_type == ReleaseType::Album && self.timeline.is_album_era() {
            self.band.fame = (self.band.fame + 3).min(constants::MAX_FAME);
            self.log(
                "📈 It's an album-oriented era — the announcement alone earns you buzz (+3 fame).",
            );
        }

        Ok(())
    }
}
