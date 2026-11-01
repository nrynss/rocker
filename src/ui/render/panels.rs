use ratatui::{
    Frame,
    layout::{Constraint, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, List, ListItem, ListState, Paragraph, Wrap},
};

use crate::data::{calculate_weeks_to_years_months, constants, format_money};
use crate::game::Game;

use crate::ui::app::{App, LogKind};

use super::{ACCENT, gauge, scale_color};

pub(super) fn draw_header(frame: &mut Frame, game: &Game, area: Rect) {
    let weeks_elapsed = game.week.saturating_sub(1);
    let year = constants::STARTING_YEAR + weeks_elapsed / constants::WEEKS_PER_YEAR;
    let week_in_year = weeks_elapsed % constants::WEEKS_PER_YEAR + 1;
    let era = game.timeline.get_current_era().era_name.clone();

    let title = Line::from(vec![
        Span::styled("🎸 ROCKER", Style::new().fg(ACCENT).bold()),
        Span::raw("  ·  "),
        Span::styled(
            format!("Week {} of {}", week_in_year, year),
            Style::new().bold(),
        ),
        Span::raw("  ·  "),
        Span::styled(era, Style::new().fg(Color::Cyan)),
    ]);
    let subtitle = Line::from(vec![
        Span::styled(game.player.name.clone(), Style::new().fg(Color::Yellow)),
        Span::raw(" fronting "),
        Span::styled(
            format!("'{}'", game.band.name),
            Style::new().fg(Color::Magenta).bold(),
        ),
        Span::raw(format!(
            "  ·  career: {}",
            calculate_weeks_to_years_months(game.week)
        )),
    ]);

    frame.render_widget(
        Paragraph::new(vec![title, subtitle]).block(Block::bordered()),
        area,
    );
}

pub(super) fn draw_player_panel(frame: &mut Frame, game: &Game, area: Rect) {
    let block = Block::bordered().title(" 💰 You ");
    let inner = block.inner(area);
    frame.render_widget(block, area);

    let [
        money_area,
        health_area,
        stress_area,
        happiness_area,
        creativity_area,
        warn_area,
    ] = Layout::vertical([
        Constraint::Length(1),
        Constraint::Length(1),
        Constraint::Length(1),
        Constraint::Length(1),
        Constraint::Length(1),
        Constraint::Min(0),
    ])
    .areas(inner);

    let money_style = if game.player.money < 0 {
        Style::new().fg(Color::Red).bold()
    } else {
        Style::new().fg(Color::Green).bold()
    };
    frame.render_widget(
        Paragraph::new(Line::from(vec![
            Span::raw("Money  "),
            Span::styled(format_money(game.player.money), money_style),
        ])),
        money_area,
    );

    frame.render_widget(
        gauge(
            format!("Health {}%", game.player.health),
            game.player.health,
            scale_color(game.player.health, true),
        ),
        health_area,
    );
    frame.render_widget(
        gauge(
            format!("Stress {}%", game.player.stress),
            game.player.stress,
            scale_color(game.player.stress, false),
        ),
        stress_area,
    );
    frame.render_widget(
        gauge(
            format!("Happiness {}%", game.player.happiness),
            game.player.happiness,
            scale_color(game.player.happiness, true),
        ),
        happiness_area,
    );
    frame.render_widget(
        gauge(
            format!("Creativity {}%", game.player.creativity),
            game.player.creativity,
            scale_color(game.player.creativity, true),
        ),
        creativity_area,
    );

    let mut warnings: Vec<Line> = Vec::new();
    if game.player.health <= constants::CRITICAL_HEALTH_THRESHOLD {
        // Mid-activity the doctor is not an option — the Visit Doctor row
        // only continues the commitment (§F, #41) — so the advice has to be
        // what actually helps: walk out of a tour or a session, or let a
        // break run out, since its recovery lands on the last week. Two
        // short lines, because this panel is ~18 columns wide on an 80-column
        // terminal and the old one-liner was cut off mid-advice.
        let advice = match game.ongoing_activity() {
            None => "  see a doctor (9)",
            Some(crate::game::OngoingActivity::Break(_)) => "  rest it out",
            Some(_) => "  walk out (X)",
        };
        let critical = Style::new().fg(Color::Red).bold();
        warnings.push(Line::styled("⚠ CRITICAL health", critical));
        warnings.push(Line::styled(advice, critical));
    }
    if game.player.is_addicted() {
        warnings.push(Line::styled(
            "⚠ Addiction problem",
            Style::new().fg(Color::Yellow),
        ));
    }
    frame.render_widget(Paragraph::new(warnings), warn_area);
}

