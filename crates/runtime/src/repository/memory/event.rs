//! In-memory event log implementation.

use std::sync::RwLock;

use crate::events::Event;
use crate::repository::{EventRepository, RepositoryError, Result};

/// In-memory event log for testing and development.
///
/// Thread-safe but not persistent across process restarts.
/// Events are stored in a Vec and offsets are simply the index.
pub struct InMemoryEventRepository {
    session_id: String,
    events: RwLock<Vec<Event>>,
}

impl InMemoryEventRepository {
    /// Create a new empty in-memory event log.
    pub fn new(session_id: String) -> Self {
        Self {
            session_id,
            events: RwLock::new(Vec::new()),
        }
    }

    /// Get all events (for testing/debugging).
    pub fn get_all(&self) -> Result<Vec<Event>> {
        let events = self
            .events
            .read()
            .map_err(|_| RepositoryError::LockPoisoned)?;

        Ok(events.clone())
    }
}

impl EventRepository for InMemoryEventRepository {
    fn append(&mut self, event: &Event) -> Result<u64> {
        let mut events = self
            .events
            .write()
            .map_err(|_| RepositoryError::LockPoisoned)?;

        events.push(event.clone());
        Ok(events.len() as u64)
    }

    fn read_at_offset(&self, byte_offset: u64) -> Result<Option<(Event, u64)>> {
        // For in-memory repository, we treat "offset" as an index for simplicity
        let events = self
            .events
            .read()
            .map_err(|_| RepositoryError::LockPoisoned)?;

        match events.get(byte_offset as usize).cloned() {
            Some(event) => {
                let next_offset = byte_offset + 1; // Next index
                Ok(Some((event, next_offset)))
            }
            None => Ok(None),
        }
    }

    fn flush(&mut self) -> Result<()> {
        // No-op for in-memory - already "flushed"
        Ok(())
    }

    fn size(&self) -> Result<u64> {
        let events = self
            .events
            .read()
            .map_err(|_| RepositoryError::LockPoisoned)?;

        Ok(events.len() as u64)
    }

    fn session_id(&self) -> &str {
        &self.session_id
    }
}
