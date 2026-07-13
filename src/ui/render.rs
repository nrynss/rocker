use ratatui::{
    Frame,
    layout::{Constraint, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Clear, Gauge, List, ListItem, ListState, Paragraph, Wrap},
};

use crate::data::{calculate_weeks_to_years_months, constants, format_money};
use crate::game::music::{MarketingCampaignType, ReleaseType};
use crate::game::world::MusicGenre;
use crate::game::{Game, PRESSING_TIERS};

use super::app::{App, FileMode, LogKind, Screen, SetupField};

const ACCENT: Color = Color::Red;

pub fn draw(frame: &mut Frame, app: &mut App) {
    match &app.screen {
        Screen::Setup { .. } => draw_setup(frame, app),
        Screen::GameOver => draw_game_over(frame, app),
        _ => {
            draw_main(frame, app);
            match &app.screen {
                Screen::Deals { .. } => draw_deals_modal(frame, app),
                Screen::SupportOffer => draw_support_modal(frame, app),
                Screen::MarketingRelease { .. } | Screen::MarketingCampaign { .. } => {
                    draw_marketing_modal(frame, app)
                }
                Screen::File { .. } => draw_file_modal(frame, app),
                Screen::VenuePicker { .. } => draw_venue_picker_modal(frame, app),
                Screen::RegionPicker { .. } => draw_region_picker_modal(frame, app),
                Screen::PressingPicker { .. } => draw_pressing_picker_modal(frame, app),
                _ => {}
            }
        }
    }
}

// --- Setup ---

fn draw_setup(frame: &mut Frame, app: &App) {
    let Screen::Setup { field } = &app.screen else { return };
    let picking_genre = *field == SetupField::Genre;

    // The genre list needs more room than the two name prompts.
    let area = centered_rect(60, if picking_genre { 75 } else { 50 }, frame.area());
    let block = Block::bordered()
        .title(" 🎸 ROCKER ")
        .title_style(Style::new().fg(ACCENT).bold());
    let inner = block.inner(area);
    frame.render_widget(block, area);

    let field_line = |label: &str, value: &str, active: bool| -> Line<'static> {
        let cursor = if active { "█" } else { "" };
        let style = if active {
            Style::new().fg(Color::Yellow).bold()
        } else {
            Style::new().fg(Color::DarkGray)
        };
        Line::from(vec![
            Span::styled(format!("{:<12}", label), style),
            Span::styled(format!("{}{}", value, cursor), Style::new().fg(Color::White)),
        ])
    };

    let mut lines = vec![
        Line::from(""),
        Line::styled("R  O  C  K  E  R", Style::new().fg(ACCENT).bold()).centered(),
        Line::styled(
            format!("Rock Star Management — est. {}", constants::STARTING_YEAR),
            Style::new().fg(Color::DarkGray),
        )
        .centered(),
        Line::from(""),
        Line::from("The Beatles just broke up. The stage is yours.").centered(),
        Line::from(""),
        field_line("Your name:", &app.name_input, *field == SetupField::Name),
        field_line("Band name:", &app.band_input, *field == SetupField::BandName),
    ];

    if picking_genre {
        lines.push(Line::from(""));
        lines.push(Line::styled("What do you play?", Style::new().fg(Color::Yellow).bold()));
        for (i, genre) in MusicGenre::ALL.iter().enumerate() {
            let (marker, style) = if i == app.genre_selected {
                ("▸", Style::new().fg(Color::Yellow).bold())
            } else {
                (" ", Style::new().fg(Color::DarkGray))
            };
            lines.push(Line::styled(format!("  {} {}", marker, genre.name()), style));
        }
        lines.push(Line::from(""));
        lines.push(
            Line::styled("↑↓ choose · Enter confirm · Esc quit", Style::new().fg(Color::DarkGray))
                .centered(),
        );
    } else {
        lines.push(Line::from(""));
        lines.push(
            Line::styled("Enter to confirm · Esc to quit", Style::new().fg(Color::DarkGray))
                .centered(),
        );
    }

    frame.render_widget(Paragraph::new(lines).wrap(Wrap { trim: false }), inner);
}

// --- Main screen ---

fn draw_main(frame: &mut Frame, app: &mut App) {
    let [header_area, stats_area, bottom_area] =
        Layout::vertical([Constraint::Length(4), Constraint::Length(10), Constraint::Min(8)])
            .areas(frame.area());

    draw_header(frame, &app.game, header_area);

    let [player_area, band_area, members_area, scene_area] =
        Layout::horizontal([Constraint::Percentage(25); 4]).areas(stats_area);
    draw_player_panel(frame, &app.game, player_area);
    draw_band_panel(frame, &app.game, band_area);
    draw_members_panel(frame, &app.game, members_area);
    draw_scene_panel(frame, &app.game, scene_area);

    let [menu_area, log_area] =
        Layout::horizontal([Constraint::Percentage(42), Constraint::Percentage(58)]).areas(bottom_area);
    draw_menu(frame, app, menu_area);
    draw_log(frame, app, log_area);
}