pub(super) fn draw_band_panel(frame: &mut Frame, game: &Game, area: Rect) {
    let block = Block::bordered().title(" 🎸 Band ");
    let inner = block.inner(area);
    frame.render_widget(block, area);

    let [fame_area, skill_area, rest_area] = Layout::vertical([
        Constraint::Length(1),
        Constraint::Length(1),
        Constraint::Min(0),
    ])
    .areas(inner);

    frame.render_widget(
        gauge(
            format!("Fame {}% · {}", game.band.fame, game.band.get_fame_level()),
            game.band.fame,
            Color::Magenta,
        ),
        fame_area,
    );
    frame.render_widget(
        gauge(
            format!(
                "Skill {}% · {}",
                game.band.skill,
                game.band.get_skill_level()
            ),
            game.band.skill,
            Color::Blue,
        ),
        skill_area,
    );

    let deal_line = match game.band.current_deal() {
        Some(deal) => Line::from(vec![
            Span::raw("Deal   "),
            Span::styled(
                format!(
                    "{} ({}/{} albums)",
                    deal.label_name, deal.albums_delivered, deal.albums_required
                ),
                Style::new().fg(Color::Yellow),
            ),
        ]),
        None => Line::from(vec![
            Span::raw("Deal   "),
            Span::styled("unsigned", Style::new().fg(Color::DarkGray)),
        ]),
    };

    let lines = vec![
        Line::from(format!("Morale {}%", game.band.band_morale())),
        Line::from(format!(
            "Unreleased songs: {}",
            game.band.unreleased_songs.len()
        )),
        Line::from(format!(
            "Singles {} · Albums {}",
            game.band.singles_released.len(),
            game.band.albums_released.len(),
        )),
        deal_line,
        Line::from(format!("Dropping soon: {}", game.just_released_music.len())),
    ];
    frame.render_widget(Paragraph::new(lines), rest_area);
}

pub(super) fn draw_members_panel(frame: &mut Frame, game: &Game, area: Rect) {
    let block = Block::bordered().title(" 👥 Members ");
    let inner = block.inner(area);
    frame.render_widget(block, area);

    let mut lines: Vec<Line> = Vec::new();
    for member in &game.band.members {
        let mut spans = vec![
            Span::styled(format!("{:<8}", member.name), Style::new().bold()),
            Span::raw(format!("{} ", member.instrument)),
            Span::styled(
                format!("s{} l{}", member.skill, member.loyalty),
                Style::new().fg(Color::DarkGray),
            ),
        ];
        if member.drug_problem {
            spans.push(Span::styled(" ⚠", Style::new().fg(Color::Yellow)));
        }
        lines.push(Line::from(spans));
    }
    frame.render_widget(Paragraph::new(lines), inner);
}

