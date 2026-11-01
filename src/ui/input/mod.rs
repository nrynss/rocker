//! Keyboard input event handlers for each screen family, implementing `App` input helpers.

mod deals;
mod file;
mod lifestyle;
mod main;
mod marketing;
mod pickers;
mod setup;

use crate::ui::app::App;

impl App {
    /// The running commitment, named for a sentence ("a tour", "a break"), or
    /// `None` when the band is free.
    ///
    /// Every menu row that opens a *booking* picker — Record, Play a Gig, Go
    /// on Tour — asks this before opening. While an activity runs,
    /// `process_turn` ignores whatever the player submits and plays the
    /// activity's next week instead (design §F), so a picker opened mid-tour
    /// would quote a booking, take the player's Enter, then silently drop the
    /// booking and spend the week on the tour. Refusing at the door is the
    /// only honest answer; the row's hotkey and Enter both route through
    /// `activate`, so a guard there covers every entry point.
    fn busy_with(&self) -> Option<String> {
        self.game
            .ongoing_activity()
            .map(|activity| format!("a {}", activity.label().to_lowercase()))
    }
}

/// Cycle a 0-based selection index one step over `count` items, wrapping at
/// both ends: `forward` (Down/Right) advances, otherwise (Up/Left) it steps
/// back. Shared by every picker's ↑↓/←→ navigation. `count` must be non-zero,
/// which every caller guarantees (fixed-length tables, or an early return on
/// an empty list).
fn cycle_index(current: usize, count: usize, forward: bool) -> usize {
    if forward {
        (current + 1) % count
    } else {
        current.checked_sub(1).unwrap_or(count - 1)
    }
}
