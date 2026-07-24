//! The record-label ledger browser (v0.8 L-UI-1, design §H).
//!
//! A read-only list of every label in the world: its tier, market reach,
//! roster size, and a read on its balance sheet. The player's own label (the
//! one behind their signed deal) shows its *exact* capital; every other label
//! shows only a qualitative band — never a rival's spied figure (§H). Scrolls
//! when the industry outgrows the modal.

use ratatui::{
    Frame,
    style::{Color, Style},
    text::{Line, Span},
    widgets::{Block, Clear, Paragraph, Wrap},
};

use crate::game::world::{Label, LabelTier};
// The qualitative capital bands (§H — never expose a rival's exact figure)
// key off the game's own tier capital thresholds, aliased here so a retune of
// the tier floors can never silently desync the ledger's "steady"/"flush"
// bands from the tier logic (single source of truth, review follow-up).
use crate::game::{
    TIER_INDEPENDENT_MIN_CAPITAL as CAPITAL_STEADY_FLOOR,
    TIER_MAJOR_MIN_CAPITAL as CAPITAL_FLUSH_FLOOR,
};
use crate::ui::app::{App, Screen};

use super::super::{ACCENT, centered_rect, group_thousands};

/// Human label for a label's tier (`LabelTier` has no display impl of its
/// own, so this UI-local mapping stands in). `pub(super)` so the deal picker
/// (L-UI-2) annotates an offer with its label's *current* tier from the same
/// vocabulary.
pub(super) fn tier_label(tier: LabelTier) -> &'static str {
    match tier {
        LabelTier::Boutique => "Boutique",
        LabelTier::Independent => "Independent",
        LabelTier::Major => "Major",
    }
}

/// The qualitative capital band for a rival label, from the sign and
/// magnitude of its balance sheet (§H). Word plus the colour it reads in.
/// `pub(super)` so the deal picker (L-UI-2) reads an offering label's health
/// with the same never-a-spied-exact-figure wording used here.
pub(super) fn capital_band(capital: i64) -> (&'static str, Color) {
    if capital < 0 {
        ("in the red", Color::Red)
    } else if capital < CAPITAL_STEADY_FLOOR {
        ("getting by", Color::Yellow)
    } else if capital < CAPITAL_FLUSH_FLOOR {
        ("steady", Color::Cyan)
    } else {
        ("flush", Color::Green)
    }
}

/// Exact capital with a leading sign, comma-grouped (`$1,234,567` /
/// `-$4,200`). Only ever shown for the player's own label.
fn format_capital(capital: i64) -> String {
    if capital < 0 {
        format!("-${}", group_thousands(capital.unsigned_abs()))
    } else {
        format!("${}", group_thousands(capital as u64))
    }
}

pub(crate) fn draw_labels_modal(frame: &mut Frame, app: &App) {
    let Screen::Labels { scroll } = app.screen else {
        return;
    };
    let area = centered_rect(78, 72, frame.area());
    frame.render_widget(Clear, area);
    let block = Block::bordered()
        .title(" 🏢 Record Labels ")
        .title_style(Style::new().fg(Color::Yellow).bold())
        .title_bottom(" ↑/↓ scroll · Esc close ");
    let inner = block.inner(area);
    frame.render_widget(block, area);

    // `labels` is a BTreeMap keyed by id (majors, then independents, then
    // boutiques, in worldgen order), so iteration order is stable.
    let labels: Vec<&Label> = app.game.world.labels.values().collect();
    if labels.is_empty() {
        // A pre-0.8 save carries no labels at all (the map is serde-default).
        let lines = vec![
            Line::from(""),
            Line::from("No labels in this world — this save predates the label industry.")
                .centered(),
        ];
        frame.render_widget(Paragraph::new(lines).wrap(Wrap { trim: false }), inner);
        return;
    }

    let player_label_id = app.player_label_id();

    // Two lines per label (a name line and a detail line): window by label so
    // an entry is never split across the scroll boundary.
    let per_label = 2usize;
    let visible = (inner.height as usize / per_label).max(1);
    let start = scroll.min(labels.len().saturating_sub(1));
    let end = (start + visible).min(labels.len());

    let mut lines = Vec::with_capacity((end - start) * per_label);
    for label in &labels[start..end] {
        let is_player = player_label_id == Some(label.id);
        lines.push(name_line(label, is_player));
        lines.push(detail_line(label, is_player));
    }
    frame.render_widget(Paragraph::new(lines), inner);
}

/// Line one: a marker, the label's name, and its tier. The player's own label
/// burns in the accent colour with a star, echoing the charts modal.
fn name_line(label: &Label, is_player: bool) -> Line<'static> {
    let (style, marker) = if is_player {
        (Style::new().fg(ACCENT).bold(), "★ ")
    } else {
        (Style::new().fg(Color::White).bold(), "  ")
    };
    Line::from(vec![
        Span::styled(marker, style),
        Span::styled(label.name.clone(), style),
        Span::styled(
            format!("  [{}]", tier_label(label.tier)),
            Style::new().fg(Color::DarkGray),
        ),
    ])
}

/// Line two: market reach, roster size, and the capital read — exact for the
/// player's own label, a qualitative band for everyone else (§H).
fn detail_line(label: &Label, is_player: bool) -> Line<'static> {
    let roster = label.roster.len();
    let capital_span = if is_player {
        Span::styled(
            format!("your label · {}", format_capital(label.capital)),
            Style::new().fg(Color::Green).bold(),
        )
    } else {
        let (word, color) = capital_band(label.capital);
        Span::styled(word.to_string(), Style::new().fg(color))
    };
    let sep = || Span::styled("  ·  ", Style::new().fg(Color::DarkGray));
    Line::from(vec![
        Span::raw("    "),
        Span::styled(
            format!("reach {}", label.market_reach),
            Style::new().fg(Color::White),
        ),
        sep(),
        Span::styled(
            format!("{} act{}", roster, if roster == 1 { "" } else { "s" }),
            Style::new().fg(Color::White),
        ),
        sep(),
        capital_span,
    ])
}
