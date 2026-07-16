//! Keyboard input event handlers for each screen family, implementing `App` input helpers.

mod deals;
mod file;
mod lifestyle;
mod main;
mod marketing;
mod pickers;
mod setup;

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
