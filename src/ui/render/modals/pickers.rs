//! Venue, pressing-run, tour-region, and tour-booking (rig/length/quote)
//! pickers.

use ratatui::{
    Frame,
    layout::{Constraint, Layout},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Clear, List, ListItem, ListState, Paragraph, Wrap},
};

use crate::data::format_money;
use crate::game::PRESSING_TIERS;
use crate::game::TourRig;
use crate::game::music::{DistributionChannel, ReleaseType};
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

/// The pressing-run picker, extended (design §E-3, M6) with a distribution
/// channel choice alongside it — mirrors the tour booking picker's
/// rig-on-↑↓/length-on-←→ split (`draw_tour_booking_picker_modal`). The
/// channel row is only shown while unsigned; the label decides both pressing
/// and reach for a signed act, and never reaches this screen.
pub(crate) fn draw_pressing_picker_modal(frame: &mut Frame, app: &App) {
    let Screen::PressingPicker {
        release_type,
        selected,
        channel,
    } = app.screen
    else {
        return;
    };
    let signed = app.game.band.current_deal().is_some();
    let fee = if signed { 0 } else { channel.fee() };

    let kind = match release_type {
        ReleaseType::Single => "Single",
        ReleaseType::Album => "Album",
    };
    let area = centered_rect(78, 56, frame.area());
    frame.render_widget(Clear, area);
    let block = Block::bordered()
        .title(format!(" 📀 Press the {} ", kind))
        .title_bottom(" ↑↓ pressing run · ←→ distribution · Enter record · Esc cancel ");
    let inner = block.inner(area);
    frame.render_widget(block, area);

    let [tiers_area, channel_area] =
        Layout::vertical([Constraint::Min(6), Constraint::Length(6)]).areas(inner);

    let recording = app.game.recording_cost(&release_type);
    let items: Vec<ListItem> = PRESSING_TIERS
        .iter()
        .map(|(name, copies)| {
            let pressing = app.game.pressing_cost(&release_type, *copies);
            let total = recording + pressing + fee;
            let affordable = app.game.player.can_afford(total);
            let style = if affordable {
                Style::new().fg(Color::White)
            } else {
                Style::new().fg(Color::DarkGray)
            };
            ListItem::new(Line::from(vec![
                Span::styled(format!("{:<14}", name), style.bold()),
                Span::styled(format!("  {:>6} copies", copies), style),
                Span::styled(format!("  Pressing: {:<8}", format_money(pressing)), style),
                Span::styled(format!("  Total: {:<8}", format_money(total)), style),
            ]))
        })
        .collect();
    let list = List::new(items)
        .block(Block::default().title("Pressing run"))
        .highlight_style(Style::new().add_modifier(Modifier::REVERSED));
    let mut state = ListState::default().with_selected(Some(selected));
    frame.render_stateful_widget(list, tiers_area, &mut state);

    if signed {
        frame.render_widget(
            Paragraph::new(vec![
                Line::from("Distribution"),
                Line::styled(
                    "  Your label's network handles reach — no channel to choose.",
                    Style::new().fg(Color::DarkGray),
                ),
            ]),
            channel_area,
        );
        return;
    }

    let fame = app.game.band.fame;
    let channel_lines: Vec<Line> = DistributionChannel::ALL
        .iter()
        .map(|&c| {
            let locked = !c.is_available(fame);
            let is_selected = c == channel;
            let mut style = if locked {
                Style::new().fg(Color::DarkGray)
            } else {
                Style::new().fg(Color::White)
            };
            if is_selected {
                style = style.add_modifier(Modifier::REVERSED);
            }
            let gate = if locked {
                format!(" 🔒 needs fame {}", c.fame_gate())
            } else {
                String::new()
            };
            Line::styled(
                format!(
                    " {:<22} fee {:<7} reach floor {:<5.2}{}",
                    c.label(),
                    format_money(c.fee()),
                    c.reach_floor(),
                    gate
                ),
                style,
            )
        })
        .collect();
    frame.render_widget(
        Paragraph::new(
            std::iter::once(Line::from("Distribution"))
                .chain(channel_lines)
                .collect::<Vec<_>>(),
        ),
        channel_area,
    );
}

