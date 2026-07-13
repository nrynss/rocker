use ratatui::{
    Frame,
    style::{Color, Style},
    text::Line,
    widgets::{Block, Paragraph, Wrap},
};

use crate::data::{calculate_weeks_to_years_months, constants, format_money};

use crate::ui::app::App;

use super::{ACCENT, centered_rect};

pub(super) fn draw_game_over(frame: &mut Frame, app: &App) {
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
        Line::from(format!(
            "Career length   {}",
            calculate_weeks_to_years_months(game.week)
        ))
        .centered(),
        Line::from(format!(
            "Fame            {}% ({})",
            game.band.fame,
            game.band.get_fame_level()
        ))
        .centered(),
        Line::from(format!(
            "Money           {}",
            format_money(game.player.money)
        ))
        .centered(),
        Line::from(format!(
            "Released        {} single(s), {} album(s)",
            game.band.singles_released.len(),
            game.band.albums_released.len()
        ))
        .centered(),
        Line::from(""),
        Line::styled(
            "Thanks for playing ROCKER — press any key to exit",
            Style::new().fg(Color::DarkGray),
        )
        .centered(),
    ];
    frame.render_widget(Paragraph::new(lines).wrap(Wrap { trim: false }), inner);
}
