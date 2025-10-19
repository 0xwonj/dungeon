//! In-memory action log reader for testing.
//!
//! This is a simple implementation of `ActionLogReader` that stores
//! entries in memory, useful for unit testing ProverWorker without
//! file I/O dependencies.

use std::sync::{Arc, Mutex};

use crate::api::Result;
use crate::repository::traits::ActionLogReader;
use crate::repository::types::ActionLogEntry;

/// In-memory action log reader for testing.
///
/// Stores action log entries in a Vec and provides sequential access.
/// Thread-safe with interior mutability using Mutex.
pub struct InMemoryActionLogReader {
    /// Entries stored in memory
    entries: Arc<Mutex<Vec<ActionLogEntry>>>,

    /// Current read position (index into entries vec)
    position: Arc<Mutex<usize>>,

    /// Session identifier
    session_id: String,
}

impl InMemoryActionLogReader {
    /// Create a new in-memory reader.
    pub fn new(session_id: String) -> Self {
        Self {
            entries: Arc::new(Mutex::new(Vec::new())),
            position: Arc::new(Mutex::new(0)),
            session_id,
        }
    }

    /// Create a new reader with pre-loaded entries.
    pub fn with_entries(session_id: String, entries: Vec<ActionLogEntry>) -> Self {
        Self {
            entries: Arc::new(Mutex::new(entries)),
            position: Arc::new(Mutex::new(0)),
            session_id,
        }
    }

    /// Append a new entry (simulates writer).
    ///
    /// This is useful for testing scenarios where entries are added
    /// while the reader is consuming them.
    pub fn append(&self, entry: ActionLogEntry) {
        let mut entries = self.entries.lock().unwrap();
        entries.push(entry);
    }

    /// Get the total number of entries.
    pub fn len(&self) -> usize {
        self.entries.lock().unwrap().len()
    }

    /// Check if empty.
    pub fn is_empty(&self) -> bool {
        self.entries.lock().unwrap().is_empty()
    }
}

impl ActionLogReader for InMemoryActionLogReader {
    fn read_next(&self) -> Result<Option<ActionLogEntry>> {
        let entries = self.entries.lock().unwrap();
        let mut position = self.position.lock().unwrap();

        if *position >= entries.len() {
            return Ok(None);
        }

        let entry = entries[*position].clone();
        *position += 1;

        Ok(Some(entry))
    }

    fn refresh(&self) -> Result<bool> {
        // In-memory implementation doesn't need refresh
        // Just check if there are more entries available
        let entries = self.entries.lock().unwrap();
        let position = self.position.lock().unwrap();
        Ok(*position < entries.len())
    }

    fn current_offset(&self) -> u64 {
        // Return position as offset (in-memory doesn't use byte offsets)
        *self.position.lock().unwrap() as u64
    }

    fn session_id(&self) -> &str {
        &self.session_id
    }

    fn has_more(&self) -> bool {
        let entries = self.entries.lock().unwrap();
        let position = self.position.lock().unwrap();
        *position < entries.len()
    }

    fn seek(&self, offset: u64) -> Result<()> {
        // In-memory uses entry index as "offset"
        let entries = self.entries.lock().unwrap();
        let mut position = self.position.lock().unwrap();

        let index = offset as usize;
        if index > entries.len() {
            return Err(crate::repository::RepositoryError::InvalidOffset {
                offset,
                file_size: entries.len() as u64,
            }
            .into());
        }

        *position = index;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use game_core::{Action, CharacterActionKind, EntityId, GameState};

    fn create_test_entry(nonce: u64) -> ActionLogEntry {
        ActionLogEntry {
            nonce,
            clock: nonce,
            action: Action::character(EntityId::PLAYER, CharacterActionKind::Wait),
            before_state: Box::new(GameState::default()),
            after_state: Box::new(GameState::default()),
            delta: None,
        }
    }

    #[test]
    fn test_in_memory_reader_basic() {
        let entries = vec![
            create_test_entry(0),
            create_test_entry(1),
            create_test_entry(2),
        ];

        let reader = InMemoryActionLogReader::with_entries("test".to_string(), entries);

        // Read all entries
        assert_eq!(reader.read_next().unwrap().unwrap().nonce, 0);
        assert_eq!(reader.read_next().unwrap().unwrap().nonce, 1);
        assert_eq!(reader.read_next().unwrap().unwrap().nonce, 2);

        // Reached end
        assert!(reader.read_next().unwrap().is_none());
    }

    #[test]
    fn test_in_memory_reader_append() {
        let reader = InMemoryActionLogReader::new("test".to_string());

        // Initially empty
        assert!(reader.read_next().unwrap().is_none());

        // Append entry
        reader.append(create_test_entry(0));

        // Now can read
        assert_eq!(reader.read_next().unwrap().unwrap().nonce, 0);
    }

    #[test]
    fn test_in_memory_reader_has_more() {
        let entries = vec![create_test_entry(0)];
        let reader = InMemoryActionLogReader::with_entries("test".to_string(), entries);

        assert!(reader.has_more());
        reader.read_next().unwrap();
        assert!(!reader.has_more());
    }
}
