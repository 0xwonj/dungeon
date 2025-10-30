//! Footer widget displaying context-sensitive key bindings.

use ratatui::{
    Frame,
    layout::Rect,
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph},
};

use crate::state::{AppMode, AppState};

/// Render the footer panel with key bindings help.
///
/// Displays context-sensitive controls based on current app mode.
pub fn render(frame: &mut Frame, area: Rect, app_state: &AppState) {
    let text = match app_state.mode {
        AppMode::Normal => vec![Line::from(vec![
            Span::raw("[hjkl/WASD/Arrows] Move | "),
            Span::raw("[Space/Enter/.] Wait | "),
            Span::raw("[x] Manual examine | "),
            Span::raw("[Tab] Cycle | "),
            Span::raw("[q] Quit"),
        ])],
        AppMode::ExamineManual => vec![Line::from(vec![
            Span::raw("[hjkl/Arrows] Move cursor | "),
            Span::raw("[Tab] Next entity | "),
            Span::raw("[Shift+Tab] Prev | "),
            Span::raw("[x/ESC] Back"),
        ])],
        _ => vec![Line::from(vec![Span::raw("[ESC] Exit mode")])],
    };

    let paragraph = Paragraph::new(text).block(Block::default().borders(Borders::ALL));

    frame.render_widget(paragraph, area);
}
