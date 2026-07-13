//! Weekly top-10 charts overlay.

use ratatui::{
    Frame,
    style::{Color, Style},
    text::{Line, Span},
    widgets::{Block, Clear, Paragraph, Wrap},
};

use crate::ui::app::App;

use super::super::{ACCENT, centered_rect};
pub(crate) fn draw_charts_modal(frame: &mut Frame, app: &App) {
    let area = centered_rect(72, 60, frame.area());
    frame.render_widget(Clear, area);
    let block = Block::bordered()
        .title(" 📈 This Week's Top 10 ")
        .title_style(Style::new().fg(Color::Yellow).bold())
        .title_bottom(" Esc close ");
    let inner = block.inner(area);
    frame.render_widget(block, area);

    let charts = &app.game.world.charts;
    if charts.is_empty() {
        let lines = vec![
            Line::from(""),
            Line::from("The charts are quiet — nobody's record is moving this week.").centered(),
            Line::from("Put something out and claim a spot.").centered(),
        ];
        frame.render_widget(Paragraph::new(lines).wrap(Wrap { trim: false }), inner);
        return;
    }

    let mut lines = vec![Line::from("")];
    for (idx, entry) in charts.iter().enumerate() {
        // Your own records burn in the accent colour with a star.
        let (style, marker) = if entry.is_player {
            (Style::new().fg(ACCENT).bold(), "★")
        } else {
            (Style::new().fg(Color::White), " ")
        };
        let weeks = if entry.weeks_on_chart == 0 {
            "NEW".to_string()
        } else {
            format!(
                "{} wk{}",
                entry.weeks_on_chart,
                if entry.weeks_on_chart == 1 { "" } else { "s" }
            )
        };
        lines.push(Line::from(vec![
            Span::styled(format!("  #{:<3}", idx + 1), style),
            Span::styled(format!("{} ", marker), style),
            Span::styled(format!("{:<28}", format!("'{}'", entry.title)), style),
            Span::styled(format!("{:<24}", entry.band_name), style),
            Span::styled(weeks, style),
        ]));
    }
    frame.render_widget(Paragraph::new(lines), inner);
}
