//! Content loaders for reading game data from files.
//!
//! This module provides loaders that convert RON/TOML files into oracle implementations.
//! All loaders use the formats defined in [`crate::formats`].

pub mod actors;
pub mod config;
pub mod factory;
pub mod item;
pub mod map;
pub mod tables;
pub mod traits;

pub use actors::{ActorLoader, AiKindSpec, InteractiveKindSpec, ProviderKindSpec};
pub use config::ConfigLoader;
pub use factory::ContentFactory;
pub use item::ItemLoader;
pub use map::MapLoader;
pub use tables::TablesLoader;
pub use traits::load_trait_registry;

use std::path::Path;

/// Common result type for loaders.
pub type LoadResult<T> = anyhow::Result<T>;

/// Helper function to read file contents.
pub(crate) fn read_file(path: &Path) -> LoadResult<String> {
    std::fs::read_to_string(path)
        .map_err(|e| anyhow::anyhow!("Failed to read file {}: {}", path.display(), e))
}
