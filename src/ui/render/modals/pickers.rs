//! Venue, pressing-run, and tour-region pickers.

use ratatui::{
    Frame,
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Clear, List, ListItem, ListState},
};

use crate::data::format_money;
use crate::game::PRESSING_TIERS;
use crate::game::music::ReleaseType;
use crate::ui::app::{App, Screen};

use super::super::{centered_rect, format_population};
pub(crate) fn draw_venue_picker_modal(frame: &mut Frame, app: &App) {
    let Screen::VenuePicker { selected } = app.screen else {
        return;
    };
    let area = centered_rect(74, 50, frame.area());
    frame.render_widget(Clear, area);

    let items: Vec<ListItem> = app
        .game
        .world
        .venues
        .iter()
        .map(|venue| {
            let locked = venue.prestige > app.game.band.fame.saturating_add(20);

            let status = if locked {
                Span::styled(" 🔒 LOCKED", Style::new().fg(Color::DarkGray))
            } else {
                Span::styled(" 🔓 UNLOCKED", Style::new().fg(Color::Green))
            };

            let style = if locked {
                Style::new().fg(Color::DarkGray)
            } else {
                Style::new().fg(Color::White)
            };

            ListItem::new(Line::from(vec![
                Span::styled(format!("{:<22}", venue.name), style.bold()),
                Span::styled(format!("  Prestige: {:<3}", venue.prestige), style),
                Span::styled(format!("  Capacity: {:<6}", venue.capacity), style),
                Span::styled(
                    format!("  Base Pay: {:<6}", format_money(venue.base_payment as i32)),
                    style,
                ),
                Span::raw("  "),
                status,
            ]))
        })
        .collect();

    let list = List::new(items)
        .block(
            Block::bordered()
                .title(" 🎤 Select Venue to Play Gig ")
                .title_bottom(" Enter perform · Esc close "),
        )
        .highlight_style(Style::new().add_modifier(Modifier::REVERSED));
    let mut state = ListState::default().with_selected(Some(selected));
    frame.render_stateful_widget(list, area, &mut state);
}

pub(crate) fn draw_pressing_picker_modal(frame: &mut Frame, app: &App) {
    let Screen::PressingPicker {
        release_type,
        selected,
    } = app.screen
    else {
        return;
    };
    let area = centered_rect(72, 40, frame.area());
    frame.render_widget(Clear, area);

    let recording = app.game.recording_cost(&release_type);
    let items: Vec<ListItem> = PRESSING_TIERS
        .iter()
        .map(|(name, copies)| {
            let pressing = app.game.pressing_cost(&release_type, *copies);
            let affordable = app.game.player.can_afford(recording + pressing);
            let style = if affordable {
                Style::new().fg(Color::White)
            } else {
                Style::new().fg(Color::DarkGray)
            };
            ListItem::new(Line::from(vec![
                Span::styled(format!("{:<14}", name), style.bold()),
                Span::styled(format!("  {:>6} copies", copies), style),
                Span::styled(format!("  Pressing: {:<8}", format_money(pressing)), style),
                Span::styled(
                    format!(
                        "  Total with studio: {:<8}",
                        format_money(recording + pressing)
                    ),
                    style,
                ),
            ]))
        })
        .collect();

    let kind = match release_type {
        ReleaseType::Single => "Single",
        ReleaseType::Album => "Album",
    };
    let list = List::new(items)
        .block(
            Block::bordered()
                .title(format!(" 📀 Press the {} — choose your run ", kind))
                .title_bottom(" Enter record · Esc cancel "),
        )
        .highlight_style(Style::new().add_modifier(Modifier::REVERSED));
    let mut state = ListState::default().with_selected(Some(selected));
    frame.render_stateful_widget(list, area, &mut state);
}

pub(crate) fn draw_region_picker_modal(frame: &mut Frame, app: &App) {
    let Screen::RegionPicker { selected } = app.screen else {
        return;
    };
    let area = centered_rect(88, 60, frame.area());
    frame.render_widget(Clear, area);

    let sorted_regions = app.game.get_sorted_regions();
    let items: Vec<ListItem> = sorted_regions
        .iter()
        .map(
            |(country_key, region_key, region_name, population, economic_strength, fame_req)| {
                let locked = app.game.band.fame < *fame_req;
                let regional_fame_key = format!("{}:{}", country_key, region_key);
                let regional_fame = *app.game.regional_fame.get(&regional_fame_key).unwrap_or(&0);

                let status = if locked {
                    Span::styled(
                        format!(" 🔒 Req Fame: {}", fame_req),
                        Style::new().fg(Color::DarkGray),
                    )
                } else {
                    Span::styled(
                        format!(" 🔓 Reg Fame: {}%", regional_fame),
                        Style::new().fg(Color::Green),
                    )
                };

                let style = if locked {
                    Style::new().fg(Color::DarkGray)
                } else {
                    Style::new().fg(Color::White)
                };

                let tier_name = if app.game.band.fame < 35 {
                    "local"
                } else if app.game.band.fame < 60 {
                    "regional"
                } else if app.game.band.fame < 80 {
                    "national"
                } else {
                    "international"
                };
                let country_travel_mult = match country_key.as_str() {
                    "united_states" => 1.5,
                    "united_kingdom" => 0.8,
                    "europe" => 1.2,
                    "japan" => 1.0,
                    "australia" => 1.4,
                    _ => 1.0,
                };
                let cost_str = if let Some(touring_costs) = app
                    .game
                    .data_files
                    .markets_data
                    .market_modifiers
                    .touring_costs
                    .get(tier_name)
                {
                    let cost =
                        (touring_costs.base_cost_per_show as f32 * country_travel_mult) as i32;
                    format_money(cost)
                } else {
                    "N/A".to_string()
                };

                let country_name = country_key.replace("_", " ");
                ListItem::new(Line::from(vec![
                    Span::styled(format!("{:<15}", region_name), style.bold()),
                    Span::styled(
                        format!(" ({:<15})", country_name),
                        Style::new().fg(Color::Cyan),
                    ),
                    Span::styled(
                        format!("  Pop: {:>8}", format_population(*population)),
                        style,
                    ),
                    Span::styled(format!("  Econ: {:>3}", economic_strength), style),
                    Span::styled(
                        format!("  Cost: {:>6}", cost_str),
                        if locked {
                            style
                        } else {
                            Style::new().fg(Color::Yellow)
                        },
                    ),
                    Span::raw("  "),
                    status,
                ]))
            },
        )
        .collect();

    let list = List::new(items)
        .block(
            Block::bordered()
                .title(" 🚌 Select Region to Tour ")
                .title_bottom(" Enter book tour · Esc close "),
        )
        .highlight_style(Style::new().add_modifier(Modifier::REVERSED));
    let mut state = ListState::default().with_selected(Some(selected));
    frame.render_stateful_widget(list, area, &mut state);
}
