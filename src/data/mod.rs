// This module handles game data, save/load functionality, and configuration

// Game constants
pub mod constants {
    pub const MAX_HEALTH: u8 = 100;
    pub const MAX_STRESS: u8 = 100;
    pub const MAX_FAME: u8 = 100;
    pub const MAX_SKILL: u8 = 100;

    pub const WEEKS_PER_YEAR: u32 = 52;
    pub const STARTING_YEAR: u32 = 1970;

    // Cost constants.
    //
    // L-SCALE (v0.8 §B): ×3 to the one honest era-real money scale — every
    // dollar in the game triples in lockstep so ratios hold while magnitude
    // lifts. (was 100 / 1000 / 50.)
    //
    // S-RECORD (v0.9 §A): the flat `SINGLE_RECORDING_COST` (300) /
    // `ALBUM_RECORDING_BASE_COST` (3000) pair retired here — a record's studio
    // bill is now `studio rate/song × songs × era modifier`, so what you are
    // making and where you make it both enter the price. See
    // `STUDIO_RATE_PER_SONG` in `game/constants.rs`.
    pub const DOCTOR_VISIT_COST: i32 = 150;

    // Game progression constants
    //
    // S-RECORD (v0.9 §A): song counts are a bounded player choice, and the two
    // formats have genuinely different shapes — a single is an A-side plus a
    // B-side (2-3), an album is 8-14. The *minima* are decided design, not
    // [tune]: below two songs it is not a single, and eight has always been
    // what an album means here.
    pub const MIN_SINGLE_SONGS: u8 = 2;
    pub const MAX_SINGLE_SONGS: u8 = 3;
    pub const MIN_ALBUM_SONGS: u8 = 8;
    pub const MAX_ALBUM_SONGS: u8 = 14;
    /// Where the album picker opens (design §A table). A single opens at its
    /// minimum — two songs *is* the format — so it needs no twin.
    pub const ALBUM_DEFAULT_SONGS: u8 = 10;
    pub const ROCKSTAR_FAME_THRESHOLD: u8 = 90;
    pub const ROCKSTAR_ALBUM_THRESHOLD: u8 = 5;
    pub const CRITICAL_HEALTH_THRESHOLD: u8 = 20;
}

// Utility functions for game data
pub fn format_money(amount: i32) -> String {
    if amount >= 0 {
        format!("${}", amount)
    } else {
        format!("-${}", amount.abs())
    }
}

pub fn calculate_weeks_to_years_months(weeks: u32) -> String {
    let years = weeks / constants::WEEKS_PER_YEAR;
    let remaining_weeks = weeks % constants::WEEKS_PER_YEAR;
    let months = remaining_weeks / 4;
    let final_weeks = remaining_weeks % 4;

    if years > 0 {
        format!("{} year(s), {} month(s)", years, months)
    } else if months > 0 {
        format!("{} month(s), {} week(s)", months, final_weeks)
    } else {
        format!("{} week(s)", final_weeks)
    }
}
