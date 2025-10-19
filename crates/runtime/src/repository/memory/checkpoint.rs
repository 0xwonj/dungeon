//! In-memory checkpoint repository implementation.

use std::collections::HashMap;
use std::sync::RwLock;

use crate::api::Result;
use crate::repository::{Checkpoint, CheckpointRepository, RepositoryError};

/// In-memory checkpoint repository for testing and development.
///
/// Thread-safe but not persistent across process restarts.
///
/// # Storage Model
///
/// Stores checkpoints indexed by (session_id, nonce):
/// ```text
/// HashMap<(SessionId, Nonce), Checkpoint>
/// ```
pub struct InMemoryCheckpointRepository {
    checkpoints: RwLock<HashMap<(String, u64), Checkpoint>>,
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

        let key = (checkpoint.session_id.clone(), checkpoint.nonce);
        checkpoints.insert(key, checkpoint.clone());
        Ok(())
    }

    fn load_latest(&self, session_id: &str) -> Result<Option<Checkpoint>> {
        let checkpoints = self
            .checkpoints
            .read()
            .map_err(|_| RepositoryError::LockPoisoned)?;

        // Find the checkpoint with the highest nonce for this session
        let latest = checkpoints
            .iter()
            .filter(|((sess_id, _), _)| sess_id == session_id)
            .max_by_key(|((_, nonce), _)| nonce)
            .map(|(_, checkpoint)| checkpoint.clone());

        Ok(latest)
    }

    fn load_at_nonce(&self, session_id: &str, nonce: u64) -> Result<Option<Checkpoint>> {
        let checkpoints = self
            .checkpoints
            .read()
            .map_err(|_| RepositoryError::LockPoisoned)?;

        let key = (session_id.to_string(), nonce);
        Ok(checkpoints.get(&key).cloned())
    }

    fn list_checkpoints(&self, session_id: &str) -> Result<Vec<u64>> {
        let checkpoints = self
            .checkpoints
            .read()
            .map_err(|_| RepositoryError::LockPoisoned)?;

        let mut nonces: Vec<u64> = checkpoints
            .keys()
            .filter(|(sess_id, _)| sess_id == session_id)
            .map(|(_, nonce)| *nonce)
            .collect();

        nonces.sort();
        Ok(nonces)
    }

    fn delete(&self, session_id: &str, nonce: u64) -> Result<()> {
        let mut checkpoints = self
            .checkpoints
            .write()
            .map_err(|_| RepositoryError::LockPoisoned)?;

        let key = (session_id.to_string(), nonce);
        checkpoints.remove(&key);
        Ok(())
    }

    fn delete_all(&self, session_id: &str) -> Result<usize> {
        let mut checkpoints = self
            .checkpoints
            .write()
            .map_err(|_| RepositoryError::LockPoisoned)?;

        let keys_to_remove: Vec<_> = checkpoints
            .keys()
            .filter(|(sess_id, _)| sess_id == session_id)
            .cloned()
            .collect();

        let count = keys_to_remove.len();
        for key in keys_to_remove {
            checkpoints.remove(&key);
        }

        Ok(count)
    }

    fn delete_before(&self, session_id: &str, before_nonce: u64) -> Result<usize> {
        let mut checkpoints = self
            .checkpoints
            .write()
            .map_err(|_| RepositoryError::LockPoisoned)?;

        let keys_to_remove: Vec<_> = checkpoints
            .keys()
            .filter(|(sess_id, nonce)| sess_id == session_id && *nonce < before_nonce)
            .cloned()
            .collect();

        let count = keys_to_remove.len();
        for key in keys_to_remove {
            checkpoints.remove(&key);
        }

        Ok(count)
    }

    fn list_sessions(&self) -> Result<Vec<String>> {
        let checkpoints = self
            .checkpoints
            .read()
            .map_err(|_| RepositoryError::LockPoisoned)?;

        let sessions: std::collections::HashSet<String> = checkpoints
            .keys()
            .map(|(session_id, _)| session_id.clone())
            .collect();

        let mut sessions: Vec<String> = sessions.into_iter().collect();
        sessions.sort();
        Ok(sessions)
    }
}
