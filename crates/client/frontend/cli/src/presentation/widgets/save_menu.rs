//! Save/Load menu widget (full-screen).
//!
//! Two-pane layout:
//! - Left: Saved states (loadable checkpoints) - Enter to load
//! - Right: Proof status and blockchain submission

use ratatui::{
    Frame,
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, Paragraph, Wrap},
};

use crate::state::SaveMenuState;
use client_frontend_core::MessageLog;

/// Render the save/load menu as a full-screen replacement.
///
/// This widget displays a two-pane layout:
/// - **Left pane**: List of saved states (nonces) - navigate with ↑/↓
/// - **Right pane**: Load State option + Proof status + Submit to Chain
/// - **Bottom**: Status log for blockchain operations
pub fn render_fullscreen(
    frame: &mut Frame,
    area: Rect,
    menu_state: &SaveMenuState,
    status_log: &MessageLog,
) {
    // Main layout: title, content (2-pane), status log, footer
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3), // Title
            Constraint::Min(0),    // Content (2-pane)
            Constraint::Length(6), // Status log (blockchain operations)
            Constraint::Length(3), // Footer (instructions)
        ])
        .split(area);

    // Render title
    render_title(frame, chunks[0]);

    // Split content into left/right panes
    let content_chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage(40), // Left: Saved states
            Constraint::Percentage(60), // Right: Details + proof ops
        ])
        .split(chunks[1]);

    // Render left pane (saved states)
    render_saved_states_pane(frame, content_chunks[0], menu_state);

    // Render right pane (details)
    render_details_pane(frame, content_chunks[1], menu_state);

    // Render status log
    render_status_log(frame, chunks[2], status_log);

    // Render footer
    render_footer(frame, chunks[3], menu_state);
}

/// Render the title bar.
fn render_title(frame: &mut Frame, area: Rect) {
    let title = Paragraph::new(vec![
        Line::from(""),
        Line::from(vec![Span::styled(
            "Save / Load Game",
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
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

/// Render the left pane showing saved states (loadable checkpoints).
fn render_saved_states_pane(frame: &mut Frame, area: Rect, menu_state: &SaveMenuState) {
    if menu_state.saved_states.is_empty() {
        // No saved states - show empty message
        let empty_msg = Paragraph::new(vec![
            Line::from(""),
            Line::from(vec![Span::styled(
                "No saved states.",
                Style::default().fg(Color::Gray),
            )]),
            Line::from(""),
            Line::from(vec![Span::styled(
                "Press Ctrl+S to save",
                Style::default()
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::ITALIC),
            )]),
        ])
        .alignment(Alignment::Center)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::Cyan))
                .title(" Saved States "),
        );

        frame.render_widget(empty_msg, area);
        return;
    }

    // Build list items for saved states
    let items: Vec<ListItem> = menu_state
        .saved_states
        .iter()
        .enumerate()
        .map(|(idx, state_info)| {
            let is_selected = idx == menu_state.selected_index;
            let prefix = if is_selected { "► " } else { "  " };

            let line = Line::from(vec![
                Span::styled(prefix, Style::default().fg(Color::Yellow)),
                Span::styled(
                    format!("Nonce {}", state_info.nonce),
                    if is_selected {
                        Style::default()
                            .fg(Color::White)
                            .add_modifier(Modifier::BOLD)
                    } else {
                        Style::default().fg(Color::White)
                    },
                ),
            ]);

            ListItem::new(line)
        })
        .collect();

    let list = List::new(items).block(
        Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::Cyan))
            .title(" Saved States (↑/↓ to navigate) ")
            .title_alignment(Alignment::Left),
    );

    frame.render_widget(list, area);
}

