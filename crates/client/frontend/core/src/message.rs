//! Shared message log primitives for CLI and future UIs.
use std::collections::VecDeque;

/// Severity level for UI messages produced from runtime events.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum MessageLevel {
    Info,
    Warning,
    Error,
}

/// Snapshot of a single message entry.
#[derive(Clone, Debug)]
pub struct MessageEntry {
    pub text: String,
    pub timestamp: Option<u64>,
    pub level: MessageLevel,
}

impl MessageEntry {
    pub fn new(text: impl Into<String>, timestamp: Option<u64>, level: MessageLevel) -> Self {
        Self {
            text: text.into(),
            timestamp,
            level,
        }
    }
}

/// Circular buffer of messages displayed to the player.
#[derive(Clone, Debug)]
pub struct MessageLog {
    entries: VecDeque<MessageEntry>,
    capacity: usize,
}

impl MessageLog {
    pub fn new(capacity: usize) -> Self {
        let bounded_capacity = capacity.max(1);
        Self {
            entries: VecDeque::with_capacity(bounded_capacity),
            capacity: bounded_capacity,
        }
    }

    pub fn push(&mut self, entry: MessageEntry) {
        if self.entries.len() == self.capacity {
            self.entries.pop_front();
        }
        self.entries.push_back(entry);
    }

    pub fn push_text(&mut self, message: impl Into<String>) {
        self.push(MessageEntry::new(message, None, MessageLevel::Info));
    }

    pub fn recent(&self, limit: usize) -> impl Iterator<Item = &MessageEntry> {
        self.entries.iter().rev().take(limit)
    }

    pub fn iter(&self) -> impl Iterator<Item = &MessageEntry> {
        self.entries.iter()
    }
}
