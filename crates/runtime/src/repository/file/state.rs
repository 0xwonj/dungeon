//! File-based StateRepository implementation.

use std::fs;
use std::path::{Path, PathBuf};

use game_core::GameState;

use crate::api::Result;
use crate::repository::{RepositoryError, StateRepository};

/// File-based implementation of StateRepository.
///
/// Stores game states as individual bincode files indexed by nonce.
///
/// # File Format
///
/// States are stored as `state_{nonce}.bin` in bincode format for:
/// - Compact size
/// - Fast serialization/deserialization
/// - Support for complex types (HashMap with non-string keys)
pub struct FileStateRepository {
    base_dir: PathBuf,
}

impl FileStateRepository {
    /// Create a new file-based state repository.
    pub fn new(base_dir: impl AsRef<Path>) -> Result<Self> {
        let base_dir = base_dir.as_ref().to_path_buf();
        fs::create_dir_all(&base_dir).map_err(RepositoryError::Io)?;
        Ok(Self { base_dir })
    }

    /// Get the path to a state file.
    fn state_path(&self, nonce: u64) -> PathBuf {
        self.base_dir.join(format!("state_{}.bin", nonce))
    }
}

impl StateRepository for FileStateRepository {
    fn save(&self, nonce: u64, state: &GameState) -> Result<()> {
        let path = self.state_path(nonce);
        let temp_path = path.with_extension("bin.tmp");

        // Serialize to bincode
        let bytes =
            bincode::serialize(state).map_err(|e| RepositoryError::Serialization(e.to_string()))?;

        // Write to temp file
        fs::write(&temp_path, bytes).map_err(RepositoryError::Io)?;

        // Atomic rename
        fs::rename(&temp_path, &path).map_err(RepositoryError::Io)?;

        tracing::debug!("Saved state[{}] to {}", nonce, path.display());

        Ok(())
    }

    fn load(&self, nonce: u64) -> Result<Option<GameState>> {
        let path = self.state_path(nonce);

        if !path.exists() {
            return Ok(None);
        }

        let bytes = fs::read(&path).map_err(RepositoryError::Io)?;
        let state: GameState = bincode::deserialize(&bytes)
            .map_err(|e| RepositoryError::Serialization(e.to_string()))?;

        tracing::debug!("Loaded state[{}] from {}", nonce, path.display());

        Ok(Some(state))
    }

    fn exists(&self, nonce: u64) -> bool {
        self.state_path(nonce).exists()
    }

    fn delete(&self, nonce: u64) -> Result<()> {
        let path = self.state_path(nonce);

        if path.exists() {
            fs::remove_file(&path).map_err(RepositoryError::Io)?;
            tracing::debug!("Deleted state[{}]", nonce);
        }

        Ok(())
    }

    fn list_nonces(&self) -> Result<Vec<u64>> {
        let mut nonces = Vec::new();

        let entries = fs::read_dir(&self.base_dir).map_err(RepositoryError::Io)?;

        for entry in entries {
            let entry = entry.map_err(RepositoryError::Io)?;
            let path = entry.path();

            if let Some(filename) = path.file_name().and_then(|s| s.to_str())
                && let Some(nonce_str) = filename
                    .strip_prefix("state_")
                    .and_then(|s| s.strip_suffix(".bin"))
                && let Ok(nonce) = nonce_str.parse::<u64>()
            {
                nonces.push(nonce);
            }
        }

        nonces.sort_unstable();
        Ok(nonces)
    }
}
