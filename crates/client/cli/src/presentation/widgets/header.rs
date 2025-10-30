//! Header widget displaying turn information and game mode.

use client_core::view_model::ViewModel;
use ratatui::{
    Frame,
    layout::Rect,
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph},
};

use crate::state::{AppMode, AppState};

/// Render the header panel with turn info and current mode.
///
/// Displays turn clock, current actor, active actor count, and app mode.
pub fn render(frame: &mut Frame, area: Rect, view_model: &ViewModel, app_state: &AppState) {
    let mode_text = match app_state.mode {
        AppMode::Normal => "",
        AppMode::ExamineManual => " [EXAMINE - MANUAL]",
        AppMode::Targeting { .. } => " [TARGETING]",
        AppMode::Inventory => " [INVENTORY]",
    };

    let text = vec![Line::from(vec![
        Span::raw("Time: "),
        Span::styled(
            view_model.turn.clock.to_string(),
            Style::default().fg(Color::Yellow),
        ),
        Span::raw(" | Actor: "),
        Span::styled(
            format!("{:?}", view_model.turn.current_actor),
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        ),
        Span::raw(" | Active: "),
        Span::styled(
            view_model.turn.active_actors.len().to_string(),
            Style::default().fg(Color::LightGreen),
        ),
        Span::styled(
            mode_text,
            Style::default()
                .fg(Color::Magenta)
                .add_modifier(Modifier::BOLD),
        ),
    ])];

    let paragraph =
        Paragraph::new(text).block(Block::default().borders(Borders::ALL).title("Game"));

    frame.render_widget(paragraph, area);
}
