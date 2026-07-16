//! Regional Top 100 charts overlay: region tabs, scroll to 100, movement.

use ratatui::{
    Frame,
    layout::{Constraint, Layout},
    style::{Color, Style},
    text::{Line, Span},
    widgets::{Block, Clear, Paragraph, Wrap},
};

use crate::game::world::ChartEntry;
use crate::ui::app::{App, Screen};

use super::super::{ACCENT, centered_rect};

/// Rows visible at once below the tab bar — enough for "top 10 at a
/// glance" with room to spare; scrolling reaches the rest of the 100.
const VISIBLE_ROWS: usize = 14;

pub(crate) fn draw_charts_modal(frame: &mut Frame, app: &App) {
    let Screen::Charts { region, scroll } = app.screen else {
        return;
    };
    let area = centered_rect(78, 68, frame.area());
    frame.render_widget(Clear, area);
    let block = Block::bordered()
        .title(" 📈 The Charts ")
        .title_style(Style::new().fg(Color::Yellow).bold())
        .title_bottom(" ←/→ region · ↑/↓ scroll · Esc close ");
    let inner = block.inner(area);
    frame.render_widget(block, area);

    let [tabs_area, list_area] =
        Layout::vertical([Constraint::Length(1), Constraint::Min(3)]).areas(inner);

    // The tab bar: the active region burns in the accent colour.
    let mut tab_spans = Vec::new();
    for (i, tab) in crate::game::world::ChartRegion::TAB_ORDER
        .iter()
        .enumerate()
    {
        if i > 0 {
            tab_spans.push(Span::raw("  "));
        }
        let style = if *tab == region {
            Style::new().fg(ACCENT).bold()
        } else {
            Style::new().fg(Color::DarkGray)
        };
        tab_spans.push(Span::styled(tab.label(), style));
    }
    frame.render_widget(Paragraph::new(Line::from(tab_spans)), tabs_area);

    let entries = app.charts_region_entries(region);
    if entries.is_empty() {
        let lines = vec![
            Line::from(""),
            Line::from(format!(
                "The {} board is quiet — nobody's record is moving this week.",
                region.label()
            ))
            .centered(),
        ];
        frame.render_widget(Paragraph::new(lines).wrap(Wrap { trim: false }), list_area);
        return;
    }

    let start = scroll.min(entries.len().saturating_sub(1));
    let end = (start + VISIBLE_ROWS).min(entries.len());

    let mut lines = Vec::with_capacity(end - start);
    for (idx, entry) in entries[start..end].iter().enumerate() {
        let position = start + idx + 1;
        lines.push(chart_row(entry, position));
    }
    frame.render_widget(Paragraph::new(lines), list_area);
}

/// One line: rank, movement, title, act, peak, weeks-on-chart. The
/// player's own records burn in the accent colour with a star.
fn chart_row(entry: &ChartEntry, position: usize) -> Line<'static> {
    let (style, marker) = if entry.is_player {
        (Style::new().fg(ACCENT).bold(), "★")
    } else {
        (Style::new().fg(Color::White), " ")
    };

    // Movement is derived from the stored peak: a record still at or
    // bettering its best-ever rank is climbing, one that has fallen off
    // its peak is sliding back down.
    let movement = if entry.weeks_on_chart == 0 {
        "NEW"
    } else if entry.peak_position == 0 || (position as u8) <= entry.peak_position {
        "↑"
    } else {
        "↓"
    };

    let weeks = format!(
        "{} wk{}",
        entry.weeks_on_chart,
        if entry.weeks_on_chart == 1 { "" } else { "s" }
    );
    let peak = if entry.peak_position == 0 {
        "—".to_string()
    } else {
        format!("peak #{}", entry.peak_position)
    };

    Line::from(vec![
        Span::styled(format!("  #{:<3}", position), style),
        Span::styled(format!("{:<4}", marker), style),
        Span::styled(format!("{:<5}", movement), style),
        Span::styled(format!("{:<26}", format!("'{}'", entry.title)), style),
        Span::styled(format!("{:<22}", entry.band_name), style),
        Span::styled(format!("{:<11}", peak), style),
        Span::styled(weeks, style),
    ])
}
