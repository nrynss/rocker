//! Tour / gig report overlay (design §B — the tour report). One row per
//! resolved show (a one-off gig is the same report with a single row),
//! scrollable so a 4-week, 20-show tour stays readable, plus a summary
//! footer.

use ratatui::{
    Frame,
    layout::{Constraint, Layout},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Clear, List, ListItem, ListState, Paragraph, Wrap},
};

use crate::data::format_money;
use crate::ui::app::{App, Screen};

use super::super::{ACCENT, centered_rect};

/// A verdict's display color, matching the reception-quality story the
/// tour log already tells (§B — Verdicts).
fn verdict_color(verdict: &str) -> Color {
    match verdict {
        "transcendent" => Color::Magenta,
        "great" => Color::Green,
        "solid" => Color::Yellow,
        _ => Color::Red,
    }
}

/// Keep long synthesized tour-venue names ("Venue (City, Region)") from
/// blowing out the row width.
fn truncate(s: &str, max: usize) -> String {
    if s.chars().count() <= max {
        s.to_string()
    } else {
        let mut out: String = s.chars().take(max.saturating_sub(1)).collect();
        out.push('…');
        out
    }
}

pub(crate) fn draw_tour_report_modal(frame: &mut Frame, app: &App) {
    let Screen::TourReport { scroll } = app.screen else {
        return;
    };
    let area = centered_rect(88, 80, frame.area());
    frame.render_widget(Clear, area);

    let block = Block::bordered()
        .title(" 🚌 Tour Report ")
        .title_style(Style::new().fg(Color::Yellow).bold())
        .title_bottom(" ↑↓ scroll · Esc close ");
    let inner = block.inner(area);
    frame.render_widget(block, area);

    let Some(report) = &app.game.last_tour_report else {
        let lines = vec![
            Line::from(""),
            Line::from("No tour report yet — play a show.").centered(),
        ];
        frame.render_widget(Paragraph::new(lines).wrap(Wrap { trim: false }), inner);
        return;
    };

    let [header_area, list_area, footer_area] = Layout::vertical([
        Constraint::Length(1),
        Constraint::Min(3),
        Constraint::Length(4),
    ])
    .areas(inner);

    frame.render_widget(
        Paragraph::new(Line::styled(
            format!(
                "  {:<6}{:<34}{:<14}{:<11}{:<13}{}",
                "Week", "Venue (city)", "Verdict", "Reception", "Sold/Cap", "Take"
            ),
            Style::new().fg(Color::Cyan).bold(),
        )),
        header_area,
    );

    let items: Vec<ListItem> = report
        .rows
        .iter()
        .map(|row| {
            let verdict_style = Style::new().fg(verdict_color(&row.verdict));
            ListItem::new(Line::from(vec![
                Span::raw(format!("  {:<6}", row.week)),
                Span::raw(format!("{:<34}", truncate(&row.venue_name, 33))),
                Span::styled(format!("{:<14}", row.verdict), verdict_style),
                Span::raw(format!("{:<11}", row.reception)),
                Span::raw(format!(
                    "{:<13}",
                    format!("{}/{}", row.attendance, row.capacity)
                )),
                Span::styled(format_money(row.take as i32), Style::new().fg(Color::Green)),
            ]))
        })
        .collect();

    let list = List::new(items).highlight_style(Style::new().add_modifier(Modifier::REVERSED));
    let selected = scroll.min(report.rows.len().saturating_sub(1));
    let mut state = ListState::default().with_selected(Some(selected));
    frame.render_stateful_widget(list, list_area, &mut state);

    let verdict_line = if report.went_very_well() {
        Line::styled(
            "  The tour went very well — spirits are high.",
            Style::new().fg(ACCENT).bold(),
        )
    } else {
        Line::from("  A mixed run — some nights landed, some didn't.")
    };

    let footer = vec![
        Line::from(format!(
            "  Shows played {}  ·  Avg reception {}",
            report.rows.len(),
            report.avg_reception
        )),
        verdict_line,
        Line::from(format!(
            "  Total gross {}  ·  Fame gained +{}",
            format_money(report.total_gross as i32),
            report.fame_gained
        )),
    ];
    frame.render_widget(Paragraph::new(footer), footer_area);
}
