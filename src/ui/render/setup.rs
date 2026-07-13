use ratatui::{
    Frame,
    style::{Color, Style},
    text::{Line, Span},
    widgets::{Block, Paragraph, Wrap},
};

use crate::data::constants;
use crate::game::genre::MusicGenre;

use crate::ui::app::{App, Screen, SetupField};

use super::ACCENT;

pub(super) fn draw_setup(frame: &mut Frame, app: &App) {
    let Screen::Setup { field } = &app.screen else {
        return;
    };
    let picking_genre = *field == SetupField::Genre;

    // The genre list needs more room than the two name prompts.
    let area = super::centered_rect(60, if picking_genre { 75 } else { 50 }, frame.area());
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
            Span::styled(
                format!("{}{}", value, cursor),
                Style::new().fg(Color::White),
            ),
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
        field_line(
            "Band name:",
            &app.band_input,
            *field == SetupField::BandName,
        ),
    ];

    if picking_genre {
        lines.push(Line::from(""));
        lines.push(Line::styled(
            "What do you play?",
            Style::new().fg(Color::Yellow).bold(),
        ));
        for (i, genre) in MusicGenre::ALL.iter().enumerate() {
            let (marker, style) = if i == app.genre_selected {
                ("▸", Style::new().fg(Color::Yellow).bold())
            } else {
                (" ", Style::new().fg(Color::DarkGray))
            };
            lines.push(Line::styled(
                format!("  {} {}", marker, genre.name()),
                style,
            ));
        }
        lines.push(Line::from(""));
        lines.push(
            Line::styled(
                "↑↓ choose · Enter confirm · Esc quit",
                Style::new().fg(Color::DarkGray),
            )
            .centered(),
        );
    } else {
        lines.push(Line::from(""));
        lines.push(
            Line::styled(
                "Enter to confirm · Esc to quit",
                Style::new().fg(Color::DarkGray),
            )
            .centered(),
        );
    }

    frame.render_widget(Paragraph::new(lines).wrap(Wrap { trim: false }), inner);
}
