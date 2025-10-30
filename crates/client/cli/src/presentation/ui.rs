//! UI rendering using new widget architecture with ViewModel.
//!
//! This module provides the main render entry point that composes all widgets
//! to create the complete terminal UI.
use anyhow::Result;
use game_core::env::MapOracle;
use ratatui::layout::{Constraint, Direction, Layout};

use crate::{
    presentation::{terminal::Tui, theme::RatatuiTheme, widgets},
    state::AppState,
};
use client_core::{message::MessageLog, view_model::ViewModel};

/// Render the terminal UI using ViewModel and widget system.
///
/// This function composes all widgets to create the complete UI:
/// - Header: turn clock, current actor, mode indicator
/// - Game area: map (60%), player stats (20%), examine panel (20%)
/// - Messages: recent message log
/// - Footer: context-sensitive key bindings
///
/// All widgets consume ViewModel directly with no adapter layers.
pub fn render_with_view_model(
    terminal: &mut Tui,
    view_model: &ViewModel,
    messages: &MessageLog,
    app_state: &AppState,
    map: &dyn MapOracle,
) -> Result<()> {
    let theme = RatatuiTheme;

    terminal.draw(|frame| {
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(3),                                       // Header
                Constraint::Min(0),                                          // Game area
                Constraint::Length(widgets::messages::MESSAGE_PANEL_HEIGHT), // Messages
                Constraint::Length(2),                                       // Footer
            ])
            .split(frame.area());

        widgets::header::render(frame, chunks[0], view_model, app_state);

        widgets::game_area::render(frame, chunks[1], view_model, app_state, map, &theme);

        let recent_messages: Vec<_> = messages
            .recent(widgets::messages::MESSAGE_PANEL_HEIGHT as usize)
            .cloned()
            .collect();
        widgets::messages::render(frame, chunks[2], &recent_messages, &theme);

        widgets::footer::render(frame, chunks[3], app_state);
    })?;

    Ok(())
}
