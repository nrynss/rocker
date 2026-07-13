use ratatui::{
    Frame,
    layout::{Constraint, Layout},
};

use crate::ui::app::App;

use super::panels;

pub(super) fn draw_main(frame: &mut Frame, app: &mut App) {
    // The stats band fits the scene panel's eight lines (chart line included).
    let [header_area, stats_area, bottom_area] = Layout::vertical([
        Constraint::Length(4),
        Constraint::Length(11),
        Constraint::Min(8),
    ])
    .areas(frame.area());

    panels::draw_header(frame, &app.game, header_area);

    let [player_area, band_area, members_area, scene_area] =
        Layout::horizontal([Constraint::Percentage(25); 4]).areas(stats_area);
    panels::draw_player_panel(frame, &app.game, player_area);
    panels::draw_band_panel(frame, &app.game, band_area);
    panels::draw_members_panel(frame, &app.game, members_area);
    panels::draw_scene_panel(frame, &app.game, scene_area);

    let [menu_area, log_area] =
        Layout::horizontal([Constraint::Percentage(42), Constraint::Percentage(58)])
            .areas(bottom_area);
    panels::draw_menu(frame, app, menu_area);
    panels::draw_log(frame, app, log_area);
}
