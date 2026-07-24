//! Deal offer detail and support-slot offer overlays.

use ratatui::{
    Frame,
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Clear, List, ListItem, ListState, Paragraph, Wrap},
};

use crate::data::format_money;
use crate::game::world::{Label, LabelStatus};
use crate::ui::app::{App, Screen};

use super::super::centered_rect;
// L-UI-2 (§H): the deal picker reads an offering label's *current* health with
// the same qualitative vocabulary as the label ledger — never a rival's exact
// figure. Single source of truth, shared with `labels.rs`.
use super::labels::{capital_band, tier_label};

/// The health annotation for an offering label (design §H): its qualitative
/// capital band (colour-coded, never an exact rival figure) plus a distress
/// flag when the label is in trouble. Shared by the offer list and the detail
/// view so an offer reads the same in both. A folded/acquired label would not
/// be tabling offers, so only the live `Distressed` state is flagged.
fn health_spans(label: &Label) -> Vec<Span<'static>> {
    let (word, color) = capital_band(label.capital);
    let mut spans = vec![Span::styled(word.to_string(), Style::new().fg(color))];
    if label.status == LabelStatus::Distressed {
        spans.push(Span::styled(" ⚠ distressed", Style::new().fg(Color::Red)));
    }
    spans
}
pub(crate) fn draw_deals_modal(frame: &mut Frame, app: &App) {
    let Screen::Deals { selected, detail } = &app.screen else {
        return;
    };
    // Wider than the v0.7 picker (was 72%): L-UI-2 (§H) annotates each offer
    // row with the offering label's live health, which needs the extra columns.
    let area = centered_rect(82, 72, frame.area());
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
        ]);

        // L-UI-2 (§H): the label's *current* standing — its live tier, reach,
        // and a health read — so "rising indie vs safe major" is legible before
        // signing. Read from the live world entity (the block above is the
        // static offer-time snapshot); a pre-0.8 save with no seeded labels
        // simply omits this section.
        if let Some(label) = app.label_by_name(&offer.label_name) {
            lines.push(Line::from(""));
            lines.push(Line::styled(
                "  Where they stand now",
                Style::new().fg(Color::Cyan).bold(),
            ));
            let mut standing = vec![
                Span::raw("  "),
                Span::styled(tier_label(label.tier).to_string(), Style::new().bold()),
                Span::styled("  ·  ", Style::new().fg(Color::DarkGray)),
                Span::styled(
                    format!("reach {}/100", label.market_reach),
                    Style::new().fg(Color::White),
                ),
                Span::styled("  ·  ", Style::new().fg(Color::DarkGray)),
            ];
            standing.extend(health_spans(label));
            lines.push(Line::from(standing));
        }

        lines.extend([
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
                let mut spans = vec![
                    Span::styled(format!("{:<20}", offer.label_name), Style::new().bold()),
                    Span::styled(
                        format!("{:<12}", offer.label_tier),
                        Style::new().fg(Color::Cyan),
                    ),
                    Span::raw(format!(
                        "{} adv · {:.0}% · {} alb",
                        format_money(offer.advance as i32),
                        offer.royalty_rate * 100.0,
                        offer.albums_required
                    )),
                ];
                // L-UI-2 (§H): a health read on the offering label's live
                // balance sheet, so "rising indie vs safe major" is legible at
                // the table. Omitted (row unchanged) when no live label backs
                // the offer — e.g. a pre-0.8 save's JSON-priced offer.
                if let Some(label) = app.label_by_name(&offer.label_name) {
                    spans.push(Span::styled("  ·  ", Style::new().fg(Color::DarkGray)));
                    spans.extend(health_spans(label));
                }
                ListItem::new(Line::from(spans))
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