pub(super) fn draw_scene_panel(frame: &mut Frame, game: &Game, area: Rect) {
    let block = Block::bordered().title(" 🎵 Scene ");
    let inner = block.inner(area);
    frame.render_widget(block, area);

    let era = game.timeline.get_current_era();
    let mut scene_bands: Vec<_> = game.world.bands.iter().collect();
    scene_bands.sort_by_key(|b| std::cmp::Reverse(b.fame));
    let top_band = scene_bands
        .first()
        .map(|b| format!("{} ({}%)", b.name, b.fame))
        .unwrap_or_else(|| "nobody".to_string());

    let mut lines = vec![
        Line::from(format!("Trend      {}", game.world.current_trends)),
        Line::from(format!("Demand     {}%", game.world.music_market.demand)),
        Line::from(format!(
            "Economy    {}",
            game.world.music_market.economic_state
        )),
        Line::from(format!(
            "Innovation {}%",
            era.market_conditions.innovation_openness
        )),
        Line::from(format!(
            "Bands      {} in the scene",
            game.world.bands.len()
        )),
        Line::from(format!("Top Act    {}", top_band)),
    ];
    // The reigning #1 record on the Local board — in your colours when
    // it's yours, and absent entirely while the charts are still empty.
    // §E split Local in two; the panel has room for one line, so it shows
    // the bigger of the two #1s (a chart-topping album usually outsells a
    // chart-topping single, and when it doesn't, the single is the story).
    if let Some(hit) = [
        crate::game::music::ReleaseType::Single,
        crate::game::music::ReleaseType::Album,
    ]
    .into_iter()
    .filter_map(|format| {
        game.world
            .chart_board(crate::game::world::ChartRegion::Local, format)
            .first()
    })
    .max_by_key(|entry| entry.score)
    {
        let text = format!("No. 1      '{}' — {}", hit.title, hit.band_name);
        lines.push(if hit.is_player {
            Line::styled(text, Style::new().fg(ACCENT).bold())
        } else {
            Line::from(text)
        });
    }
    lines.push(Line::from(Span::styled(
        format!("Hot: {}", era.dominant_genres.join(", ")),
        Style::new().fg(Color::Cyan),
    )));
    frame.render_widget(Paragraph::new(lines).wrap(Wrap { trim: true }), inner);
}

pub(super) fn draw_menu(frame: &mut Frame, app: &App, area: Rect) {
    // The activity strip (design §H, S-UI) takes the top of this column while a
    // commitment is running: what the band is doing, week n of m, and how to
    // carry on or walk away. The menu stays below it because a *turn* action is
    // what advances the activity — the screen-opening rows (Charts, Discography,
    // Save, Load) do not — so hiding the menu would leave the player with no way
    // to move. Six rows: three content lines plus the border, with one spare so
    // a two-line headline (a long band name in "Opening for …") cannot push the
    // week counter out of the box. It comes out of the menu's height, so at
    // 80x24 the action list scrolls — a fair trade, because the counter is the
    // one thing this screen exists to show.
    let (strip_area, menu_area) = match &app.game.ongoing_activity {
        Some(_) => {
            let [strip, rest] =
                Layout::vertical([Constraint::Length(6), Constraint::Min(4)]).areas(area);
            (Some(strip), rest)
        }
        None => (None, area),
    };

    if let (Some(strip), Some(activity)) = (strip_area, &app.game.ongoing_activity) {
        draw_activity_strip(frame, activity, strip);
    }

    let entries = app.menu_entries();
    let items: Vec<ListItem> = entries
        .iter()
        .map(|entry| {
            let key_style = if entry.enabled {
                Style::new().fg(Color::Yellow).bold()
            } else {
                Style::new().fg(Color::DarkGray)
            };
            let label_style = if entry.enabled {
                Style::new().fg(Color::White)
            } else {
                Style::new().fg(Color::DarkGray)
            };
            let detail_style = if entry.enabled {
                Style::new().fg(Color::Green)
            } else {
                Style::new()
                    .fg(Color::DarkGray)
                    .add_modifier(Modifier::ITALIC)
            };
            ListItem::new(Line::from(vec![
                Span::styled(
                    format!(" {} ", entry.hotkey.to_ascii_uppercase()),
                    key_style,
                ),
                Span::styled(format!("{:<15}", entry.label), label_style),
                Span::styled(entry.detail.clone(), detail_style),
            ]))
        })
        .collect();

    let list = List::new(items)
        .block(Block::bordered().title(" This Week (↑↓ + Enter, or hotkey) "))
        .highlight_style(Style::new().add_modifier(Modifier::REVERSED));

    let mut state = ListState::default().with_selected(Some(app.menu_selected));
    frame.render_stateful_widget(list, menu_area, &mut state);
}

