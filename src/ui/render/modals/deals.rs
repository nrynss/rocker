//! Deal offer detail and support-slot offer overlays.

use ratatui::{
    Frame,
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Clear, List, ListItem, ListState, Paragraph, Wrap},
};

use crate::data::format_money;
use crate::ui::app::{App, Screen};

use super::super::centered_rect;
pub(crate) fn draw_deals_modal(frame: &mut Frame, app: &App) {
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

pub(crate) fn draw_support_modal(frame: &mut Frame, app: &App) {
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
