//! Start screen widget for New Game / Continue selection.

use ratatui::{
    Frame,
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, Paragraph},
};

use crate::state::StartScreenState;

/// Renders the start screen (New Game / Continue).
pub fn render_start_screen(frame: &mut Frame, area: Rect, state: &StartScreenState) {
    // Main layout: title, menu with sessions, footer
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(5), // Title banner
            Constraint::Min(0),    // Menu options + sessions
            Constraint::Length(3), // Footer
        ])
        .split(area);

    // Title banner
    render_title(frame, chunks[0]);

    // Menu options with session list
    render_menu_with_sessions(frame, chunks[1], state);

    // Footer instructions
    render_footer(frame, chunks[2]);
}

fn render_title(frame: &mut Frame, area: Rect) {
    let title = Paragraph::new(vec![
        Line::from(""),
        Line::from(vec![Span::styled(
            "DUNGEON",
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        )]),
        Line::from(vec![Span::styled(
            "Zero-Knowledge Roguelike",
            Style::default().fg(Color::Gray),
        )]),
    ])
    .alignment(Alignment::Center)
    .block(
        Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::Cyan)),
    );

    frame.render_widget(title, area);
}

fn render_menu_with_sessions(frame: &mut Frame, area: Rect, state: &StartScreenState) {
    let mut menu_items = vec![];

    // First item: New Game
    menu_items.push(ListItem::new(Line::from(vec![
        Span::styled(
            if state.selected == 0 { "► " } else { "  " },
            Style::default().fg(Color::Yellow),
        ),
        Span::styled(
            "New Game",
            if state.selected == 0 {
                Style::default()
                    .fg(Color::Cyan)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(Color::Cyan)
            },
        ),
    ])));

    // Add separator
    if !state.sessions.is_empty() {
        menu_items.push(ListItem::new(Line::from(vec![Span::styled(
            "  ─────────────────────",
            Style::default().fg(Color::DarkGray),
        )])));
    }

    // Add sessions
    for (idx, session) in state.sessions.iter().enumerate() {
        let is_selected = state.selected == idx + 1;

        menu_items.push(ListItem::new(Line::from(vec![
            Span::styled(
                if is_selected { "► " } else { "  " },
                Style::default().fg(Color::Yellow),
            ),
            Span::styled(
                &session.session_id,
                if is_selected {
                    Style::default()
                        .fg(Color::White)
                        .add_modifier(Modifier::BOLD)
                } else {
                    Style::default().fg(Color::White)
                },
            ),
            Span::styled(
                format!(" (nonce {})", session.latest_nonce),
                if is_selected {
                    Style::default().fg(Color::Green)
                } else {
                    Style::default().fg(Color::DarkGray)
                },
            ),
        ])));
    }

    let list = List::new(menu_items).block(
        Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::Cyan))
            .title(" Choose Game ")
            .title_alignment(Alignment::Center),
    );

    frame.render_widget(list, area);
}

fn render_footer(frame: &mut Frame, area: Rect) {
    let footer = Paragraph::new(vec![
        Line::from(""),
        Line::from(vec![
            Span::styled("↑/↓", Style::default().fg(Color::Yellow)),
            Span::styled(" Navigate  ", Style::default().fg(Color::Gray)),
            Span::styled("Enter", Style::default().fg(Color::Yellow)),
            Span::styled(" Select  ", Style::default().fg(Color::Gray)),
            Span::styled("q/Esc", Style::default().fg(Color::Yellow)),
            Span::styled(" Quit", Style::default().fg(Color::Gray)),
        ]),
    ])
    .alignment(Alignment::Center)
    .block(Block::default().borders(Borders::NONE));

    frame.render_widget(footer, area);
}
