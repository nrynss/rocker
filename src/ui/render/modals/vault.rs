//! The vault (design §D/§H): finished masters awaiting release.
//!
//! §D made a record a two-step thing — a session yields a master the band
//! owns, and a separate decision presses and ships it. This is where that
//! second decision starts, and it is the only screen in the game that shows
//! a record which exists but is not out.

use ratatui::{
    Frame,
    layout::{Constraint, Layout},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Clear, List, ListItem, ListState, Paragraph, Wrap},
};

use crate::data::format_money;
use crate::game::PRESSING_TIERS;
use crate::ui::app::{App, Screen};

use super::super::centered_rect;

pub(crate) fn draw_vault_modal(frame: &mut Frame, app: &App) {
    let Screen::Vault { selected } = app.screen else {
        return;
    };
    let masters = app.game.vault();

    let area = centered_rect(80, 60, frame.area());
    frame.render_widget(Clear, area);
    let block = Block::bordered()
        .title(" 🎚️ The Vault — finished, not yet out ")
        .title_bottom(" Enter press & release · Esc close ");
    let inner = block.inner(area);
    frame.render_widget(block, area);

    if masters.is_empty() {
        frame.render_widget(
            Paragraph::new(vec![
                Line::from("Nothing finished."),
                Line::styled(
                    "Book a session and play it out — the master lands here when it wraps.",
                    Style::new().fg(Color::DarkGray),
                ),
            ])
            .wrap(Wrap { trim: false }),
            inner,
        );
        return;
    }

    let [list_area, detail_area] =
        Layout::vertical([Constraint::Min(4), Constraint::Length(7)]).areas(inner);

    let items: Vec<ListItem> = masters
        .iter()
        .map(|master| {
            let kind = crate::game::studios::format_label(&master.release_type);
            ListItem::new(Line::from(vec![
                Span::styled(format!("{:<30}", master.name), Style::new().bold()),
                Span::styled(format!("  {:<7}", kind), Style::new().fg(Color::Cyan)),
                Span::styled(format!("  {:>2} songs", master.songs.len()), Style::new()),
                Span::styled(
                    format!("  quality {:>3}", master.release_quality),
                    Style::new(),
                ),
                Span::styled(
                    format!("  finished wk {}", master.week_finished),
                    Style::new().fg(Color::DarkGray),
                ),
            ]))
        })
        .collect();
    let list = List::new(items)
        .block(Block::default().title("Masters"))
        .highlight_style(Style::new().add_modifier(Modifier::REVERSED));
    let mut state = ListState::default().with_selected(Some(selected.min(masters.len() - 1)));
    frame.render_stateful_widget(list, list_area, &mut state);

    // What release day would cost this master at the cheapest run — the same
    // `quote_release` the action charges from, so the vault can never promise
    // a bill the release doesn't send.
    let index = selected.min(masters.len() - 1);
    let master = &masters[index];
    let detail: Vec<Line> = match app.game.quote_release(index, Some(0)) {
        Ok(quote) => vec![
            Line::styled("Release", Style::new().bold()),
            Line::from(format!(
                "  Cut at the {} — {} songs, songs avg {}, quality {}.",
                master.studio.label(),
                master.songs.len(),
                master.songs_involved_quality_avg,
                master.release_quality
            )),
            if quote.pressing_cost > 0 {
                Line::from(format!(
                    "  From {} ({} copies) for {}{}",
                    PRESSING_TIERS[0].0,
                    quote.copies,
                    format_money(quote.pressing_cost),
                    if quote.distribution_fee > 0 {
                        format!(
                            ", plus {} for {}",
                            format_money(quote.distribution_fee),
                            quote.channel.label()
                        )
                    } else {
                        String::new()
                    }
                ))
            } else {
                Line::from(format!(
                    "  Your label presses {} copies — nothing out of pocket.",
                    quote.copies
                ))
            },
            if app.game.player.can_afford(quote.total()) {
                Line::styled("  Ready to press.", Style::new().fg(Color::Green))
            } else {
                Line::styled(
                    format!(
                        "  Not enough cash — you have {}.",
                        format_money(app.game.player.money)
                    ),
                    Style::new().fg(Color::Red),
                )
            },
            Line::styled(
                "  A master keeps. Sitting on it to release into a better market is a real move.",
                Style::new().fg(Color::DarkGray),
            ),
        ],
        Err(message) => vec![
            Line::styled("Release", Style::new().bold()),
            Line::styled(format!("  {message}"), Style::new().fg(Color::Red)),
        ],
    };
    frame.render_widget(
        Paragraph::new(detail).wrap(Wrap { trim: false }),
        detail_area,
    );
}
