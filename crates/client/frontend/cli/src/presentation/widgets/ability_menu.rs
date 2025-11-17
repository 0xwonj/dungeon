//! Ability menu widget for viewing and assigning actions to slots.

use ratatui::{
    Frame,
    layout::{Alignment, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph, Wrap},
};

use crate::state::ActionSlots;

/// Render the ability menu overlay.
///
/// Displays available actions and current slot assignments.
/// User can press 1-9 to assign actions to slots.
pub fn render(
    frame: &mut Frame,
    area: Rect,
    available_actions: &[game_core::ActionKind],
    action_slots: &ActionSlots,
) {
    let mut lines = vec![
        Line::from(vec![Span::styled(
            "Ability Menu",
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        )]),
        Line::from(""),
        Line::from(vec![Span::styled(
            "Available Actions:",
            Style::default().fg(Color::Yellow),
        )]),
        Line::from(""),
    ];

    // Display available actions
    for (i, &action_kind) in available_actions.iter().enumerate() {
        let slot_num = i + 1;

        // Check if this action is already assigned to this slot
        let assigned = action_slots.get(i) == Some(action_kind);

        let style = if assigned {
            Style::default().fg(Color::Green)
        } else {
            Style::default().fg(Color::White)
        };

        let assigned_marker = if assigned { " [ASSIGNED]" } else { "" };

        lines.push(Line::from(vec![
            Span::styled(
                format!("  {}. ", slot_num),
                Style::default().fg(Color::Yellow),
            ),
            Span::styled(format!("{:?}{}", action_kind, assigned_marker), style),
        ]));
    }

    lines.push(Line::from(""));
    lines.push(Line::from(vec![Span::styled(
        "Press 1-9 to assign action to slot | ESC to close",
        Style::default()
            .fg(Color::Gray)
            .add_modifier(Modifier::ITALIC),
    )]));

    let paragraph = Paragraph::new(lines)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::Cyan))
                .title(" Abilities ")
                .title_alignment(Alignment::Center),
        )
        .wrap(Wrap { trim: false });

    frame.render_widget(paragraph, area);
}
