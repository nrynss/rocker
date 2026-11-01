//! The "walk out?" confirmation for a running commitment (design §F, #41).

use ratatui::{
    Frame,
    layout::Rect,
    style::{Color, Style},
    text::{Line, Span},
    widgets::{Block, Clear, Paragraph, Wrap},
};

use crate::data::format_money;
use crate::game::OngoingActivity;
use crate::ui::app::App;

/// What walking out keeps and what it forfeits, in the player's terms.
///
/// Mirrors the settle-as-abandoned paths (`settle_recording`, `settle_tour`,
/// `settle_support_tour`, `settle_break` with `completed: false`): the
/// confirmation must promise exactly what the walk-out then does.
pub(crate) fn abandon_terms(activity: &OngoingActivity) -> (String, String) {
    fn weeks(n: u8) -> String {
        format!("{n} week{}", if n == 1 { "" } else { "s" })
    }
    match activity {
        OngoingActivity::Recording(session) => (
            "nothing — half-finished tapes are unusable".to_string(),
            format!(
                "the {} song{} and the {} session, with no master",
                session.songs.len(),
                if session.songs.len() == 1 { "" } else { "s" },
                format_money(session.cost)
            ),
        ),
        OngoingActivity::Tour(tour) => {
            let taken: u32 = tour.rows.iter().map(|row| row.take).sum();
            (
                format!(
                    "{} show{} played and {} taken",
                    tour.rows.len(),
                    if tour.rows.len() == 1 { "" } else { "s" },
                    format_money(taken as i32)
                ),
                format!(
                    "the last {} of shows, the tour's fame (national and regional), and the {} booking",
                    weeks(tour.weeks_total.saturating_sub(tour.weeks_done)),
                    format_money(tour.cost)
                ),
            )
        }
        OngoingActivity::SupportTour(support) => (
            format!("the {} guarantee", format_money(support.pay as i32)),
            format!("the exposure (fame +{} on offer)", support.fame_gain),
        ),
        OngoingActivity::Break(rest) => (
            format!("the {} off so far", weeks(rest.weeks_done)),
            "the end-of-break recharge (stress reset, health, happiness, creativity)".to_string(),
        ),
    }
}

pub(crate) fn draw_confirm_abandon_modal(frame: &mut Frame, app: &App) {
    let Some(activity) = app.game.ongoing_activity() else {
        return;
    };
    let (keep, forfeit) = abandon_terms(activity);

    // A fixed box rather than a percentage: at this width the headline and
    // the forfeit wrap to two lines at most each, and twelve rows hold that
    // worst case, so the `[Y]`/`[N]` hint can never be clipped off the
    // bottom of an 80x24 terminal (#41's 80-column budget).
    let screen = frame.area();
    let width = screen.width.min(64);
    let height = screen.height.min(12);
    let area = Rect {
        x: screen.x + (screen.width - width) / 2,
        y: screen.y + (screen.height - height) / 2,
        width,
        height,
    };
    frame.render_widget(Clear, area);
    let block = Block::bordered()
        .title(" 🛑 Walk out? ")
        .title_style(Style::new().fg(Color::Red).bold());
    let inner = block.inner(area);
    frame.render_widget(block, area);

    let lines = vec![
        Line::from(""),
        Line::from(vec![
            Span::styled(
                format!(" {}: ", activity.label()),
                Style::new().fg(Color::Yellow).bold(),
            ),
            Span::raw(format!("{} ({})", activity.headline(), activity.progress())),
        ]),
        Line::from(""),
        Line::from(vec![
            Span::styled(" You keep:    ", Style::new().fg(Color::Green)),
            Span::raw(keep),
        ]),
        Line::from(vec![
            Span::styled(" You forfeit: ", Style::new().fg(Color::Red)),
            Span::raw(forfeit),
        ]),
        Line::from(""),
        Line::styled(
            " [Y] walk out · [N]/[Esc] carry on",
            Style::new().fg(Color::DarkGray),
        ),
    ];
    frame.render_widget(Paragraph::new(lines).wrap(Wrap { trim: false }), inner);
}