fn draw_header(frame: &mut Frame, game: &Game, area: Rect) {
    let weeks_elapsed = game.week.saturating_sub(1);
    let year = constants::STARTING_YEAR + weeks_elapsed / constants::WEEKS_PER_YEAR;
    let week_in_year = weeks_elapsed % constants::WEEKS_PER_YEAR + 1;
    let era = game.timeline.get_current_era().era_name.clone();

    let title = Line::from(vec![
        Span::styled("🎸 ROCKER", Style::new().fg(ACCENT).bold()),
        Span::raw("  ·  "),
        Span::styled(format!("Week {} of {}", week_in_year, year), Style::new().bold()),
        Span::raw("  ·  "),
        Span::styled(era, Style::new().fg(Color::Cyan)),
    ]);
    let subtitle = Line::from(vec![
        Span::styled(game.player.name.clone(), Style::new().fg(Color::Yellow)),
        Span::raw(" fronting "),
        Span::styled(format!("'{}'", game.band.name), Style::new().fg(Color::Magenta).bold()),
        Span::raw(format!("  ·  career: {}", calculate_weeks_to_years_months(game.week))),
    ]);

    frame.render_widget(
        Paragraph::new(vec![title, subtitle]).block(Block::bordered()),
        area,
    );
}

fn gauge(label: String, value: u8, color: Color) -> Gauge<'static> {
    Gauge::default()
        .ratio(f64::from(value.min(100)) / 100.0)
        .label(label)
        .gauge_style(Style::new().fg(color).bg(Color::Black))
        .use_unicode(true)
}

fn scale_color(value: u8, low_is_bad: bool) -> Color {
    let good = if low_is_bad { value } else { 100 - value };
    match good {
        0..=30 => Color::Red,
        31..=60 => Color::Yellow,
        _ => Color::Green,
    }
}

