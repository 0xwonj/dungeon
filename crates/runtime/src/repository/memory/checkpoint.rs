//! In-memory checkpoint repository implementation.

use std::collections::HashMap;
use std::sync::RwLock;

use crate::api::Result;
use crate::repository::{Checkpoint, CheckpointRepository, RepositoryError};

/// In-memory checkpoint repository for testing and development.
///
/// Thread-safe but not persistent across process restarts.
pub struct InMemoryCheckpointRepository {
    checkpoints: RwLock<HashMap<String, Checkpoint>>,
}

impl InMemoryCheckpointRepository {
    /// Create a new empty in-memory checkpoint repository.
    pub fn new() -> Self {
        Self {
            checkpoints: RwLock::new(HashMap::new()),
        }
    }
}

impl Default for InMemoryCheckpointRepository {
    fn default() -> Self {
        Self::new()
    }
}

impl CheckpointRepository for InMemoryCheckpointRepository {
    fn save(&self, checkpoint: &Checkpoint) -> Result<()> {
        let mut checkpoints = self
            .checkpoints
            .write()
            .map_err(|_| RepositoryError::LockPoisoned)?;

        checkpoints.insert(checkpoint.session_id.clone(), checkpoint.clone());
        Ok(())
    }

    fn load(&self, session_id: &str) -> Result<Option<Checkpoint>> {
        let checkpoints = self
            .checkpoints
            .read()
            .map_err(|_| RepositoryError::LockPoisoned)?;

        Ok(checkpoints.get(session_id).cloned())
    }

    fn delete(&self, session_id: &str) -> Result<()> {
        let mut checkpoints = self
            .checkpoints
            .write()
            .map_err(|_| RepositoryError::LockPoisoned)?;

        checkpoints.remove(session_id);
        Ok(())
    }

    fn list_sessions(&self) -> Result<Vec<String>> {
        let checkpoints = self
            .checkpoints
            .read()
            .map_err(|_| RepositoryError::LockPoisoned)?;

        Ok(checkpoints.keys().cloned().collect())
    }
}
