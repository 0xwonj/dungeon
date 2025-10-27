//! Content factory for building oracles from data files.

use std::path::{Path, PathBuf};

use crate::loaders::{
    ActorLoader, ConfigLoader, ItemLoader, LoadResult, MapLoader, TablesLoader, load_trait_registry,
};
use crate::traits::TraitRegistry;

/// Content factory that loads all game content from a data directory.
///
/// # Directory Structure
///
/// ```text
/// data_dir/
/// ├── config.toml
/// ├── tables.toml
/// ├── items.ron
/// ├── npcs.ron
/// └── maps/
///     ├── test_dungeon.ron
///     └── boss_arena.ron
/// ```
pub struct ContentFactory {
    data_dir: PathBuf,
}

impl ContentFactory {
    /// Creates a new content factory pointing to a data directory.
    ///
    /// # Arguments
    ///
    /// * `data_dir` - Path to the directory containing data files
    pub fn new(data_dir: impl Into<PathBuf>) -> Self {
        Self {
            data_dir: data_dir.into(),
        }
    }

    /// Load game configuration from `config.toml`.
    pub fn load_config(&self) -> LoadResult<game_core::GameConfig> {
        let path = self.data_dir.join("config.toml");
        ConfigLoader::load(&path)
    }

    /// Load game rules tables from `tables.toml`.
    ///
    /// **PLACEHOLDER**: Currently returns empty data since TablesOracle has no methods.
    pub fn load_tables(&self) -> LoadResult<()> {
        let path = self.data_dir.join("tables.toml");
        TablesLoader::load(&path)
    }

    /// Load item catalog from `items.ron`.
    pub fn load_items(&self) -> LoadResult<Vec<game_core::ItemDefinition>> {
        let path = self.data_dir.join("items.ron");
        ItemLoader::load(&path)
    }

    /// Load trait registry from `traits/` directory.
    pub fn load_trait_registry(&self) -> LoadResult<TraitRegistry> {
        let traits_dir = self.data_dir.join("traits");
        load_trait_registry(&traits_dir)
    }

    /// Load actor catalog from `actors.ron`.
    ///
    /// Loads both players and NPCs with their templates, providers, and trait profiles.
    ///
    /// # Arguments
    ///
    /// * `trait_registry` - Registry containing trait layer presets (load via `load_trait_registry()`)
    pub fn load_actors(
        &self,
        trait_registry: &TraitRegistry,
    ) -> LoadResult<
        Vec<(
            String,
            game_core::ActorTemplate,
            crate::loaders::ProviderKindSpec,
            crate::traits::TraitProfile,
        )>,
    > {
        let path = self.data_dir.join("actors.ron");
        ActorLoader::load(&path, trait_registry)
    }

    /// Load a map from `maps/{map_name}.ron`.
    ///
    /// Returns terrain data only (no entities).
    /// For entity placement, use `load_scenario()`.
    ///
    /// # Arguments
    ///
    /// * `map_name` - Name of the map file (without `.ron` extension)
    pub fn load_map(
        &self,
        map_name: &str,
    ) -> LoadResult<(
        game_core::MapDimensions,
        std::collections::HashMap<game_core::Position, game_core::StaticTile>,
    )> {
        let path = self.data_dir.join("maps").join(format!("{}.ron", map_name));
        MapLoader::load(&path)
    }

    /// Returns the data directory path.
    pub fn data_dir(&self) -> &Path {
        &self.data_dir
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_factory_paths() {
        let factory = ContentFactory::new("/tmp/data");
        assert_eq!(factory.data_dir(), Path::new("/tmp/data"));
    }
}
