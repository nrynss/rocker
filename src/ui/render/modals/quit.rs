//! The save-first quit prompt (`q`): save and quit, save and keep playing,
//! or cancel. Nothing here ends the career — leaving just closes the app.

use ratatui::{
    Frame,
    layout::Rect,
    style::{Color, Style},
    text::{Line, Span},
    widgets::{Block, Clear, Paragraph, Wrap},
};

use crate::ui::app::{App, Screen};

pub(crate) fn draw_confirm_quit_modal(frame: &mut Frame, app: &App) {
    let Screen::ConfirmQuit { error } = &app.screen else {
        return;
    };

    // Fixed size, like the walk-out box: the save path (≤40 characters) and
    // a save error each wrap to two lines at most at this width, and fourteen
    // rows hold every line at once, so the key hints are never clipped on
    // an 80x24 terminal.
    let screen = frame.area();
    let width = screen.width.min(64);
    let height = screen.height.min(14);
    let area = Rect {
        x: screen.x + (screen.width - width) / 2,
        y: screen.y + (screen.height - height) / 2,
        width,
        height,
    };
    frame.render_widget(Clear, area);
    let block = Block::bordered()
        .title(" 🚪 Quit? ")
        .title_style(Style::new().fg(Color::Yellow).bold());
    let inner = block.inner(area);
    frame.render_widget(block, area);

    let key = Style::new().fg(Color::Yellow).bold();
    let mut lines = vec![
        Line::from(""),
        Line::from(vec![
            Span::raw(" Save to "),
            Span::styled(app.save_path.clone(), Style::new().fg(Color::Cyan)),
            Span::raw(" first (replaces any save there)."),
        ]),
    ];
    if let Some(activity) = app.game.ongoing_activity() {
        lines.push(Line::styled(
            format!(
                " The {} is saved with it and resumes on load.",
                activity.label().to_lowercase()
            ),
            Style::new().fg(Color::DarkGray),
        ));
    }
    lines.push(Line::from(""));
    lines.push(Line::from(vec![
        Span::styled(" [S] ", key),
        Span::raw("save and quit"),
    ]));
    lines.push(Line::from(vec![
        Span::styled(" [C] ", key),
        Span::raw("save and continue playing"),
    ]));
    lines.push(Line::from(vec![
        Span::styled(" [N]/[Esc] ", key),
        Span::raw("cancel — nothing saved"),
    ]));
    if let Some(error) = error {
        lines.push(Line::from(""));
        lines.push(Line::styled(
            format!(" ❌ {error} — not quitting."),
            Style::new().fg(Color::Red),
        ));
    }
    frame.render_widget(Paragraph::new(lines).wrap(Wrap { trim: false }), inner);
}
