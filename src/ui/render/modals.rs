use ratatui::{
    Frame,
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Clear, List, ListItem, ListState, Paragraph, Wrap},
};

use crate::data::format_money;
use crate::game::PRESSING_TIERS;
use crate::game::music::{MarketingCampaignType, ReleaseType};

use crate::ui::app::{App, FileMode, Screen};

use super::{ACCENT, centered_rect, format_population};

pub(super) fn draw_deals_modal(frame: &mut Frame, app: &App) {
    let Screen::Deals { selected, detail } = &app.screen else {
        return;
    };
    let area = centered_rect(72, 70, frame.area());
    frame.render_widget(Clear, area);

    let offers = &app.game.pending_deal_offers;
    if *detail {
        let offer = &offers[(*selected).min(offers.len() - 1)];
        let block = Block::bordered()
            .title(format!(" ✍️ {} ({}) ", offer.label_name, offer.label_tier))
            .title_style(Style::new().fg(Color::Yellow).bold());
        let inner = block.inner(area);
        frame.render_widget(block, area);

        let data = &offer.original_label_data;
        let mut lines = vec![
            Line::from(""),
            Line::from(format!(
                "  Advance          {}",
                format_money(offer.advance as i32)
            )),
            Line::from(format!(
                "  Royalty rate     {:.1}%",
                offer.royalty_rate * 100.0
            )),
            Line::from(format!("  Albums required  {}", offer.albums_required)),
        ];
        if let Some(deadline) = offer.expires_week {
            let weeks_left = deadline.saturating_sub(app.game.week);
            lines.push(Line::from(format!(
                "  Offer expires in {} week{}",
                weeks_left,
                if weeks_left == 1 { "" } else { "s" }
            )));
        }
        lines.extend([
            Line::from(""),
            Line::styled("  About the label", Style::new().fg(Color::Cyan).bold()),
            Line::from(format!("  Market reach       {}/100", data.market_reach)),
            Line::from(format!("  Financial power    {}/100", data.financial_power)),
            Line::from(format!(
                "  Artist development {}/100",
                data.artist_development
            )),
            Line::from(format!(
                "  Creative freedom   {}/100",
                data.creative_freedom
            )),
            Line::from(format!("  Reputation: {}", data.reputation)),
            Line::from(""),
            Line::styled(
                "  [A]ccept · [R]eject · [Esc] back",
                Style::new().fg(Color::DarkGray),
            ),
        ]);
        frame.render_widget(Paragraph::new(lines).wrap(Wrap { trim: false }), inner);
    } else {
        let items: Vec<ListItem> = offers
            .iter()
            .map(|offer| {
                ListItem::new(Line::from(vec![
                    Span::styled(format!("{:<22}", offer.label_name), Style::new().bold()),
                    Span::styled(
                        format!("{:<12}", offer.label_tier),
                        Style::new().fg(Color::Cyan),
                    ),
                    Span::raw(format!(
                        "{} adv · {:.0}% · {} albums",
                        format_money(offer.advance as i32),
                        offer.royalty_rate * 100.0,
                        offer.albums_required
                    )),
                ]))
            })
            .collect();
        let list = List::new(items)
            .block(
                Block::bordered()
                    .title(" ✍️ Record Deal Offers ")
                    .title_bottom(" Enter details · A accept · R reject · Esc close "),
            )
            .highlight_style(Style::new().add_modifier(Modifier::REVERSED));
        let mut state = ListState::default().with_selected(Some(*selected));
        frame.render_stateful_widget(list, area, &mut state);
    }
}