/// Render the right pane showing details and proof operations.
fn render_details_pane(frame: &mut Frame, area: Rect, menu_state: &SaveMenuState) {
    if menu_state.saved_states.is_empty() {
        frame.render_widget(
            Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::White))
                .title(" Details "),
            area,
        );
        return;
    }

    // Get selected state
    let selected_state = &menu_state.saved_states[menu_state.selected_index];

    let mut lines = vec![
        Line::from(vec![
            Span::styled("Selected State: ", Style::default().fg(Color::Gray)),
            Span::styled(
                format!("Nonce {}", selected_state.nonce),
                Style::default()
                    .fg(Color::Cyan)
                    .add_modifier(Modifier::BOLD),
            ),
        ]),
        Line::from(""),
        Line::from(""),
        Line::from(vec![Span::styled(
            "Load State:",
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::UNDERLINED),
        )]),
        Line::from(""),
        Line::from(vec![
            Span::styled(
                "  [ENTER] ",
                Style::default()
                    .fg(Color::Green)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled("Load This State", Style::default().fg(Color::White)),
        ]),
        Line::from(""),
        Line::from(""),
    ];

    // Show session creation option (only if sui feature is enabled)
    #[cfg(feature = "sui")]
    {
        // Only show session management for Genesis state (nonce 0)
        if selected_state.nonce == 0 {
            if let Some(session) = &menu_state.session_info {
                // Session already exists - show info
                lines.push(Line::from(vec![Span::styled(
                    "Blockchain Session:",
                    Style::default()
                        .fg(Color::Yellow)
                        .add_modifier(Modifier::UNDERLINED),
                )]));
                lines.push(Line::from(""));
                lines.push(Line::from(vec![
                    Span::styled("  ID: ", Style::default().fg(Color::Gray)),
                    Span::styled(
                        format!("{}...", &session.id[..12]),
                        Style::default().fg(Color::Cyan),
                    ),
                ]));
                lines.push(Line::from(vec![
                    Span::styled("  State Root: ", Style::default().fg(Color::Gray)),
                    Span::styled(
                        hex::encode(&session.state_root[..4]),
                        Style::default().fg(Color::Magenta),
                    ),
                    Span::styled("...", Style::default().fg(Color::Gray)),
                ]));
                lines.push(Line::from(vec![
                    Span::styled("  Nonce: ", Style::default().fg(Color::Gray)),
                    Span::styled(
                        format!("{}", session.nonce),
                        Style::default().fg(Color::White),
                    ),
                ]));
                lines.push(Line::from(""));
                lines.push(Line::from(""));
            } else {
                // No session - show create option
                lines.push(Line::from(vec![Span::styled(
                    "Session Management:",
                    Style::default()
                        .fg(Color::Yellow)
                        .add_modifier(Modifier::UNDERLINED),
                )]));
                lines.push(Line::from(""));
                lines.push(Line::from(vec![
                    Span::styled(
                        "  [C] ",
                        Style::default()
                            .fg(Color::Green)
                            .add_modifier(Modifier::BOLD),
                    ),
                    Span::styled(
                        "Create Session on Blockchain",
                        Style::default().fg(Color::White),
                    ),
                ]));
                lines.push(Line::from(vec![
                    Span::styled("      ", Style::default()),
                    Span::styled(
                        "(Uses state 0 as initial state)",
                        Style::default()
                            .fg(Color::DarkGray)
                            .add_modifier(Modifier::ITALIC),
                    ),
                ]));
                lines.push(Line::from(""));
                lines.push(Line::from(""));
            }
        }
    }

    // Show associated action batch info if available
    if let Some(batch_idx) = selected_state.batch_index {
        if let Some(batch) = menu_state.action_batches.get(batch_idx) {
            lines.push(Line::from(vec![Span::styled(
                "Associated Proof Batch:",
                Style::default()
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::UNDERLINED),
            )]));
            lines.push(Line::from(""));

            lines.push(Line::from(vec![
                Span::styled("  Actions: ", Style::default().fg(Color::Gray)),
                Span::styled(
                    format!(
                        "Nonce {} → {} ({} actions)",
                        batch.start_nonce,
                        batch.end_nonce,
                        batch.action_count()
                    ),
                    Style::default().fg(Color::White),
                ),
            ]));

            // Format status
            let (status_str, status_color) = match &batch.status {
                runtime::ActionBatchStatus::Complete => ("Ready for proving", Color::Green),
                runtime::ActionBatchStatus::Proving => ("Proving in progress...", Color::Yellow),
                runtime::ActionBatchStatus::Proven { .. } => {
                    ("Proven (ready to submit)", Color::Cyan)
                }
                runtime::ActionBatchStatus::UploadingToWalrus => {
                    ("Uploading to Walrus...", Color::Yellow)
                }
                runtime::ActionBatchStatus::BlobUploaded { .. } => {
                    ("Blob uploaded to Walrus", Color::LightBlue)
                }
                runtime::ActionBatchStatus::SubmittingOnchain { .. } => {
                    ("Submitting to chain...", Color::Yellow)
                }
                runtime::ActionBatchStatus::OnChain { .. } => {
                    ("Submitted on-chain", Color::Magenta)
                }
                runtime::ActionBatchStatus::Failed { .. } => ("Failed", Color::Red),
                runtime::ActionBatchStatus::InProgress => ("In progress", Color::Gray),
            };

            lines.push(Line::from(vec![
                Span::styled("  Status: ", Style::default().fg(Color::Gray)),
                Span::styled(status_str, Style::default().fg(status_color)),
            ]));

            lines.push(Line::from(""));

            // Show proof hash if available
            if let runtime::ActionBatchStatus::Proven { proof_file, .. } = &batch.status {
                lines.push(Line::from(vec![
                    Span::styled("  Proof: ", Style::default().fg(Color::Gray)),
                    Span::styled(
                        proof_file,
                        Style::default()
                            .fg(Color::Cyan)
                            .add_modifier(Modifier::ITALIC),
                    ),
                ]));
                lines.push(Line::from(""));
            }

            // Show blob info if uploaded
            let blob_info = match &batch.status {
                runtime::ActionBatchStatus::BlobUploaded {
                    blob_object_id,
                    walrus_blob_id,
                }
                | runtime::ActionBatchStatus::SubmittingOnchain {
                    blob_object_id,
                    walrus_blob_id,
                }
                | runtime::ActionBatchStatus::OnChain {
                    blob_object_id,
                    walrus_blob_id,
                    ..
                } => Some((blob_object_id.as_str(), walrus_blob_id.as_str())),
                _ => None,
            };

            if let Some((blob_obj_id, walrus_id)) = blob_info {
                lines.push(Line::from(vec![
                    Span::styled("  Walrus Blob: ", Style::default().fg(Color::Gray)),
                    Span::styled(
                        &walrus_id[..walrus_id.len().min(12)],
                        Style::default()
                            .fg(Color::LightBlue)
                            .add_modifier(Modifier::ITALIC),
                    ),
                    Span::styled("...", Style::default().fg(Color::Gray)),
                ]));
                lines.push(Line::from(vec![
                    Span::styled("  Blob Object: ", Style::default().fg(Color::Gray)),
                    Span::styled(
                        &blob_obj_id[..blob_obj_id.len().min(12)],
                        Style::default()
                            .fg(Color::Cyan)
                            .add_modifier(Modifier::ITALIC),
                    ),
                    Span::styled("...", Style::default().fg(Color::Gray)),
                ]));
                lines.push(Line::from(""));
            }

            // Show transaction digest if submitted
            if let runtime::ActionBatchStatus::OnChain { tx_digest, .. } = &batch.status {
                lines.push(Line::from(vec![
                    Span::styled("  Tx Digest: ", Style::default().fg(Color::Gray)),
                    Span::styled(
                        &tx_digest[..tx_digest.len().min(12)],
                        Style::default()
                            .fg(Color::Magenta)
                            .add_modifier(Modifier::ITALIC),
                    ),
                    Span::styled("...", Style::default().fg(Color::Gray)),
                ]));
                lines.push(Line::from(""));
            }

            // Always show all actions, but gray out unavailable ones
            lines.push(Line::from(""));
            lines.push(Line::from(vec![Span::styled(
                "Actions:",
                Style::default()
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::UNDERLINED),
            )]));
            lines.push(Line::from(""));

            // [W] Upload to Walrus - only available after Proven AND if sui feature enabled
            #[cfg(feature = "sui")]
            let can_upload = matches!(batch.status, runtime::ActionBatchStatus::Proven { .. });
            #[cfg(not(feature = "sui"))]
            let can_upload = false;

            lines.push(Line::from(vec![
                Span::styled(
                    "  [W] ",
                    if can_upload {
                        Style::default()
                            .fg(Color::Cyan)
                            .add_modifier(Modifier::BOLD)
                    } else {
                        Style::default().fg(Color::DarkGray)
                    },
                ),
                Span::styled(
                    "Upload to Walrus",
                    if can_upload {
                        Style::default().fg(Color::White)
                    } else {
                        Style::default().fg(Color::DarkGray)
                    },
                ),
            ]));

            // [S] Submit to Blockchain - only available after BlobUploaded AND if sui feature enabled
            #[cfg(feature = "sui")]
            let can_submit = matches!(
                batch.status,
                runtime::ActionBatchStatus::BlobUploaded { .. }
            );
            #[cfg(not(feature = "sui"))]
            let can_submit = false;

            lines.push(Line::from(vec![
                Span::styled(
                    "  [S] ",
                    if can_submit {
                        Style::default()
                            .fg(Color::Magenta)
                            .add_modifier(Modifier::BOLD)
                    } else {
                        Style::default().fg(Color::DarkGray)
                    },
                ),
                Span::styled(
                    "Submit to Blockchain",
                    if can_submit {
                        Style::default().fg(Color::White)
                    } else {
                        Style::default().fg(Color::DarkGray)
                    },
                ),
            ]));

            // Old code removed
            if false {
                lines.push(Line::from(""));
                lines.push(Line::from(""));
                lines.push(Line::from(vec![Span::styled(
                    "Blockchain Submission:",
                    Style::default()
                        .fg(Color::Yellow)
                        .add_modifier(Modifier::UNDERLINED),
                )]));
                lines.push(Line::from(""));
                lines.push(Line::from(vec![
                    Span::styled(
                        "  [S] ",
                        Style::default()
                            .fg(Color::Cyan)
                            .add_modifier(Modifier::BOLD),
                    ),
                    Span::styled("Submit Proof to Chain", Style::default().fg(Color::White)),
                ]));
            }
        }
    } else {
        // Genesis state (nonce 0) - no associated batch
        if selected_state.nonce == 0 {
            lines.push(Line::from(vec![Span::styled(
                "Genesis State",
                Style::default()
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::UNDERLINED),
            )]));
            lines.push(Line::from(""));
            lines.push(Line::from(vec![Span::styled(
                "  Initial game state before any actions",
                Style::default()
                    .fg(Color::Gray)
                    .add_modifier(Modifier::ITALIC),
            )]));
        } else {
            lines.push(Line::from(vec![Span::styled(
                "No associated proof batch",
                Style::default()
                    .fg(Color::Gray)
                    .add_modifier(Modifier::ITALIC),
            )]));
        }
    }

    let details = Paragraph::new(lines)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::White))
                .title(" Details & Operations "),
        )
        .wrap(Wrap { trim: false });

    frame.render_widget(details, area);
}

