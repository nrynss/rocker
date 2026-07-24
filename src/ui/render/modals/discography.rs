//! The band's discography — every record with its commercial performance.
//!
//! v0.8 L-UI-1 (explicit user ask: "there is no way to see the sales of each
//! record of the band right now"). A read-only browser that lists every
//! record the band has out (and anything dropping soon) with the numbers that
//! tell its commercial story: copies sold, income earned, chart peak, and its
//! certification badge. Scrolls when the catalogue outgrows the modal.

use ratatui::{
    Frame,
    style::{Color, Style},
    text::{Line, Span},
    widgets::{Block, Clear, Paragraph, Wrap},
};

use crate::game::music::{Release, ReleaseType};
use crate::ui::app::{App, Screen};

use super::super::{centered_rect, group_thousands};

/// Human label for a release kind (`ReleaseType` has no display impl of its
/// own, so this UI-local mapping stands in — a read-only view helper).
fn type_label(t: ReleaseType) -> &'static str {
    match t {
        ReleaseType::Single => "Single",
        ReleaseType::Album => "Album",
    }
}

/// Certification badge for a release (§D levels): silver/gold/platinum and
/// multi-platinum. Mirrors the marketing modal's badge vocabulary so a record
/// reads the same wherever it is shown; empty string when uncertified.
fn cert_badge(certified: u8) -> String {
    match certified {
        0 => String::new(),
        1 => "🥈".to_string(),
        2 => "🥇".to_string(),
        3 => "💠".to_string(),
        n => format!("💠×{}", n - 2),
    }
}

/// Copies sold, formatted for legibility: comma-grouped up to a million
/// (`48,200`), then abbreviated (`1.2M`) for the platinum-scale sellers.
fn format_copies(copies: u32) -> String {
    if copies >= 1_000_000 {
        format!("{:.1}M", copies as f64 / 1_000_000.0)
    } else {
        group_thousands(u64::from(copies))
    }
}

pub(crate) fn draw_discography_modal(frame: &mut Frame, app: &App) {
    let Screen::Discography { scroll } = app.screen else {
        return;
    };
    let area = centered_rect(80, 72, frame.area());
    frame.render_widget(Clear, area);
    let block = Block::bordered()
        .title(" 💿 Discography ")
        .title_style(Style::new().fg(Color::Yellow).bold())
        .title_bottom(" ↑/↓ scroll · Esc close ");
    let inner = block.inner(area);
    frame.render_widget(block, area);

    let records = app.discography_rows();
    if records.is_empty() {
        let lines = vec![
            Line::from(""),
            Line::from("No records yet — cut a single or album, then come back to watch it sell.")
                .centered(),
        ];
        frame.render_widget(Paragraph::new(lines).wrap(Wrap { trim: false }), inner);
        return;
    }

    // Two lines per record (a title line and a sales line), so a record is
    // never split across the scroll boundary — window by record.
    let per_record = 2usize;
    let visible = (inner.height as usize / per_record).max(1);
    let start = scroll.min(records.len().saturating_sub(1));
    let end = (start + visible).min(records.len());

    let mut lines = Vec::with_capacity((end - start) * per_record);
    for (release, released) in &records[start..end] {
        lines.push(title_line(release, *released));
        lines.push(sales_line(release));
    }
    frame.render_widget(Paragraph::new(lines), inner);
}

/// Line one: the record's name, its kind, its certification badge, and — for
/// something not yet in stores — a "dropping soon" tag.
fn title_line(release: &Release, released: bool) -> Line<'static> {
    let mut spans = vec![
        Span::styled(
            format!("'{}'", release.name),
            Style::new().fg(Color::White).bold(),
        ),
        Span::styled(
            format!("  {}", type_label(release.release_type)),
            Style::new().fg(Color::DarkGray),
        ),
    ];
    let badge = cert_badge(release.certified);
    if !badge.is_empty() {
        spans.push(Span::raw(format!("  {}", badge)));
    }
    if !released {
        spans.push(Span::styled(
            "  · dropping soon",
            Style::new().fg(Color::Yellow),
        ));
    }
    Line::from(spans)
}

/// Line two: the commercial story — copies sold, income earned, chart peak.
fn sales_line(release: &Release) -> Line<'static> {
    let peak = match release.peak_chart_position {
        Some(pos) => format!("peak #{}", pos),
        None => "never charted".to_string(),
    };
    let sep = || Span::styled("  ·  ", Style::new().fg(Color::DarkGray));
    Line::from(vec![
        Span::raw("    "),
        Span::styled(
            format!("{} copies", format_copies(release.copies_sold)),
            Style::new().fg(Color::White),
        ),
        sep(),
        Span::styled(
            format!(
                "${} gross",
                group_thousands(u64::from(release.total_income_generated))
            ),
            Style::new().fg(Color::Green),
        ),
        sep(),
        Span::styled(peak, Style::new().fg(Color::Cyan)),
    ])
}