/// Which sold-out/low-stock release to re-press (design §E-1 indie half,
/// M6).
pub(crate) fn draw_repress_picker_modal(frame: &mut Frame, app: &App) {
    let Screen::RePressPicker { selected } = app.screen else {
        return;
    };
    let area = centered_rect(74, 50, frame.area());
    frame.render_widget(Clear, area);

    let releases = app.game.repressable_releases();
    let items: Vec<ListItem> = releases
        .iter()
        .map(|release| {
            ListItem::new(Line::from(vec![
                Span::styled(format!("{:<28}", release.name), Style::new().bold()),
                Span::styled(
                    format!(
                        "  {:>7} / {:<7} sold",
                        release.copies_sold, release.copies_pressed
                    ),
                    Style::new(),
                ),
            ]))
        })
        .collect();

    let list = List::new(items)
        .block(
            Block::bordered()
                .title(" 🏭 Re-press — choose a release ")
                .title_bottom(" Enter choose run · Esc close "),
        )
        .highlight_style(Style::new().add_modifier(Modifier::REVERSED));
    let mut state = ListState::default().with_selected(Some(selected));
    frame.render_stateful_widget(list, area, &mut state);
}

/// The pressing tier for a re-press, once the release is chosen (design
/// §E-1 indie half, M6) — the same tiers/costs as the initial pressing
/// picker, minus the recording cost (there's no re-recording to top up a
/// run).
pub(crate) fn draw_repress_tier_picker_modal(frame: &mut Frame, app: &App) {
    let Screen::RePressTierPicker {
        release_id,
        selected,
    } = app.screen
    else {
        return;
    };
    let release_type = app
        .game
        .band
        .singles_released
        .iter()
        .chain(app.game.band.albums_released.iter())
        .find(|r| r.id == release_id)
        .map(|r| r.release_type)
        .unwrap_or(ReleaseType::Single);

    let area = centered_rect(72, 40, frame.area());
    frame.render_widget(Clear, area);

    let items: Vec<ListItem> = PRESSING_TIERS
        .iter()
        .map(|(name, copies)| {
            let pressing = app.game.pressing_cost(&release_type, *copies);
            let affordable = app.game.player.can_afford(pressing);
            let style = if affordable {
                Style::new().fg(Color::White)
            } else {
                Style::new().fg(Color::DarkGray)
            };
            ListItem::new(Line::from(vec![
                Span::styled(format!("{:<14}", name), style.bold()),
                Span::styled(format!("  {:>6} copies", copies), style),
                Span::styled(format!("  Cost: {:<8}", format_money(pressing)), style),
            ]))
        })
        .collect();

    let list = List::new(items)
        .block(
            Block::bordered()
                .title(" 🏭 Choose a fresh pressing run ")
                .title_bottom(" Enter press · Esc back "),
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

                // No cost preview here (M1, design §A): cost depends on the
                // rig and length chosen next, not the region or fame — the
                // booking picker shows the itemized quote.
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
                .title_bottom(" Enter choose rig & length · Esc close "),
        )
        .highlight_style(Style::new().add_modifier(Modifier::REVERSED));
    let mut state = ListState::default().with_selected(Some(selected));
    frame.render_stateful_widget(list, area, &mut state);
}

