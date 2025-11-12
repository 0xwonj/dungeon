//! File-based action batch repository implementation.
//!
//! Action batches are stored as individual JSON files:
//! ```text
//! {base_dir}/{session_id}/batches/batch_{end_nonce:010}.json
//! ```
//!
//! Each batch is uniquely identified by its end_nonce within a session.

use std::fs;
use std::path::{Path, PathBuf};

use serde_json;

use crate::repository::{
    ActionBatch, ActionBatchRepository, ActionBatchStatus, RepositoryError, Result,
};

/// File-based implementation of ActionBatchRepository.
///
/// Stores each batch as a JSON file in the session's batches directory.
pub struct FileActionBatchRepository {
    base_dir: PathBuf,
}

impl FileActionBatchRepository {
    /// Create a new file-based action batch repository.
    ///
    /// The directory structure will be:
    /// ```text
    /// {base_dir}/
    ///   {session_id}/
    ///     batches/
    ///       batch_0000000009.json
    ///       batch_0000000019.json
    ///       ...
    /// ```
    pub fn new(base_dir: impl AsRef<Path>) -> Result<Self> {
        let base_dir = base_dir.as_ref().to_path_buf();

        if !base_dir.exists() {
            fs::create_dir_all(&base_dir)?;
        }

        Ok(Self { base_dir })
    }

    /// Get the batches directory for a session.
    fn batches_dir(&self, session_id: &str) -> PathBuf {
        self.base_dir.join(session_id).join("batches")
    }

    /// Get the file path for a batch.
    fn batch_path(&self, session_id: &str, end_nonce: u64) -> PathBuf {
        self.batches_dir(session_id)
            .join(format!("batch_{:010}.json", end_nonce))
    }

    /// Ensure the batches directory exists.
    fn ensure_dir(&self, session_id: &str) -> Result<()> {
        let dir = self.batches_dir(session_id);
        if !dir.exists() {
            fs::create_dir_all(&dir)?;
        }
        Ok(())
    }
}

impl ActionBatchRepository for FileActionBatchRepository {
    fn save(&self, batch: &ActionBatch) -> Result<()> {
        self.ensure_dir(&batch.session_id)?;

        let path = self.batch_path(&batch.session_id, batch.end_nonce);
        let json = serde_json::to_string_pretty(batch)
            .map_err(|e| RepositoryError::Json(format!("Failed to serialize batch: {}", e)))?;

        fs::write(&path, json)?;

        Ok(())
    }

    fn load(&self, session_id: &str, end_nonce: u64) -> Result<Option<ActionBatch>> {
        let path = self.batch_path(session_id, end_nonce);

        if !path.exists() {
            return Ok(None);
        }

        let json = fs::read_to_string(&path)?;

        let batch: ActionBatch = serde_json::from_str(&json)
            .map_err(|e| RepositoryError::Json(format!("Failed to deserialize batch: {}", e)))?;

        Ok(Some(batch))
    }

    fn list(&self, session_id: &str) -> Result<Vec<ActionBatch>> {
        let dir = self.batches_dir(session_id);

        if !dir.exists() {
            return Ok(Vec::new());
        }

        let entries = fs::read_dir(&dir)?;

        let mut batches = Vec::new();

        for entry in entries {
            let entry = entry?;

            let path = entry.path();
            if path.extension().and_then(|s| s.to_str()) == Some("json") {
                let json = fs::read_to_string(&path)?;

                let batch: ActionBatch = serde_json::from_str(&json).map_err(|e| {
                    RepositoryError::Json(format!("Failed to deserialize batch: {}", e))
                })?;

                batches.push(batch);
            }
        }

        // Sort by end_nonce for consistent ordering
        batches.sort_by_key(|b| b.end_nonce);

        Ok(batches)
    }

    fn list_by_status(
        &self,
        session_id: &str,
        status: ActionBatchStatus,
    ) -> Result<Vec<ActionBatch>> {
        let all_batches = self.list(session_id)?;

        // Filter by status (comparing discriminants)
        let filtered = all_batches
            .into_iter()
            .filter(|batch| {
                std::mem::discriminant(&batch.status) == std::mem::discriminant(&status)
            })
            .collect();

        Ok(filtered)
    }

    fn delete(&self, session_id: &str, end_nonce: u64) -> Result<()> {
        let path = self.batch_path(session_id, end_nonce);

        if path.exists() {
            fs::remove_file(&path)?;
        }

        Ok(())
    }

    fn get_current_batch(&self, session_id: &str) -> Result<Option<ActionBatch>> {
        let batches = self.list_by_status(session_id, ActionBatchStatus::InProgress)?;
        Ok(batches.into_iter().next())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn setup() -> (TempDir, FileActionBatchRepository) {
        let temp_dir = TempDir::new().unwrap();
        let repo = FileActionBatchRepository::new(temp_dir.path()).unwrap();
        (temp_dir, repo)
    }

    #[test]
    fn test_save_and_load() {
        let (_temp, repo) = setup();

        let batch = ActionBatch::new("session123".to_string(), 0);
        repo.save(&batch).unwrap();

        let loaded = repo.load("session123", 0).unwrap();
        assert!(loaded.is_some());
        let loaded = loaded.unwrap();
        assert_eq!(loaded.session_id, "session123");
        assert_eq!(loaded.start_nonce, 0);
    }

    #[test]
    fn test_list() {
        let (_temp, repo) = setup();

        let batch1 = ActionBatch::new("session123".to_string(), 0);
        let mut batch2 = ActionBatch::new("session123".to_string(), 10);
        batch2.end_nonce = 19;

        repo.save(&batch1).unwrap();
        repo.save(&batch2).unwrap();

        let batches = repo.list("session123").unwrap();
        assert_eq!(batches.len(), 2);
        assert_eq!(batches[0].start_nonce, 0);
        assert_eq!(batches[1].start_nonce, 10);
    }

    #[test]
    fn test_list_by_status() {
        let (_temp, repo) = setup();

        let mut batch1 = ActionBatch::new("session123".to_string(), 0);
        batch1.mark_complete(9);

        let batch2 = ActionBatch::new("session123".to_string(), 10);

        repo.save(&batch1).unwrap();
        repo.save(&batch2).unwrap();

        let complete = repo
            .list_by_status("session123", ActionBatchStatus::Complete)
            .unwrap();
        assert_eq!(complete.len(), 1);
        assert_eq!(complete[0].end_nonce, 9);

        let in_progress = repo
            .list_by_status("session123", ActionBatchStatus::InProgress)
            .unwrap();
        assert_eq!(in_progress.len(), 1);
        assert_eq!(in_progress[0].start_nonce, 10);
    }

    #[test]
    fn test_delete() {
        let (_temp, repo) = setup();

        let batch = ActionBatch::new("session123".to_string(), 0);
        repo.save(&batch).unwrap();

        repo.delete("session123", 0).unwrap();

        let loaded = repo.load("session123", 0).unwrap();
        assert!(loaded.is_none());
    }

    #[test]
    fn test_get_current_batch() {
        let (_temp, repo) = setup();

        let batch = ActionBatch::new("session123".to_string(), 0);
        repo.save(&batch).unwrap();

        let current = repo.get_current_batch("session123").unwrap();
        assert!(current.is_some());
        assert_eq!(current.unwrap().start_nonce, 0);
    }
}
