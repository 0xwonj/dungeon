//! Helpers for constructing oracle bundles consumed by the runtime.
use std::path::PathBuf;
use std::sync::Arc;

use runtime::{ActionOracleImpl, ActorOracleImpl, ConfigOracleImpl, ItemOracleImpl, MapOracleImpl};

// Re-export OracleBundle from runtime
pub use runtime::OracleBundle;

pub trait OracleFactory: Send + Sync {
    fn build(&self) -> OracleBundle;
}

/// Oracle factory that loads game content from data files.
///
/// This factory uses the game-content crate's loaders to read RON/TOML files
/// and construct oracle implementations from real game data.
///
/// # Directory Structure
///
/// The factory expects the following files in the data directory:
/// ```text
/// data_dir/
/// ├── config.toml
/// ├── tables.toml (placeholder)
/// ├── items.ron
/// ├── npcs.ron
/// └── maps/
///     └── {map_name}.ron
/// ```
#[derive(Clone, Debug)]
pub struct ContentOracleFactory {
    data_dir: PathBuf,
    map_name: String,
}

impl ContentOracleFactory {
    /// Create a new content oracle factory.
    ///
    /// # Arguments
    ///
    /// * `data_dir` - Path to directory containing data files
    /// * `map_name` - Name of the map to load (without .ron extension)
    pub fn new(data_dir: impl Into<PathBuf>, map_name: impl Into<String>) -> Self {
        Self {
            data_dir: data_dir.into(),
            map_name: map_name.into(),
        }
    }

    /// Create with default paths (game-content/data, map "test_dungeon").
    ///
    /// This tries to find the data directory in the following order:
    /// 1. CONTENT_DATA_DIR environment variable
    /// 2. Relative to current executable (../../crates/game/content/data)
    /// 3. Relative to current directory (crates/game/content/data)
    pub fn default_paths() -> Self {
        let data_dir = if let Ok(env_dir) = std::env::var("CONTENT_DATA_DIR") {
            // Use environment variable if set
            PathBuf::from(env_dir)
        } else if let Ok(exe_path) = std::env::current_exe() {
            // Try relative to executable (when installed/released)
            exe_path
                .parent()
                .and_then(|p| p.parent())
                .and_then(|p| p.parent())
                .map(|root| root.join("crates/game/content/data"))
                .unwrap_or_else(|| {
                    // Fallback to current directory (development)
                    std::env::current_dir()
                        .unwrap_or_else(|_| PathBuf::from("."))
                        .join("crates/game/content/data")
                })
        } else {
            // Last fallback
            PathBuf::from("crates/game/content/data")
        };

        Self::new(data_dir, "test_dungeon")
    }
}

impl OracleFactory for ContentOracleFactory {
    fn build(&self) -> OracleBundle {
        use game_content::ContentFactory;

        // Verify data directory exists
        if !self.data_dir.exists() {
            panic!(
                "Content data directory not found: {}\n\
                 Current working directory: {}\n\
                 Set CONTENT_DATA_DIR environment variable to override.",
                self.data_dir.display(),
                std::env::current_dir()
                    .map(|p| p.display().to_string())
                    .unwrap_or_else(|_| "<unknown>".to_string())
            );
        }

        let factory = ContentFactory::new(&self.data_dir);

        // Load config
        let config = factory.load_config().unwrap_or_else(|e| {
            panic!(
                "Failed to load config.toml from {}: {}",
                self.data_dir.display(),
                e
            )
        });

        // Load items
        let item_definitions = factory.load_items().unwrap_or_else(|e| {
            panic!(
                "Failed to load items.ron from {}: {}",
                self.data_dir.display(),
                e
            )
        });

        // Load trait registry
        let trait_registry = factory.load_trait_registry().unwrap_or_else(|e| {
            panic!(
                "Failed to load trait registry from {}: {}",
                self.data_dir.display(),
                e
            )
        });

        // Load actors with trait profiles
        let actor_data = factory.load_actors(&trait_registry).unwrap_or_else(|e| {
            panic!(
                "Failed to load actors.ron from {}: {}",
                self.data_dir.display(),
                e
            )
        });

        // Load map (terrain only, no entities)
        let (dimensions, tiles) = factory.load_map(&self.map_name).unwrap_or_else(|e| {
            panic!(
                "Failed to load map '{}' from {}: {}",
                self.map_name,
                self.data_dir.display(),
                e
            )
        });

        // Build actor oracle with templates (trait profiles already resolved by ActorLoader)
        let mut actor_oracle = ActorOracleImpl::new();
        for (actor_id, template) in actor_data {
            // ActorLoader has already resolved trait_profile and set it in template
            actor_oracle.add(actor_id, template);
        }

        // Build item oracle
        let mut item_oracle = ItemOracleImpl::new();
        for item_def in item_definitions {
            item_oracle.add_definition(item_def);
        }

        // Build map oracle (terrain only)
        let map_oracle = MapOracleImpl::new(dimensions, tiles);

        // Build other oracles
        let actions_oracle = ActionOracleImpl::new();
        let config_oracle = ConfigOracleImpl::new(config);

        OracleBundle::new(
            Arc::new(map_oracle),
            Arc::new(item_oracle),
            Arc::new(actions_oracle),
            Arc::new(actor_oracle),
            Arc::new(config_oracle),
        )
    }
}
