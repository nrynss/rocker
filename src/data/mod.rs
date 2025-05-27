// This module handles game data, save/load functionality, and configuration

// Game constants
pub mod constants {
    pub const MAX_HEALTH: u8 = 100;
    pub const MAX_ENERGY: u8 = 100;
    pub const MAX_STRESS: u8 = 100;
    pub const MAX_FAME: u8 = 100;
    pub const MAX_SKILL: u8 = 100;

    pub const WEEKS_PER_YEAR: u32 = 52;

    // Cost constants
    pub const SINGLE_RECORDING_COST: i32 = 100;
    pub const ALBUM_RECORDING_BASE_COST: i32 = 1000;
    pub const DOCTOR_VISIT_COST: i32 = 50;
    pub const EQUIPMENT_REPAIR_COST_RANGE: (i32, i32) = (50, 200);

    // Game progression constants
    pub const MIN_ALBUM_SONGS: u8 = 8;
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
