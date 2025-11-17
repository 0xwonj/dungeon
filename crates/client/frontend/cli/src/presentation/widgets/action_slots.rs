//! Action slots widget for displaying hotkey bindings.

use ratatui::{
    Frame,
    layout::Rect,
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph},
};

use crate::state::ActionSlots;

/// Render the action slots bar (hotkey display).
///
/// Shows current action assignments for keys 1-9.
pub fn render(frame: &mut Frame, area: Rect, action_slots: &ActionSlots) {
    let mut spans = vec![];

    for slot in 0..9 {
        let slot_num = slot + 1;

        // Get action assigned to this slot
        let action = action_slots.get(slot);

        let (key_style, action_text, action_style) = if let Some(action_kind) = action {
            (
                Style::default()
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::BOLD),
                format!("{:?}", action_kind),
                Style::default().fg(Color::White),
            )
        } else {
            (
                Style::default().fg(Color::DarkGray),
                "---".to_string(),
                Style::default().fg(Color::DarkGray),
            )
        };

        spans.push(Span::styled(format!("{}", slot_num), key_style));
        spans.push(Span::raw(":"));
        spans.push(Span::styled(action_text, action_style));

        if slot < 8 {
            spans.push(Span::raw(" | "));
        }
    }

    let line = Line::from(spans);

    let paragraph = Paragraph::new(line).block(
        Block::default()
            .borders(Borders::ALL)
            .title(" Action Slots "),
    );

    frame.render_widget(paragraph, area);
}