pub(super) fn draw_charts_modal(frame: &mut Frame, app: &App) {
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

pub(super) fn draw_support_modal(frame: &mut Frame, app: &App) {
    let Some(offer) = &app.game.pending_support_offer else {
        return;
    };

    let area = centered_rect(58, 45, frame.area());
    frame.render_widget(Clear, area);
    let block = Block::bordered()
        .title(" 🎟️ Support Slot Offer ")
        .title_style(Style::new().fg(Color::Yellow).bold());
    let inner = block.inner(area);
    frame.render_widget(block, area);

    let weeks_left = offer.expires_week.saturating_sub(app.game.week);
    let lines = vec![
        Line::from(""),
        Line::from(vec![
            Span::styled(
                offer.host_band.clone(),
                Style::new().fg(Color::Magenta).bold(),
            ),
            Span::raw(format!(
                " (fame {}%) want you as their opening act.",
                offer.host_fame
            )),
        ])
        .centered(),
        Line::from(""),
        Line::from(format!("  Length     {} weeks on the road", offer.weeks)),
        Line::from(format!("  Pay        {}", format_money(offer.pay as i32))),
        Line::from(format!("  Exposure   fame +{}", offer.fame_gain)),
        Line::from(format!(
            "  Offer expires in {} week{}",
            weeks_left,
            if weeks_left == 1 { "" } else { "s" }
        )),
        Line::from(""),
        Line::styled(
            "  [A]ccept · [R]eject · [Esc] think it over",
            Style::new().fg(Color::DarkGray),
        ),
    ];
    frame.render_widget(Paragraph::new(lines).wrap(Wrap { trim: false }), inner);
}

pub(super) fn draw_marketing_modal(frame: &mut Frame, app: &App) {
    let area = centered_rect(72, 60, frame.area());
    frame.render_widget(Clear, area);

    match &app.screen {
        Screen::MarketingRelease { selected } => {
            let targets = app.marketing_targets();
            let items: Vec<ListItem> = targets
                .iter()
                .map(|t| {
                    let status = if t.pending {
                        Span::styled("upcoming ", Style::new().fg(Color::Yellow))
                    } else {
                        Span::styled("in stores", Style::new().fg(Color::Green))
                    };
                    ListItem::new(Line::from(vec![
                        Span::styled(format!("{:<30}", t.name), Style::new().bold()),
                        status,
                        Span::raw(format!("  buzz {}%", t.buzz)),
                    ]))
                })
                .collect();
            let list = List::new(items)
                .block(
                    Block::bordered()
                        .title(" 📣 Promote which release? ")
                        .title_bottom(" Enter choose · Esc close "),
                )
                .highlight_style(Style::new().add_modifier(Modifier::REVERSED));
            let mut state = ListState::default().with_selected(Some(*selected));
            frame.render_stateful_widget(list, area, &mut state);
        }
        Screen::MarketingCampaign {
            release_name,
            selected,
            ..
        } => {
            let items: Vec<ListItem> = MarketingCampaignType::ALL
                .iter()
                .map(|c| {
                    let spec = c.spec();
                    ListItem::new(Line::from(vec![
                        Span::styled(format!("{:<18}", spec.name), Style::new().bold()),
                        Span::styled(format!("${:<6}", spec.cost), Style::new().fg(Color::Green)),
                        Span::raw(format!(
                            "{} weeks · +{} buzz",
                            spec.duration_weeks, spec.effectiveness_bonus
                        )),
                    ]))
                })
                .collect();
            let list = List::new(items)
                .block(
                    Block::bordered()
                        .title(format!(" 📣 Campaign for '{}' ", release_name))
                        .title_bottom(" Enter launch · Esc back "),
                )
                .highlight_style(Style::new().add_modifier(Modifier::REVERSED));
            let mut state = ListState::default().with_selected(Some(*selected));
            frame.render_stateful_widget(list, area, &mut state);
        }
        _ => {}
    }
}

pub(super) fn draw_file_modal(frame: &mut Frame, app: &App) {
    let Screen::File { mode, input } = &app.screen else {
        return;
    };
    let title = match mode {
        FileMode::Save => " 💾 Save game ",
        FileMode::Load => " 📂 Load game ",
    };

    let area = centered_rect(50, 22, frame.area());
    frame.render_widget(Clear, area);
    let block = Block::bordered().title(title);
    let inner = block.inner(area);
    frame.render_widget(block, area);

    let lines = vec![
        Line::from(""),
        Line::from(vec![
            Span::raw("  File: "),
            Span::styled(format!("{}█", input), Style::new().fg(Color::Yellow)),
        ]),
        Line::from(""),
        Line::styled(
            format!(
                "  empty = {} · Enter confirm · Esc cancel",
                crate::ui::app::SAVE_FILE_DEFAULT
            ),
            Style::new().fg(Color::DarkGray),
        ),
    ];
    frame.render_widget(Paragraph::new(lines), inner);
}

pub(super) fn draw_venue_picker_modal(frame: &mut Frame, app: &App) {
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

pub(super) fn draw_pressing_picker_modal(frame: &mut Frame, app: &App) {
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

pub(super) fn draw_region_picker_modal(frame: &mut Frame, app: &App) {
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
