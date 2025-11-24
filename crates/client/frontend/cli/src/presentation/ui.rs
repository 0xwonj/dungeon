//! UI rendering using new widget architecture with ViewModel.
//!
//! This module provides the main render entry point that composes all widgets
//! to create the complete terminal UI.
use anyhow::Result;
use game_core::env::MapOracle;
use ratatui::layout::{Alignment, Constraint, Direction, Layout};

use crate::{
    presentation::{terminal::Tui, theme::RatatuiTheme, widgets},
    state::{ActionSlots, AppMode, AppState},
};
use client_frontend_core::{message::MessageLog, view_model::ViewModel};

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
/// This function routes rendering based on the current app mode:
/// - **Full-screen modes**: Completely replace the game UI (SaveMenu, Inventory, etc.)
/// - **Overlay modes**: Render game UI with a modal on top (AbilityMenu)
/// - **Standard modes**: Render normal game UI (Normal, Examine, Targeting)
///
/// All widgets consume ViewModel directly with no adapter layers.
pub fn render_with_view_model(terminal: &mut Tui, ctx: &RenderContext) -> Result<()> {
    let theme = RatatuiTheme;

    terminal.draw(|frame| {
        // Route to full-screen modes first
        if ctx.app_state.mode.is_fullscreen() {
            render_fullscreen_mode(frame, ctx);
            return;
        }

        // Otherwise render standard game UI
        render_game_ui(frame, ctx, &theme);

        // Apply overlays on top of game UI
        if ctx.app_state.mode.is_overlay() {
            render_overlay_mode(frame, ctx);
        }
    })?;

    Ok(())
}

/// Render full-screen mode UI (replaces game view entirely).
fn render_fullscreen_mode(frame: &mut ratatui::Frame, ctx: &RenderContext) {
    match &ctx.app_state.mode {
        AppMode::StartScreen(start_state) => {
            widgets::start_screen::render_start_screen(frame, frame.area(), start_state);
        }
        AppMode::SaveMenu(menu_state) => {
            widgets::save_menu::render_fullscreen(
                frame,
                frame.area(),
                menu_state,
                &ctx.app_state.save_menu_log,
            );
        }
        AppMode::Inventory => {
            // TODO: Implement inventory screen
            // For now, show placeholder
            let placeholder = ratatui::widgets::Paragraph::new("Inventory (not implemented)")
                .alignment(Alignment::Center)
                .block(
                    ratatui::widgets::Block::default()
                        .borders(ratatui::widgets::Borders::ALL)
                        .title(" Inventory "),
                );
            frame.render_widget(placeholder, frame.area());
        }
        _ => {
            // Should never reach here due to is_fullscreen() guard
            unreachable!("render_fullscreen_mode called with non-fullscreen mode")
        }
    }
}

/// Render overlay mode UI (on top of game view).
fn render_overlay_mode(frame: &mut ratatui::Frame, ctx: &RenderContext) {
    match ctx.app_state.mode {
        AppMode::AbilityMenu => {
            // Center the ability menu overlay
            let area = centered_rect(60, 80, frame.area());
            widgets::ability_menu::render(frame, area, ctx.available_actions, ctx.action_slots);
        }
        _ => {
            // Should never reach here due to is_overlay() guard
            unreachable!("render_overlay_mode called with non-overlay mode")
        }
    }
}

/// Render standard game UI (header, game area, messages, action slots, footer).
///
/// This is the default UI shown during Normal, Examine, and Targeting modes.
fn render_game_ui(frame: &mut ratatui::Frame, ctx: &RenderContext, theme: &RatatuiTheme) {
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
        theme,
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
        theme,
    );

    // Action slots bar
    widgets::action_slots::render(frame, chunks[3], ctx.action_slots);

    widgets::footer::render(frame, chunks[4], ctx.app_state);
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
