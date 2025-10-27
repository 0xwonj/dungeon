//! Game configuration loader.

use std::path::Path;

use game_core::GameConfig;

use crate::loaders::{LoadResult, read_file};

/// Loader for game configuration from TOML files.
pub struct ConfigLoader;

impl ConfigLoader {
    /// Load config data from a TOML file.
    ///
    /// # Arguments
    ///
    /// * `path` - Path to the TOML file containing GameConfig
    ///
    /// # Returns
    ///
    /// Returns a GameConfig.
    pub fn load(path: &Path) -> LoadResult<GameConfig> {
        let content = read_file(path)?;
        let config: GameConfig = toml::from_str(&content)
            .map_err(|e| anyhow::anyhow!("Failed to parse config TOML: {}", e))?;

        Ok(config)
    }
}
