//! Start screen for choosing New Game or Continue.

use anyhow::Result;
use crossterm::event::{self, Event, KeyCode, KeyEvent};
use ratatui::{
    Frame, Terminal,
    backend::CrosstermBackend,
    layout::{Alignment, Constraint, Direction, Layout},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, Paragraph},
};

use client_bootstrap::SessionInfo;

/// User's choice from the start screen.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum StartChoice {
    /// Start a new game
    NewGame,
    /// Continue from existing session (with session index)
    Continue(usize),
}

/// Show the start screen and get user's choice.
///
/// If sessions exist, shows "New Game" and session list.
/// If no sessions exist, automatically returns NewGame.
pub fn show_start_screen(
    terminal: &mut Terminal<CrosstermBackend<std::io::Stdout>>,
    sessions: &[SessionInfo],
) -> Result<StartChoice> {
    // If no sessions, automatically start new game
    if sessions.is_empty() {
        return Ok(StartChoice::NewGame);
    }

    // State: 0 = New Game selected, 1+ = session index + 1
    let mut selected = 0;
    let max_index = sessions.len(); // 0 = New Game, 1..=sessions.len() = sessions

    loop {
        terminal.draw(|f| {
            render_start_screen(f, selected, sessions);
        })?;

        if let Event::Key(KeyEvent { code, .. }) = event::read()? {
            match code {
                KeyCode::Up | KeyCode::Char('k') => {
                    selected = selected.saturating_sub(1);
                }
                KeyCode::Down | KeyCode::Char('j') => {
                    if selected < max_index {
                        selected += 1;
                    }
                }
                KeyCode::Enter => {
                    if selected == 0 {
                        return Ok(StartChoice::NewGame);
                    } else {
                        return Ok(StartChoice::Continue(selected - 1));
                    }
                }
                KeyCode::Char('q') | KeyCode::Esc => {
                    return Err(anyhow::anyhow!("User quit from start screen"));
                }
                _ => {}
            }
        }
    }
}

/// Render the start screen UI.
fn render_start_screen(frame: &mut Frame, selected: usize, sessions: &[SessionInfo]) {
    let area = frame.area();

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
    render_menu_with_sessions(frame, chunks[1], selected, sessions);

    // Footer instructions
    render_footer(frame, chunks[2]);
}

fn render_title(frame: &mut Frame, area: ratatui::layout::Rect) {
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

fn render_menu_with_sessions(
    frame: &mut Frame,
    area: ratatui::layout::Rect,
    selected: usize,
    sessions: &[SessionInfo],
) {
    let mut menu_items = vec![];

    // First item: New Game
    menu_items.push(ListItem::new(Line::from(vec![
        Span::styled(
            if selected == 0 { "► " } else { "  " },
            Style::default().fg(Color::Yellow),
        ),
        Span::styled(
            "New Game",
            if selected == 0 {
                Style::default()
                    .fg(Color::Cyan)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(Color::Cyan)
            },
        ),
    ])));

    // Add separator
    if !sessions.is_empty() {
        menu_items.push(ListItem::new(Line::from(vec![Span::styled(
            "  ─────────────────────",
            Style::default().fg(Color::DarkGray),
        )])));
    }

    // Add sessions
    for (idx, session) in sessions.iter().enumerate() {
        let is_selected = selected == idx + 1;

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

fn render_footer(frame: &mut Frame, area: ratatui::layout::Rect) {
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
