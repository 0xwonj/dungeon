//! Item catalog loader.

use std::path::Path;

use game_core::ItemDefinition;
use serde::{Deserialize, Serialize};

use crate::loaders::{LoadResult, read_file};

/// Item catalog structure for RON files.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ItemCatalog {
    pub items: Vec<ItemDefinition>,
}

/// Loader for item catalog from RON files.
pub struct ItemLoader;

impl ItemLoader {
    /// Load item catalog from a RON file.
    ///
    /// # Arguments
    ///
    /// * `path` - Path to the RON file containing ItemCatalog
    ///
    /// # Returns
    ///
    /// Returns a Vec of ItemDefinitions.
    pub fn load(path: &Path) -> LoadResult<Vec<ItemDefinition>> {
        let content = read_file(path)?;
        let catalog: ItemCatalog = ron::from_str(&content)
            .map_err(|e| anyhow::anyhow!("Failed to parse item catalog RON: {}", e))?;

        Ok(catalog.items)
    }
}
