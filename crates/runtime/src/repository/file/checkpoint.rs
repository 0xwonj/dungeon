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
    fn checkpoint_path(&self, session_id: &str) -> PathBuf {
        self.base_dir
            .join(format!("checkpoint_{}.json", session_id))
    }
}

impl CheckpointRepository for FileCheckpointRepository {
    fn save(&self, checkpoint: &Checkpoint) -> Result<()> {
        let path = self.checkpoint_path(&checkpoint.session_id);
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

    fn load(&self, session_id: &str) -> Result<Option<Checkpoint>> {
        let path = self.checkpoint_path(session_id);

        if !path.exists() {
            return Ok(None);
        }

        let json = fs::read_to_string(&path).map_err(RepositoryError::Io)?;
        let checkpoint: Checkpoint =
            serde_json::from_str(&json).map_err(|e| RepositoryError::Json(e.to_string()))?;

        tracing::info!(
            "Loaded checkpoint for session '{}' with {} topic offsets",
            checkpoint.session_id,
            checkpoint.event_ref.topic_offsets.len()
        );

        Ok(Some(checkpoint))
    }

    fn delete(&self, session_id: &str) -> Result<()> {
        let path = self.checkpoint_path(session_id);

        if path.exists() {
            fs::remove_file(&path).map_err(RepositoryError::Io)?;
            tracing::info!("Deleted checkpoint: {}", path.display());
        }

        Ok(())
    }

    fn list_sessions(&self) -> Result<Vec<String>> {
        let mut sessions = Vec::new();

        let entries = fs::read_dir(&self.base_dir).map_err(RepositoryError::Io)?;

        for entry in entries {
            let entry = entry.map_err(RepositoryError::Io)?;
            let path = entry.path();

            if let Some(filename) = path.file_name().and_then(|s| s.to_str())
                && let Some(session_id) = filename
                    .strip_prefix("checkpoint_")
                    .and_then(|s| s.strip_suffix(".json"))
            {
                sessions.push(session_id.to_string());
            }
        }

        sessions.sort();
        Ok(sessions)
    }
}
