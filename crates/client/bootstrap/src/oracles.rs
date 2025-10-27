//! Helpers for constructing oracle bundles consumed by the runtime.
use std::path::PathBuf;
use std::sync::Arc;

use runtime::{
    ActorOracleImpl, ConfigOracleImpl, ItemOracleImpl, MapOracleImpl, OracleManager,
    TablesOracleImpl,
};

use crate::config::MapSize;

/// Bundle of oracle implementations that the runtime consumes.
#[derive(Clone)]
pub struct OracleBundle {
    pub map: Arc<MapOracleImpl>,
    pub items: Arc<ItemOracleImpl>,
    pub tables: Arc<TablesOracleImpl>,
    pub actors: Arc<ActorOracleImpl>,
    pub config: Arc<ConfigOracleImpl>,
}

impl OracleBundle {
    pub fn manager(&self) -> OracleManager {
        OracleManager::new(
            Arc::clone(&self.map),
            Arc::clone(&self.items),
            Arc::clone(&self.tables),
            Arc::clone(&self.actors),
            Arc::clone(&self.config),
        )
    }
}

pub trait OracleFactory: Send + Sync {
    fn build(&self) -> OracleBundle;
}

/// Temporary factory that relies on runtime-provided test fixtures.
///
/// As `game-content` becomes populated this will move to use real data.
#[derive(Clone, Debug)]
pub struct TestOracleFactory {
    size: MapSize,
}

impl TestOracleFactory {
    pub const fn new(size: MapSize) -> Self {
        Self { size }
    }
}

impl OracleFactory for TestOracleFactory {
    fn build(&self) -> OracleBundle {
        use game_content::traits::{TraitKind, TraitLayer, TraitProfile, TraitWeights};
        use game_core::{
            ActionAbilities, ActionAbility, ActionKind, ActorTemplate, PassiveAbilities,
            PassiveAbility, PassiveKind,
        };
        use runtime::AiConfig;

        let map = Arc::new(MapOracleImpl::test_map(self.size.width, self.size.height));
        let items = Arc::new(ItemOracleImpl::test_items());
        let tables = Arc::new(TablesOracleImpl::test_tables());
        let config = Arc::new(ConfigOracleImpl::new(game_core::GameConfig::default()));

        // Build actor oracle with test actors
        let mut oracle = ActorOracleImpl::new();

        // Add test NPCs (using string IDs now)
        // NPC 0: Weak goblin (cowardly, fast)
        let mut goblin_actions = ActionAbilities::new();
        goblin_actions.push(ActionAbility::new(ActionKind::Move));
        goblin_actions.push(ActionAbility::new(ActionKind::MeleeAttack));
        goblin_actions.push(ActionAbility::new(ActionKind::Wait));

        let mut goblin_passives = PassiveAbilities::new();
        goblin_passives.push(PassiveAbility::new(PassiveKind::Darkvision));

        oracle.add(
            "0",
            ActorTemplate::builder()
                .actions(goblin_actions)
                .passives(goblin_passives)
                .build(),
            AiConfig {
                traits: TraitProfile::compose(
                    &TraitLayer::builder()
                        .set(TraitKind::Bravery, 3)
                        .set(TraitKind::Mobility, 12)
                        .set(TraitKind::Perception, 8)
                        .set(TraitKind::Aggression, 5)
                        .build(),
                    &TraitLayer::zero(),
                    &TraitLayer::zero(),
                    &TraitLayer::zero(),
                    &TraitWeights::default_weights(),
                ),
                default_provider: runtime::ProviderKind::Ai(runtime::AiKind::Utility),
            },
        );

        OracleBundle {
            map,
            items,
            tables,
            actors: Arc::new(oracle),
            config,
        }
    }
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

// Convert game-content's ProviderKindSpec to runtime's ProviderKind
fn convert_provider_kind(spec: game_content::ProviderKindSpec) -> runtime::ProviderKind {
    use game_content::{AiKindSpec, InteractiveKindSpec, ProviderKindSpec};
    use runtime::{AiKind, InteractiveKind, ProviderKind};

    match spec {
        ProviderKindSpec::Interactive(i) => ProviderKind::Interactive(match i {
            InteractiveKindSpec::CliInput => InteractiveKind::CliInput,
            InteractiveKindSpec::NetworkInput => InteractiveKind::NetworkInput,
            InteractiveKindSpec::Replay => InteractiveKind::Replay,
        }),
        ProviderKindSpec::Ai(a) => ProviderKind::Ai(match a {
            AiKindSpec::Wait => AiKind::Wait,
            AiKindSpec::Aggressive => AiKind::Aggressive,
            AiKindSpec::Passive => AiKind::Passive,
            AiKindSpec::Scripted => AiKind::Scripted,
            AiKindSpec::Utility => AiKind::Utility,
        }),
        ProviderKindSpec::Custom(id) => ProviderKind::Custom(id),
    }
}

impl OracleFactory for ContentOracleFactory {
    fn build(&self) -> OracleBundle {
        use game_content::ContentFactory;
        use runtime::AiConfig;

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

        // Build actor oracle with templates and AI configs
        let mut actor_oracle = ActorOracleImpl::new();
        for (actor_id, template, provider_spec, trait_profile) in actor_data {
            // Convert ProviderKindSpec to runtime::ProviderKind
            let provider = convert_provider_kind(provider_spec);

            let ai_config = AiConfig {
                traits: trait_profile,
                default_provider: provider,
            };
            actor_oracle.add(actor_id, template, ai_config);
        }

        // Build item oracle
        let mut item_oracle = ItemOracleImpl::new();
        for item_def in item_definitions {
            item_oracle.add_definition(item_def);
        }

        // Build map oracle (terrain only)
        let map_oracle = MapOracleImpl::new(dimensions, tiles);

        // Build other oracles
        let tables_oracle = TablesOracleImpl::new();
        let config_oracle = ConfigOracleImpl::new(config);

        OracleBundle {
            map: Arc::new(map_oracle),
            items: Arc::new(item_oracle),
            tables: Arc::new(tables_oracle),
            actors: Arc::new(actor_oracle),
            config: Arc::new(config_oracle),
        }
    }
}