fn draw_player_panel(frame: &mut Frame, game: &Game, area: Rect) {
    let block = Block::bordered().title(" 💰 You ");
    let inner = block.inner(area);
    frame.render_widget(block, area);

    let [money_area, health_area, energy_area, stress_area, warn_area] = Layout::vertical([
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
        gauge(format!("Health {}%", game.player.health), game.player.health, scale_color(game.player.health, true)),
        health_area,
    );
    frame.render_widget(
        gauge(format!("Energy {}%", game.player.energy), game.player.energy, Color::Cyan),
        energy_area,
    );
    frame.render_widget(
        gauge(format!("Stress {}%", game.player.stress), game.player.stress, scale_color(game.player.stress, false)),
        stress_area,
    );

    let mut warnings: Vec<Line> = Vec::new();
    if game.player.health <= constants::CRITICAL_HEALTH_THRESHOLD {
        warnings.push(Line::styled("⚠ CRITICAL — see a doctor!", Style::new().fg(Color::Red).bold()));
    }
    if game.player.is_addicted() {
        warnings.push(Line::styled("⚠ Addiction problem", Style::new().fg(Color::Yellow)));
    }
    frame.render_widget(Paragraph::new(warnings), warn_area);
}

fn draw_band_panel(frame: &mut Frame, game: &Game, area: Rect) {
    let block = Block::bordered().title(" 🎸 Band ");
    let inner = block.inner(area);
    frame.render_widget(block, area);

    let [fame_area, skill_area, rest_area] =
        Layout::vertical([Constraint::Length(1), Constraint::Length(1), Constraint::Min(0)]).areas(inner);

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
            format!("Skill {}% · {}", game.band.skill, game.band.get_skill_level()),
            game.band.skill,
            Color::Blue,
        ),
        skill_area,
    );

    let deal_line = match game.band.current_deal() {
        Some(deal) => Line::from(vec![
            Span::raw("Deal   "),
            Span::styled(
                format!("{} ({}/{} albums)", deal.label_name, deal.albums_delivered, deal.albums_required),
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
        Line::from(format!("Unreleased songs: {}", game.band.unreleased_songs.len())),
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

fn draw_members_panel(frame: &mut Frame, game: &Game, area: Rect) {
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

fn draw_scene_panel(frame: &mut Frame, game: &Game, area: Rect) {
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

    let lines = vec![
        Line::from(format!("Trend      {}", game.world.current_trends)),
        Line::from(format!("Demand     {}%", game.world.music_market.demand)),
        Line::from(format!("Economy    {}", game.world.music_market.economic_state)),
        Line::from(format!("Innovation {}%", era.market_conditions.innovation_openness)),
        Line::from(format!("Bands      {} in the scene", game.world.bands.len())),
        Line::from(format!("Top Act    {}", top_band)),
        Line::from(Span::styled(
            format!("Hot: {}", era.dominant_genres.join(", ")),
            Style::new().fg(Color::Cyan),
        )),
    ];
    frame.render_widget(Paragraph::new(lines).wrap(Wrap { trim: true }), inner);
}

fn draw_menu(frame: &mut Frame, app: &App, area: Rect) {
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
                Style::new().fg(Color::DarkGray).add_modifier(Modifier::ITALIC)
            };
            ListItem::new(Line::from(vec![
                Span::styled(format!(" {} ", entry.hotkey.to_ascii_uppercase()), key_style),
                Span::styled(format!("{:<15}", entry.label), label_style),
                Span::styled(entry.detail.clone(), detail_style),
            ]))
        })
        .collect();

    let list = List::new(items)
        .block(Block::bordered().title(" This Week (↑↓ + Enter, or hotkey) "))
        .highlight_style(Style::new().add_modifier(Modifier::REVERSED));

    let mut state = ListState::default().with_selected(Some(app.menu_selected));
    frame.render_stateful_widget(list, area, &mut state);
}

fn draw_log(frame: &mut Frame, app: &App, area: Rect) {
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

// --- Modals ---

fn draw_deals_modal(frame: &mut Frame, app: &App) {
    let Screen::Deals { selected, detail } = &app.screen else { return };
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
        let lines = vec![
            Line::from(""),
            Line::from(format!("  Advance          {}", format_money(offer.advance as i32))),
            Line::from(format!("  Royalty rate     {:.1}%", offer.royalty_rate * 100.0)),
            Line::from(format!("  Albums required  {}", offer.albums_required)),
            Line::from(""),
            Line::styled("  About the label", Style::new().fg(Color::Cyan).bold()),
            Line::from(format!("  Market reach       {}/100", data.market_reach)),
            Line::from(format!("  Financial power    {}/100", data.financial_power)),
            Line::from(format!("  Artist development {}/100", data.artist_development)),
            Line::from(format!("  Creative freedom   {}/100", data.creative_freedom)),
            Line::from(format!("  Reputation: {}", data.reputation)),
            Line::from(""),
            Line::styled("  [A]ccept · [R]eject · [Esc] back", Style::new().fg(Color::DarkGray)),
        ];
        frame.render_widget(Paragraph::new(lines).wrap(Wrap { trim: false }), inner);
    } else {
        let items: Vec<ListItem> = offers
            .iter()
            .map(|offer| {
                ListItem::new(Line::from(vec![
                    Span::styled(format!("{:<22}", offer.label_name), Style::new().bold()),
                    Span::styled(format!("{:<12}", offer.label_tier), Style::new().fg(Color::Cyan)),
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

fn draw_support_modal(frame: &mut Frame, app: &App) {
    let Some(offer) = &app.game.pending_support_offer else { return };

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
            Span::styled(offer.host_band.clone(), Style::new().fg(Color::Magenta).bold()),
            Span::raw(format!(" (fame {}%) want you as their opening act.", offer.host_fame)),
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

fn draw_marketing_modal(frame: &mut Frame, app: &App) {
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
        Screen::MarketingCampaign { release_name, selected, .. } => {
            let items: Vec<ListItem> = MarketingCampaignType::ALL
                .iter()
                .map(|c| {
                    let spec = c.spec();
                    ListItem::new(Line::from(vec![
                        Span::styled(format!("{:<18}", spec.name), Style::new().bold()),
                        Span::styled(format!("${:<6}", spec.cost), Style::new().fg(Color::Green)),
                        Span::raw(format!("{} weeks · +{} buzz", spec.duration_weeks, spec.effectiveness_bonus)),
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

fn draw_file_modal(frame: &mut Frame, app: &App) {
    let Screen::File { mode, input } = &app.screen else { return };
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
            format!("  empty = {} · Enter confirm · Esc cancel", super::app::SAVE_FILE_DEFAULT),
            Style::new().fg(Color::DarkGray),
        ),
    ];
    frame.render_widget(Paragraph::new(lines), inner);
}

// --- Game over ---

fn draw_game_over(frame: &mut Frame, app: &App) {
    let game = &app.game;
    let won = game.band.fame >= constants::ROCKSTAR_FAME_THRESHOLD
        && game.band.albums_released.len() >= constants::ROCKSTAR_ALBUM_THRESHOLD as usize;

    let (title, color) = if won {
        (" 🌟 YOU'RE A ROCKSTAR 🌟 ", Color::Green)
    } else {
        (" GAME OVER ", ACCENT)
    };

    let area = centered_rect(64, 60, frame.area());
    let block = Block::bordered()
        .title(title)
        .title_style(Style::new().fg(color).bold())
        .border_style(Style::new().fg(color));
    let inner = block.inner(area);
    frame.render_widget(block, area);

    let lines = vec![
        Line::from(""),
        Line::from(game.get_status_message()).centered(),
        Line::from(""),
        Line::from(format!("Career length   {}", calculate_weeks_to_years_months(game.week))).centered(),
        Line::from(format!(
            "Fame            {}% ({})",
            game.band.fame,
            game.band.get_fame_level()
        ))
        .centered(),
        Line::from(format!("Money           {}", format_money(game.player.money))).centered(),
        Line::from(format!(
            "Released        {} single(s), {} album(s)",
            game.band.singles_released.len(),
            game.band.albums_released.len()
        ))
        .centered(),
        Line::from(""),
        Line::styled("Thanks for playing ROCKER — press any key to exit", Style::new().fg(Color::DarkGray))
            .centered(),
    ];
    frame.render_widget(Paragraph::new(lines).wrap(Wrap { trim: false }), inner);
}

// --- Helpers ---

fn centered_rect(percent_x: u16, percent_y: u16, r: Rect) -> Rect {
    let [_, mid, _] = Layout::vertical([
        Constraint::Percentage((100 - percent_y) / 2),
        Constraint::Percentage(percent_y),
        Constraint::Percentage((100 - percent_y) / 2),
    ])
    .areas(r);
    let [_, area, _] = Layout::horizontal([
        Constraint::Percentage((100 - percent_x) / 2),
        Constraint::Percentage(percent_x),
        Constraint::Percentage((100 - percent_x) / 2),
    ])
    .areas(mid);
    area
}

fn draw_venue_picker_modal(frame: &mut Frame, app: &App) {
    let Screen::VenuePicker { selected } = app.screen else { return };
    let area = centered_rect(74, 50, frame.area());
    frame.render_widget(Clear, area);

    let items: Vec<ListItem> = app.game.world.venues
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
                Span::styled(format!("  Base Pay: {:<6}", format_money(venue.base_payment as i32)), style),
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

fn draw_pressing_picker_modal(frame: &mut Frame, app: &App) {
    let Screen::PressingPicker { release_type, selected } = app.screen else { return };
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
                    format!("  Total with studio: {:<8}", format_money(recording + pressing)),
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

fn draw_region_picker_modal(frame: &mut Frame, app: &App) {
    let Screen::RegionPicker { selected } = app.screen else { return };
    let area = centered_rect(88, 60, frame.area());
    frame.render_widget(Clear, area);

    let sorted_regions = app.game.get_sorted_regions();
    let items: Vec<ListItem> = sorted_regions
        .iter()
        .map(|(country_key, region_key, region_name, population, economic_strength, fame_req)| {
            let locked = app.game.band.fame < *fame_req;
            let regional_fame_key = format!("{}:{}", country_key, region_key);
            let regional_fame = *app.game.regional_fame.get(&regional_fame_key).unwrap_or(&0);
            
            let status = if locked {
                Span::styled(format!(" 🔒 Req Fame: {}", fame_req), Style::new().fg(Color::DarkGray))
            } else {
                Span::styled(format!(" 🔓 Reg Fame: {}%", regional_fame), Style::new().fg(Color::Green))
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
            let cost_str = if let Some(touring_costs) = app.game.data_files.markets_data.market_modifiers.touring_costs.get(tier_name) {
                let cost = (touring_costs.base_cost_per_show as f32 * country_travel_mult) as i32;
                format_money(cost)
            } else {
                "N/A".to_string()
            };

            let country_name = country_key.replace("_", " ");
            ListItem::new(Line::from(vec![
                Span::styled(format!("{:<15}", region_name), style.bold()),
                Span::styled(format!(" ({:<15})", country_name), Style::new().fg(Color::Cyan)),
                Span::styled(format!("  Pop: {:>8}", format_population(*population)), style),
                Span::styled(format!("  Econ: {:>3}", economic_strength), style),
                Span::styled(format!("  Cost: {:>6}", cost_str), if locked { style } else { Style::new().fg(Color::Yellow) }),
                Span::raw("  "),
                status,
            ]))
        })
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

fn format_population(pop: u32) -> String {
    if pop >= 1_000_000 {
        format!("{:.1}M", pop as f32 / 1_000_000.0)
    } else {
        format!("{}k", pop / 1000)
    }
}
