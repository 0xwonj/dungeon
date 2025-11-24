//! In-memory action log reader for testing.
//!
//! This is a simple implementation of `ActionLogReader` that stores
//! entries in memory, useful for unit testing ProverWorker without
//! file I/O dependencies.

use std::sync::{Arc, Mutex};

use crate::repository::Result;
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
            });
        }

        *position = index;
        Ok(())
    }
}
