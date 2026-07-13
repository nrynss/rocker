//! Save / load path modal.

use ratatui::{
    Frame,
    style::{Color, Style},
    text::{Line, Span},
    widgets::{Block, Clear, Paragraph},
};

use crate::ui::app::{App, FileMode, Screen};

use super::super::centered_rect;
pub(crate) fn draw_file_modal(frame: &mut Frame, app: &App) {
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
