//! Game area widget orchestrating map, player stats, and examine panels.

use client_frontend_core::view_model::{PresentationMapper, ViewModel};
use game_core::env::MapOracle;
use ratatui::{
    Frame,
    layout::{Constraint, Direction, Layout, Rect},
    style::Style,
};

use crate::state::AppState;

use super::{examine, map, player_stats};

/// Render the main game area with 3-column layout: Map | Player Stats | Examine.
///
/// This widget orchestrates the three main gameplay panels:
/// - Left (60%): Map grid with entities
/// - Middle (20%): Player statistics
/// - Right (20%): Examine/inspection panel
pub fn render<T: PresentationMapper<Style = Style>>(
    frame: &mut Frame,
    area: Rect,
    view_model: &ViewModel,
    app_state: &AppState,
    map_oracle: &dyn MapOracle,
    theme: &T,
) {
    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage(60), // Map
            Constraint::Percentage(20), // Player stats
            Constraint::Percentage(20), // Examine
        ])
        .split(area);

    // Render map with PresentationMapper
    map::render(frame, chunks[0], view_model, app_state, theme);

    // Render player stats with PresentationMapper
    player_stats::render(frame, chunks[1], view_model, theme);

    // Render examine panel (always visible in Normal mode or when cursor active)
    let examine_ctx = examine::ExamineContext {
        highlighted_entity: app_state.highlighted_entity,
        cursor_position: app_state.examine_position(),
        is_manual: app_state.is_manual_cursor(),
    };
    examine::render(
        frame,
        chunks[2],
        &examine_ctx,
        view_model,
        map_oracle,
        theme,
    );
}
