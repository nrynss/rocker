//! The five recording studios and the shape of a record (design §C, §A).
//!
//! The studio is the quality lever, and it is priced in both currencies —
//! money and time. Modelled on `TourRig` (`actions/live.rs`) down to the
//! `ordinal()`-into-const-tables pattern, and for the same reason: fame must
//! never re-price a booking, it may only gate which bookings are on offer.
//! Same studio + same songs = same quote, at any fame (design §C — the lesson
//! v0.7 §A learned about tours).

use serde::{Deserialize, Serialize};

use super::constants::*;
use super::music::ReleaseType;

/// The five rooms a record can be cut in (design §C table). The bonus shifts
/// the release-quality distribution, it does not multiply it, so a great band
/// in a demo room still beats a poor band at the world-class room.
///
/// Not persisted on its own, but `GameAction::RecordSingle`/`RecordAlbum`
/// carry it and `GameAction` derives `Serialize`/`Deserialize` as a whole, so
/// this must too.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum RecordingStudio {
    Home,
    DemoRoom,
    Regional,
    MajorCity,
    WorldClass,
}

impl Default for RecordingStudio {
    /// The neutral baseline (design §C): bonus 0, and a rate that reproduces
    /// roughly the pre-cycle bill — a player who always takes the default
    /// sees the old game.
    fn default() -> Self {
        RecordingStudio::DemoRoom
    }
}

impl RecordingStudio {
    /// In picker order, cheapest room to dearest.
    ///
    /// `allow(dead_code)`: S-MASTER (§D) moved the pressing picker onto the
    /// *release*, and the session panel that listed the open rooms went with
    /// it — the room axis lands for real in S-UI's format→songs→studio picker
    /// (§H), which is the caller this exists for. Same holding pattern
    /// `core.rs` uses for `OngoingActivity`'s activity-screen accessors, and
    /// the tests exercise it meanwhile.
    #[allow(dead_code)]
    pub const ALL: [RecordingStudio; 5] = [
        RecordingStudio::Home,
        RecordingStudio::DemoRoom,
        RecordingStudio::Regional,
        RecordingStudio::MajorCity,
        RecordingStudio::WorldClass,
    ];

    /// Index into the `STUDIO_*` const tables — also this studio's position
    /// in `ALL`, for picker navigation.
    pub fn ordinal(self) -> usize {
        match self {
            RecordingStudio::Home => 0,
            RecordingStudio::DemoRoom => 1,
            RecordingStudio::Regional => 2,
            RecordingStudio::MajorCity => 3,
            RecordingStudio::WorldClass => 4,
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            RecordingStudio::Home => "Home 4-track",
            RecordingStudio::DemoRoom => "Local demo room",
            RecordingStudio::Regional => "Regional studio",
            RecordingStudio::MajorCity => "Major city studio",
            RecordingStudio::WorldClass => "World-class room",
        }
    }

    /// Fame required to book this room at all (design §C table). A gate, never
    /// a price: see the module comment.
    pub fn fame_gate(self) -> u8 {
        STUDIO_FAME_GATE[self.ordinal()]
    }

    /// Added to release quality inside `calculate_release_quality`, before its
    /// clamp (design §C).
    pub fn quality_bonus(self) -> i8 {
        STUDIO_QUALITY_BONUS[self.ordinal()]
    }

    /// Studio time, per song, before the era's `recording_cost_modifier`.
    pub fn rate_per_song(self) -> u32 {
        STUDIO_RATE_PER_SONG[self.ordinal()]
    }

    /// Calendar weeks each song takes in this room (design §B). A better room
    /// costs time as well as money — that is what stops the top of the ladder
    /// from being a pure money check.
    pub fn weeks_per_song(self) -> f32 {
        STUDIO_WEEKS_PER_SONG[self.ordinal()]
    }
}

/// The song-count range a format accepts (design §A). The player chooses
/// within it; the action validates and names the shortfall.
pub fn song_count_bounds(release_type: &ReleaseType) -> (u8, u8) {
    match release_type {
        ReleaseType::Single => (MIN_SINGLE_SONGS, MAX_SINGLE_SONGS),
        ReleaseType::Album => (MIN_ALBUM_SONGS, MAX_ALBUM_SONGS),
    }
}

/// A format's name in prose, for the messages that have to name it — the
/// shortfall a booking is refused for, above all (design §A).
pub fn format_label(release_type: &ReleaseType) -> &'static str {
    match release_type {
        ReleaseType::Single => "single",
        ReleaseType::Album => "album",
    }
}

/// What the picker opens on (design §A table): a two-song 45, a ten-track
/// album. Both are the minimum-risk end of their format's range that still
/// reads as the format.
pub fn default_song_count(release_type: &ReleaseType) -> u8 {
    match release_type {
        ReleaseType::Single => MIN_SINGLE_SONGS,
        ReleaseType::Album => ALBUM_DEFAULT_SONGS,
    }
}

/// The up-front quote a record picker must show before booking (design §C —
/// "the quote comes first", the tour picker's pattern reused): what the studio
/// time costs, how many weeks the band is in there, and the quality band the
/// songs on hand would land in. Every field comes from the same deterministic
/// core `action_record_*` charges from, so the quote cannot drift from the
/// booking.
///
/// `allow(dead_code)`: the menu row reads `cost`, `weeks` and the projected
/// band, which is §C's promise; the rest describe the booking's *axes* and
/// land when S-UI's format→songs→studio picker (§H) lets the player move them.
#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct RecordingQuote {
    pub release_type: ReleaseType,
    pub studio: RecordingStudio,
    pub songs: u8,
    /// Studio time only. Pressing and distribution are still billed on top at
    /// the moment of recording; §D (S-MASTER) is what moves them to release.
    pub cost: i32,
    /// `ceil(songs × weeks_per_song)`, clamped to `[1, RECORDING_MAX_WEEKS]`.
    /// Quoted, not yet enforced — S-TIME builds the machinery that actually
    /// spends them (design §F).
    pub weeks: u8,
    /// Average songwriting quality of the best `songs` unreleased tracks — the
    /// input the projected band is computed from, and worth showing on its own
    /// since it is what "write more, then pick" improves.
    pub avg_song_quality: u8,
    /// Projected release quality at the pessimistic and optimistic ends of the
    /// recording roll, studio bonus included.
    pub quality_low: u8,
    pub quality_high: u8,
}
