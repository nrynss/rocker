//! Marketing release and campaign pickers.

use ratatui::{
    Frame,
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Clear, List, ListItem, ListState},
};

use crate::game::music::MarketingCampaignType;
use crate::ui::app::{App, Screen};

use super::super::centered_rect;
pub(crate) fn draw_marketing_modal(frame: &mut Frame, app: &App) {
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
