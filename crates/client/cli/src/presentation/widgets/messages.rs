//! Messages widget displaying recent game events.

use client_core::{message::MessageEntry, view_model::PresentationMapper};
use ratatui::{
    Frame,
    layout::Rect,
    style::Style,
    widgets::{Block, Borders, List, ListDirection, ListItem},
};

/// Height of message panel in lines.
pub const MESSAGE_PANEL_HEIGHT: u16 = 5;

/// Render the message log panel.
///
/// Displays recent messages in bottom-to-top order (newest at bottom).
pub fn render<T: PresentationMapper<Style = Style>>(
    frame: &mut Frame,
    area: Rect,
    messages: &[MessageEntry],
    theme: &T,
) {
    let mut items: Vec<ListItem> = messages
        .iter()
        .map(|entry| ListItem::new(format_message(entry)).style(theme.style_message(entry.level)))
        .collect();

    // Pad with empty lines to maintain consistent height
    while items.len() < MESSAGE_PANEL_HEIGHT as usize {
        items.push(ListItem::new(""));
    }

    let list = List::new(items)
        .block(Block::default().borders(Borders::ALL).title("Messages"))
        .direction(ListDirection::BottomToTop);

    frame.render_widget(list, area);
}

/// Format a message entry with optional timestamp.
fn format_message(entry: &MessageEntry) -> String {
    match entry.timestamp {
        Some(ts) => format!("[{}] {}", ts, entry.text),
        None => entry.text.clone(),
    }
}