/// The activity screen (design §H, S-UI): while a commitment runs, say what it
/// is, which week of how many, and what the last settled week did.
///
/// "Continue" is any *week-taking* menu row — its own action is ignored and
/// the turn resolves one more week of the activity, which is what those rows
/// now say (`keep touring` and the like — `App`'s `continue_detail`, #41).
/// Instant rows (Vault, Re-press, Marketing, Lifestyle, Deals, the support
/// slot's decline) apply and take no week (#37); screen rows (Charts,
/// Discography, Save, Load, and Quit's save-first prompt) dispatch nothing;
/// booking rows refuse. "Abandon" is the `x` hotkey, which opens the §F
/// confirmation (`Screen::ConfirmAbandon`); only its `Y` dispatches
/// `GameAction::AbandonActivity`.
fn draw_activity_strip(frame: &mut Frame, activity: &crate::game::OngoingActivity, area: Rect) {
    let total = activity.weeks_total();
    let current = activity.week_number();
    let final_week = activity.is_final_week();

    let header = Line::from(vec![
        Span::styled(
            format!(" ▶ {} ", activity.label()),
            Style::new().fg(ACCENT).bold(),
        ),
        Span::styled(activity.headline(), Style::new().fg(Color::White).bold()),
    ]);

    // `n of m`, then a per-week bar and the settled count on one line — a long
    // tour reads as long at a glance. Both lines are kept short enough to fit
    // the strip at 80 columns: an earlier version put the hint on the counter's
    // line, it wrapped, and the wrap pushed "Week n of m" out of the 6-row box
    // entirely, so the one thing the screen exists to show was the thing
    // clipped.
    let week_line = Line::from(Span::styled(
        format!("   Week {current} of {total}"),
        Style::new().fg(Color::Yellow),
    ));

    let done = usize::from(activity.weeks_done());
    let total_cells = usize::from(total).max(1);
    let bar: String = (0..total_cells)
        .map(|i| if i < done { '█' } else { '░' })
        .collect();
    let bar_line = Line::from(vec![
        Span::styled(bar, Style::new().fg(Color::Green)),
        Span::styled(
            format!("  {done} settled"),
            Style::new().fg(Color::DarkGray),
        ),
        Span::styled(
            if final_week {
                "  last week"
            } else {
                // Names the *kind* of action, not "any action": Charts,
                // Discography, Save and Load open a screen and dispatch nothing,
                // so they never advance a commitment. "turns advance" rather
                // than "turn actions advance" purely for width — at 80 columns
                // the longer phrase is clipped mid-word in this box.
                "  turns advance"
            },
            Style::new().fg(Color::DarkGray),
        ),
    ]);

    frame.render_widget(
        Paragraph::new(vec![header, week_line, bar_line])
            .block(Block::bordered().title(" 🎬 Activity  —  X abandons "))
            .wrap(Wrap { trim: false }),
        area,
    );
}

pub(super) fn draw_log(frame: &mut Frame, app: &App, area: Rect) {
    let visible = area.height.saturating_sub(2) as usize;
    let items: Vec<ListItem> = app
        .log
        .iter()
        .rev()
        .take(visible.max(1))
        .rev()
        .map(|entry| {
            let style = match entry.kind {
                LogKind::Game => Style::new().fg(Color::White),
                LogKind::Ui => Style::new().fg(Color::DarkGray),
                LogKind::Error => Style::new().fg(Color::Red),
            };
            ListItem::new(Line::styled(entry.text.clone(), style))
        })
        .collect();

    frame.render_widget(
        List::new(items).block(Block::bordered().title(" 📻 News & Events ")),
        area,
    );
}
