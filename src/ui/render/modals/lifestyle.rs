//! Lifestyle tier picker — where the player lives, upkeep, and the stat
//! effects (design §B). Moving is always the player's call; the modal
//! shows the one-shot happiness swing before committing.

use ratatui::{
    Frame,
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Clear, List, ListItem, ListState},
};

use crate::data::format_money;
use crate::game::player::LifestyleTier;
use crate::ui::app::{App, Screen};

use super::super::centered_rect;

pub(crate) fn draw_lifestyle_picker_modal(frame: &mut Frame, app: &App) {
    let Screen::LifestylePicker { selected } = app.screen else {
        return;
    };
    let area = centered_rect(86, 60, frame.area());
    frame.render_widget(Clear, area);

    let current = app.game.player.lifestyle;
    let items: Vec<ListItem> = LifestyleTier::ALL
        .iter()
        .map(|&tier| {
            let is_current = tier == current;
            let move_desc = if is_current {
                "current home".to_string()
            } else if tier > current {
                format!(
                    "move up: {} up front, happiness +{}",
                    format_money(tier.move_up_cost() as i32),
                    LifestyleTier::MOVE_UP_HAPPINESS
                )
            } else {
                format!(
                    "move down: free, happiness -{}",
                    LifestyleTier::MOVE_DOWN_HAPPINESS
                )
            };

            let style = if is_current {
                Style::new().fg(Color::Green)
            } else {
                Style::new().fg(Color::White)
            };

            ListItem::new(Line::from(vec![
                Span::styled(format!("{:<16}", tier.label()), style.bold()),
                Span::styled(
                    format!(
                        "  Upkeep: {:<8}",
                        format_money(tier.upkeep_per_week() as i32)
                    ),
                    style,
                ),
                Span::styled(
                    format!("  Stress +{:<2}", tier.stress_release_bonus()),
                    style,
                ),
                Span::styled(format!("  Floor {:<3}", tier.happiness_floor()), style),
                Span::styled(format!("  Rest +{:<2}", tier.rest_healing_bonus()), style),
                Span::raw("  "),
                Span::styled(move_desc, Style::new().fg(Color::Yellow)),
            ]))
        })
        .collect();

    let list = List::new(items)
        .block(
            Block::bordered()
                .title(" 🏠 Where You Live ")
                .title_bottom(" Enter move · Esc close "),
        )
        .highlight_style(Style::new().add_modifier(Modifier::REVERSED));
    let mut state = ListState::default().with_selected(Some(selected));
    frame.render_stateful_widget(list, area, &mut state);
}