/// The rig + length picker (design §A, M1): reached after choosing a region.
/// Shows every rig and length with its fame gate, plus a live itemized quote
/// — cost, weeks, shows, and a projected gross range — computed from the
/// exact formula `action_go_on_tour` uses, so booking is never a surprise.
pub(crate) fn draw_tour_booking_picker_modal(frame: &mut Frame, app: &App) {
    let Screen::TourBookingPicker {
        region_index,
        rig,
        weeks,
    } = app.screen
    else {
        return;
    };

    let region_name = app
        .game
        .get_sorted_regions()
        .get(region_index)
        .map(|(_, _, name, _, _, _)| name.clone())
        .unwrap_or_else(|| "Unknown region".to_string());

    let area = centered_rect(84, 66, frame.area());
    frame.render_widget(Clear, area);

    let block = Block::bordered()
        .title(format!(" 🎫 Book a Tour of {region_name} "))
        .title_bottom(" ↑↓ rig · ←→ length · Enter book · Esc back ");
    let inner = block.inner(area);
    frame.render_widget(block, area);

    let [rig_area, weeks_area, quote_area] = Layout::vertical([
        Constraint::Length(6),
        Constraint::Length(3),
        Constraint::Min(6),
    ])
    .areas(inner);

    let rig_items: Vec<ListItem> = TourRig::ALL
        .iter()
        .map(|&r| {
            let available = app.game.rig_is_available(r);
            let style = if available {
                Style::new().fg(Color::White)
            } else {
                Style::new().fg(Color::DarkGray)
            };
            let (health_cost, stress_cost) = r.wear_per_week();
            let gate = if available {
                format!("🔓 fame {}+", r.fame_gate())
            } else {
                format!("🔒 needs fame {}", r.fame_gate())
            };
            ListItem::new(Line::from(vec![
                Span::styled(format!("{:<17}", r.label()), style.bold()),
                Span::styled(
                    format!(" {:>7}/wk", format_money(r.cost_per_week() as i32)),
                    style,
                ),
                Span::styled(format!("  cap ×{:.1}", r.capacity_mult()), style),
                Span::styled(
                    format!("  health -{health_cost}/stress +{stress_cost} per wk"),
                    style,
                ),
                Span::raw("  "),
                Span::styled(gate, style),
            ]))
        })
        .collect();
    let rig_list = List::new(rig_items)
        .block(Block::default().title("Rig"))
        .highlight_style(Style::new().add_modifier(Modifier::REVERSED));
    let mut rig_state = ListState::default().with_selected(Some(rig.ordinal()));
    frame.render_stateful_widget(rig_list, rig_area, &mut rig_state);

    let weeks_spans: Vec<Span> = (1..=4u8)
        .map(|w| {
            let available = app.game.tour_length_is_available(w);
            let selected = w == weeks;
            let label = if available {
                format!(" {w} week{} ", if w == 1 { "" } else { "s" })
            } else {
                format!(" {w} week{} 🔒 ", if w == 1 { "" } else { "s" })
            };
            let mut style = if available {
                Style::new().fg(Color::White)
            } else {
                Style::new().fg(Color::DarkGray)
            };
            if selected {
                style = style.add_modifier(Modifier::REVERSED);
            }
            Span::styled(label, style)
        })
        .collect();
    frame.render_widget(
        Paragraph::new(vec![Line::from("Length"), Line::from(weeks_spans)]),
        weeks_area,
    );

    let quote_lines: Vec<Line> = match app.game.quote_tour(region_index, rig, weeks) {
        Ok(quote) => {
            let affordable = app.game.player.can_afford(quote.cost);
            vec![
                Line::styled("Quote", Style::new().bold()),
                Line::from(format!(
                    "  {} of {}, {} week{}: {} shows",
                    quote.rig.label(),
                    quote.region_name,
                    quote.weeks,
                    if quote.weeks == 1 { "" } else { "s" },
                    quote.shows
                )),
                Line::from(format!("  Cost: {}", format_money(quote.cost))),
                Line::from(format!(
                    "  Projected gross: {} – {}",
                    format_money(quote.gross_low as i32),
                    format_money(quote.gross_high as i32)
                )),
                Line::from(format!(
                    "  Fame gain: +{}   Regional fame gain: +{}–{}",
                    quote.fame_gain, quote.regional_fame_gain_min, quote.regional_fame_gain_max
                )),
                if affordable {
                    Line::styled("  Ready to book.", Style::new().fg(Color::Green))
                } else {
                    Line::styled(
                        format!(
                            "  Not enough cash — you have {}.",
                            format_money(app.game.player.money)
                        ),
                        Style::new().fg(Color::Red),
                    )
                },
            ]
        }
        Err(msg) => vec![
            Line::styled("Quote", Style::new().bold()),
            Line::styled(format!("  {msg}"), Style::new().fg(Color::Red)),
        ],
    };
    frame.render_widget(
        Paragraph::new(quote_lines).wrap(Wrap { trim: false }),
        quote_area,
    );
}
