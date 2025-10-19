//! File-based CheckpointRepository implementation.

use std::fs;
use std::path::{Path, PathBuf};

use crate::api::Result;
use crate::repository::{Checkpoint, CheckpointRepository, RepositoryError};

/// File-based implementation of CheckpointRepository.
///
/// Manages checkpoint persistence with 2-phase commit pattern.
///
/// Ensures consistency between checkpoint metadata and external data:
/// 1. **Phase 1**: Save external data (state, proof) first
/// 2. **Phase 2**: Save checkpoint last (commit point)
///
/// # Storage Layout
///
/// Each checkpoint is stored as an individual file:
/// ```text
/// {base_dir}/checkpoint_{session}_nonce_{nonce:010}.json
/// ```
///
/// Example:
/// ```text
/// checkpoints/
///   ├── checkpoint_session123_nonce_0000000000.json
///   ├── checkpoint_session123_nonce_0000001000.json
///   └── checkpoint_session456_nonce_0000000000.json
/// ```
pub struct FileCheckpointRepository {
    base_dir: PathBuf,
}

impl FileCheckpointRepository {
    /// Create a new file-based checkpoint repository.
    pub fn new(base_dir: impl AsRef<Path>) -> Result<Self> {
        let base_dir = base_dir.as_ref().to_path_buf();
        fs::create_dir_all(&base_dir).map_err(RepositoryError::Io)?;
        Ok(Self { base_dir })
    }

    /// Get the path to a checkpoint file.
    fn checkpoint_path(&self, session_id: &str, nonce: u64) -> PathBuf {
        self.base_dir.join(format!(
            "checkpoint_{}_nonce_{:010}.json",
            session_id, nonce
        ))
    }

    /// Parse session_id and nonce from checkpoint filename.
    ///
    /// Expected format: `checkpoint_{session}_nonce_{nonce}.json`
    fn parse_filename(&self, filename: &str) -> Option<(String, u64)> {
        let without_ext = filename.strip_suffix(".json")?;
        let parts: Vec<&str> = without_ext.split("_nonce_").collect();
        if parts.len() != 2 {
            return None;
        }

        let session_id = parts[0].strip_prefix("checkpoint_")?;
        let nonce = parts[1].parse::<u64>().ok()?;

        Some((session_id.to_string(), nonce))
    }
}

impl CheckpointRepository for FileCheckpointRepository {
    fn save(&self, checkpoint: &Checkpoint) -> Result<()> {
        let path = self.checkpoint_path(&checkpoint.session_id, checkpoint.nonce);
        let temp_path = path.with_extension("json.tmp");

        // Write to temp file
        let json = serde_json::to_string_pretty(checkpoint)
            .map_err(|e| RepositoryError::Json(e.to_string()))?;
        fs::write(&temp_path, json).map_err(RepositoryError::Io)?;

        // Atomic rename
        fs::rename(&temp_path, &path).map_err(RepositoryError::Io)?;

        tracing::debug!("Saved checkpoint: {}", path.display());

        Ok(())
    }

    fn load_latest(&self, session_id: &str) -> Result<Option<Checkpoint>> {
        let checkpoints = self.list_checkpoints(session_id)?;

        if checkpoints.is_empty() {
            return Ok(None);
        }

        // Get the latest nonce (list is sorted ascending)
        let latest_nonce = *checkpoints.last().unwrap();

        self.load_at_nonce(session_id, latest_nonce)
    }

    fn load_at_nonce(&self, session_id: &str, nonce: u64) -> Result<Option<Checkpoint>> {
        let path = self.checkpoint_path(session_id, nonce);

        if !path.exists() {
            return Ok(None);
        }

        let json = fs::read_to_string(&path).map_err(RepositoryError::Io)?;
        let checkpoint: Checkpoint =
            serde_json::from_str(&json).map_err(|e| RepositoryError::Json(e.to_string()))?;

        tracing::info!(
            "Loaded checkpoint for session '{}' at nonce={} with action_offset={}",
            checkpoint.session_id,
            checkpoint.nonce,
            checkpoint.action_log_offset
        );

        Ok(Some(checkpoint))
    }

    fn list_checkpoints(&self, session_id: &str) -> Result<Vec<u64>> {
        let mut nonces = Vec::new();

        let entries = fs::read_dir(&self.base_dir).map_err(RepositoryError::Io)?;

        for entry in entries {
            let entry = entry.map_err(RepositoryError::Io)?;
            let path = entry.path();

            if let Some(filename) = path.file_name().and_then(|s| s.to_str())
                && let Some((sess_id, nonce)) = self.parse_filename(filename)
                && sess_id == session_id
            {
                nonces.push(nonce);
            }
        }

        nonces.sort();
        Ok(nonces)
    }

    fn delete(&self, session_id: &str, nonce: u64) -> Result<()> {
        let path = self.checkpoint_path(session_id, nonce);

        if path.exists() {
            fs::remove_file(&path).map_err(RepositoryError::Io)?;
            tracing::debug!("Deleted checkpoint: {}", path.display());
        }

        Ok(())
    }

    fn delete_all(&self, session_id: &str) -> Result<usize> {
        let nonces = self.list_checkpoints(session_id)?;
        let count = nonces.len();

        for nonce in nonces {
            self.delete(session_id, nonce)?;
        }

        tracing::info!("Deleted {} checkpoints for session '{}'", count, session_id);

        Ok(count)
    }

    fn delete_before(&self, session_id: &str, before_nonce: u64) -> Result<usize> {
        let nonces = self.list_checkpoints(session_id)?;
        let mut deleted = 0;

        for nonce in nonces {
            if nonce < before_nonce {
                self.delete(session_id, nonce)?;
                deleted += 1;
            }
        }

        if deleted > 0 {
            tracing::info!(
                "Deleted {} checkpoints before nonce {} for session '{}'",
                deleted,
                before_nonce,
                session_id
            );
        }

        Ok(deleted)
    }

    fn list_sessions(&self) -> Result<Vec<String>> {
        let mut sessions = std::collections::HashSet::new();

        let entries = fs::read_dir(&self.base_dir).map_err(RepositoryError::Io)?;

        for entry in entries {
            let entry = entry.map_err(RepositoryError::Io)?;
            let path = entry.path();

            if let Some(filename) = path.file_name().and_then(|s| s.to_str())
                && let Some((session_id, _)) = self.parse_filename(filename)
            {
                sessions.insert(session_id);
            }
        }

        let mut sessions: Vec<String> = sessions.into_iter().collect();
        sessions.sort();
        Ok(sessions)
    }
}