/// Render status log showing blockchain operation messages.
fn render_status_log(frame: &mut Frame, area: Rect, status_log: &MessageLog) {
    use client_frontend_core::MessageLevel;

    // Get recent messages (last 4 lines to fit in the box)
    let lines: Vec<Line> = status_log
        .recent(4)
        .map(|entry| {
            let style = match entry.level {
                MessageLevel::Info => Style::default().fg(Color::White),
                MessageLevel::Warning => Style::default().fg(Color::Yellow),
                MessageLevel::Error => Style::default().fg(Color::Red),
            };
            Line::from(Span::styled(&entry.text, style))
        })
        .collect();

    let log = Paragraph::new(lines)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::Yellow))
                .title(" Status Log ")
                .title_alignment(Alignment::Left),
        )
        .wrap(Wrap { trim: false });

    frame.render_widget(log, area);
}

/// Render footer with navigation instructions.
fn render_footer(frame: &mut Frame, area: Rect, menu_state: &SaveMenuState) {
    // Gray out blockchain operations if sui feature is disabled
    #[cfg(feature = "sui")]
    let (c_color, w_color, s_color, blockchain_enabled) =
        (Color::Green, Color::Cyan, Color::Magenta, true);
    #[cfg(not(feature = "sui"))]
    let (c_color, w_color, s_color, blockchain_enabled) =
        (Color::DarkGray, Color::DarkGray, Color::DarkGray, false);

    let instructions = vec![
        Line::from(""),
        Line::from(vec![
            Span::styled("↑/↓", Style::default().fg(Color::Yellow)),
            Span::raw(" Navigate  "),
            Span::styled("ENTER", Style::default().fg(Color::Green)),
            Span::raw(" Load  "),
            Span::styled("C", Style::default().fg(c_color)),
            Span::styled(" Create Session  ", {
                #[cfg(feature = "sui")]
                {
                    if blockchain_enabled
                        && menu_state.selected_index == 0
                        && menu_state.session_info.is_none()
                    {
                        Style::default()
                    } else {
                        Style::default().fg(Color::DarkGray)
                    }
                }
                #[cfg(not(feature = "sui"))]
                {
                    Style::default().fg(Color::DarkGray)
                }
            }),
            Span::styled("W", Style::default().fg(w_color)),
            Span::styled(
                " Upload  ",
                if blockchain_enabled {
                    Style::default()
                } else {
                    Style::default().fg(Color::DarkGray)
                },
            ),
            Span::styled("S", Style::default().fg(s_color)),
            Span::styled(
                " Submit  ",
                if blockchain_enabled {
                    Style::default()
                } else {
                    Style::default().fg(Color::DarkGray)
                },
            ),
            Span::styled("ESC", Style::default().fg(Color::Red)),
            Span::raw(" Back"),
        ]),
    ];

    let footer = Paragraph::new(instructions)
        .alignment(Alignment::Center)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::White)),
        );

    frame.render_widget(footer, area);
}
