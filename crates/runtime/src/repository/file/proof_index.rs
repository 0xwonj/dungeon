//! File-based ProofIndexRepository implementation.

use std::fs;
use std::path::{Path, PathBuf};

use crate::api::Result;
use crate::repository::{ProofIndex, ProofIndexRepository, RepositoryError};

/// File-based implementation of ProofIndexRepository.
///
/// Stores proof indices as individual JSON files indexed by session ID.
///
/// # File Format
///
/// Proof indices are stored as `proof_index_{session}.json` in JSON format:
/// - Human readability for debugging
/// - Easy inspection and modification
/// - Cross-platform compatibility
///
/// # Directory Structure
///
/// ```text
/// base_dir/
/// ├── proof_index_alice.json
/// ├── proof_index_bob.json
/// └── proof_index_quicksave.json
/// ```
pub struct FileProofIndexRepository {
    base_dir: PathBuf,
}

impl FileProofIndexRepository {
    /// Create a new file-based proof index repository.
    pub fn new(base_dir: impl AsRef<Path>) -> Result<Self> {
        let base_dir = base_dir.as_ref().to_path_buf();
        fs::create_dir_all(&base_dir).map_err(RepositoryError::Io)?;
        Ok(Self { base_dir })
    }

    /// Get the path to a proof index file.
    fn proof_index_path(&self, session_id: &str) -> PathBuf {
        self.base_dir
            .join(format!("proof_index_{}.json", session_id))
    }
}

impl ProofIndexRepository for FileProofIndexRepository {
    fn save(&self, index: &ProofIndex) -> Result<()> {
        let path = self.proof_index_path(&index.session_id);
        let temp_path = path.with_extension("json.tmp");

        // Write to temp file
        let json = serde_json::to_string_pretty(index)
            .map_err(|e| RepositoryError::Json(e.to_string()))?;
        fs::write(&temp_path, json).map_err(RepositoryError::Io)?;

        // Atomic rename
        fs::rename(&temp_path, &path).map_err(RepositoryError::Io)?;

        tracing::debug!("Saved proof index: {}", path.display());

        Ok(())
    }

    fn load(&self, session_id: &str) -> Result<Option<ProofIndex>> {
        let path = self.proof_index_path(session_id);

        if !path.exists() {
            return Ok(None);
        }

        let json = fs::read_to_string(&path).map_err(RepositoryError::Io)?;
        let index: ProofIndex =
            serde_json::from_str(&json).map_err(|e| RepositoryError::Json(e.to_string()))?;

        tracing::debug!(
            "Loaded proof index for session '{}': {} total proofs, proven up to nonce {}",
            index.session_id,
            index.total_proofs,
            index.proven_up_to_nonce
        );

        Ok(Some(index))
    }

    fn delete(&self, session_id: &str) -> Result<()> {
        let path = self.proof_index_path(session_id);

        if path.exists() {
            fs::remove_file(&path).map_err(RepositoryError::Io)?;
            tracing::info!("Deleted proof index: {}", path.display());
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
                    .strip_prefix("proof_index_")
                    .and_then(|s| s.strip_suffix(".json"))
            {
                sessions.push(session_id.to_string());
            }
        }

        sessions.sort();
        Ok(sessions)
    }
}
