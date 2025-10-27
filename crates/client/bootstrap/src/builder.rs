//! Builds the runtime, oracles, and config bundle used by front-ends.
use std::path::PathBuf;
use std::sync::Arc;

use anyhow::Result;
use runtime::{AiKind, ProviderKind, Runtime, Scenario, UtilityAiProvider};

use crate::config::CliConfig;
use crate::oracles::{ContentOracleFactory, OracleBundle, OracleFactory};

/// Builder that assembles runtime state, oracles, and configuration for clients.
pub struct RuntimeBuilder {
    config: CliConfig,
    oracle_factory: Arc<dyn OracleFactory>,
}

impl RuntimeBuilder {
    /// Create a new RuntimeBuilder with data-driven content from game-content.
    ///
    /// This uses ContentOracleFactory by default, loading content from RON/TOML files.
    /// Use `oracle_factory()` to override with a custom factory.
    pub fn new(config: CliConfig) -> Self {
        let default_factory = ContentOracleFactory::default_paths();
        Self {
            config,
            oracle_factory: Arc::new(default_factory),
        }
    }

    /// Provide a custom oracle factory (e.g., game-content backed implementation).
    pub fn oracle_factory(mut self, factory: impl OracleFactory + 'static) -> Self {
        self.oracle_factory = Arc::new(factory);
        self
    }

    /// Find scenario file path.
    ///
    /// Looks for test_scenario.ron in the data directory.
    fn find_scenario_path(&self) -> Option<PathBuf> {
        // Try to find data directory (same logic as ContentOracleFactory)
        let data_dir = if let Ok(env_dir) = std::env::var("CONTENT_DATA_DIR") {
            PathBuf::from(env_dir)
        } else if let Ok(exe_path) = std::env::current_exe() {
            exe_path
                .parent()
                .and_then(|p| p.parent())
                .and_then(|p| p.parent())
                .map(|root| root.join("crates/game/content/data"))
                .unwrap_or_else(|| {
                    std::env::current_dir()
                        .unwrap_or_else(|_| PathBuf::from("."))
                        .join("crates/game/content/data")
                })
        } else {
            PathBuf::from("crates/game/content/data")
        };

        let scenario_path = data_dir.join("scenarios/test_scenario.ron");
        if scenario_path.exists() {
            Some(scenario_path)
        } else {
            None
        }
    }

    pub async fn build(self) -> Result<RuntimeSetup> {
        let oracles = self.oracle_factory.build();
        let manager = oracles.manager();

        let mut builder = Runtime::builder().oracles(manager.clone());

        // Load scenario if available
        // Try to find scenario file in data directory
        let scenario_path = self.find_scenario_path();
        if let Some(path) = scenario_path {
            match Scenario::load_from_file(&path) {
                Ok(scenario) => {
                    tracing::info!("Loaded scenario from {}", path.display());
                    builder = builder.scenario(scenario);
                }
                Err(e) => {
                    tracing::warn!("Failed to load scenario from {}: {}", path.display(), e);
                }
            }
        } else {
            tracing::info!("No scenario file found, using default state");
        }

        // Enable proving if requested
        builder = builder.enable_proving(self.config.enable_proving);

        // Enable persistence if requested
        builder = builder.enable_persistence(self.config.enable_persistence);

        // Set session ID if provided
        if let Some(ref session_id) = self.config.session_id {
            builder = builder.session_id(session_id.clone());
        }

        // Set save data directory if provided
        if let Some(ref dir) = self.config.save_data_dir {
            builder = builder.persistence_dir(dir.clone());
        }

        // Set checkpoint interval if provided
        if let Some(interval) = self.config.checkpoint_interval {
            builder = builder.checkpoint_interval(interval);
        }

        // Build the runtime
        let runtime = builder.build().await?;

        // Register AI providers
        let utility_ai_kind = ProviderKind::Ai(AiKind::Utility);
        let handle = runtime.handle();

        // Register UtilityAiProvider for all NPCs
        // This provider uses 3-layer decision making (Intent → Tactic → Action)
        // All behavior is driven by TraitProfile from game-content
        handle.register_provider(utility_ai_kind, UtilityAiProvider::new())?;

        // Set UtilityAi as default for all NPCs
        handle.set_default_provider(utility_ai_kind)?;

        Ok(RuntimeSetup {
            config: self.config,
            oracles,
            runtime,
        })
    }
}

pub struct RuntimeSetup {
    pub config: CliConfig,
    pub oracles: OracleBundle,
    pub runtime: Runtime,
}
