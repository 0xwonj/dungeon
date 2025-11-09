//! UI rendering using new widget architecture with ViewModel.
//!
//! This module provides the main render entry point that composes all widgets
//! to create the complete terminal UI.
use anyhow::Result;
use game_core::env::MapOracle;
use ratatui::layout::{Constraint, Direction, Layout};

use crate::{
    presentation::{terminal::Tui, theme::RatatuiTheme, widgets},
    state::{ActionSlots, AppMode, AppState},
};
use client_core::{message::MessageLog, view_model::ViewModel};

/// Rendering context containing all state and configuration needed for UI rendering.
pub struct RenderContext<'a> {
    pub view_model: &'a ViewModel,
    pub messages: &'a MessageLog,
    pub app_state: &'a AppState,
    pub action_slots: &'a ActionSlots,
    pub available_actions: &'a [game_core::ActionKind],
    pub message_panel_height: u16,
    pub map: &'a dyn MapOracle,
}

/// Render the terminal UI using ViewModel and widget system.
///
/// This function composes all widgets to create the complete UI:
/// - Header: turn clock, current actor, mode indicator
/// - Game area: map (60%), player stats (20%), examine panel (20%)
/// - Messages: recent message log
/// - Action slots: hotkey display (1-9)
/// - Footer: context-sensitive key bindings
/// - Overlays: ability menu, targeting (modal)
///
/// All widgets consume ViewModel directly with no adapter layers.
pub fn render_with_view_model(terminal: &mut Tui, ctx: &RenderContext) -> Result<()> {
    let theme = RatatuiTheme;

    terminal.draw(|frame| {
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(3),                        // Header
                Constraint::Min(0),                           // Game area
                Constraint::Length(ctx.message_panel_height), // Messages
                Constraint::Length(3),                        // Action slots
                Constraint::Length(2),                        // Footer
            ])
            .split(frame.area());

        widgets::header::render(frame, chunks[0], ctx.view_model, ctx.app_state);

        widgets::game_area::render(
            frame,
            chunks[1],
            ctx.view_model,
            ctx.app_state,
            ctx.map,
            &theme,
        );

        let recent_messages: Vec<_> = ctx
            .messages
            .recent(ctx.message_panel_height as usize)
            .cloned()
            .collect();
        widgets::messages::render(
            frame,
            chunks[2],
            &recent_messages,
            ctx.message_panel_height,
            &theme,
        );

        // Action slots bar
        widgets::action_slots::render(frame, chunks[3], ctx.action_slots);

        widgets::footer::render(frame, chunks[4], ctx.app_state);

        // Modal overlays (rendered on top)
        if ctx.app_state.mode == AppMode::AbilityMenu {
            // Center the ability menu
            let area = centered_rect(60, 80, frame.area());
            widgets::ability_menu::render(frame, area, ctx.available_actions, ctx.action_slots);
        }
        // Targeting mode now uses in-game cursor visualization instead of overlay
    })?;

    Ok(())
}

/// Create a centered rectangle for modal overlays.
fn centered_rect(
    percent_x: u16,
    percent_y: u16,
    r: ratatui::layout::Rect,
) -> ratatui::layout::Rect {
    use ratatui::layout::{Constraint, Direction, Layout};

    let popup_layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage((100 - percent_y) / 2),
            Constraint::Percentage(percent_y),
            Constraint::Percentage((100 - percent_y) / 2),
        ])
        .split(r);

    Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage((100 - percent_x) / 2),
            Constraint::Percentage(percent_x),
            Constraint::Percentage((100 - percent_x) / 2),
        ])
        .split(popup_layout[1])[1]
}
