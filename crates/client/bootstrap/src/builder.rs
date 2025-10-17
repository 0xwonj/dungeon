//! Builds the runtime, oracles, and config bundle used by front-ends.
use std::sync::Arc;

use anyhow::Result;
use runtime::{Runtime, WaitActionProvider};

use crate::config::CliConfig;
use crate::oracles::{OracleBundle, OracleFactory, TestOracleFactory};

/// Builder that assembles runtime state, oracles, and configuration for clients.
pub struct RuntimeBuilder {
    config: CliConfig,
    oracle_factory: Arc<dyn OracleFactory>,
}

impl RuntimeBuilder {
    pub fn new(config: CliConfig) -> Self {
        let default_factory = TestOracleFactory::new(config.world.size);
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

    pub async fn build(self) -> Result<RuntimeSetup> {
        let oracles = self.oracle_factory.build();
        let manager = oracles.manager();

        let mut builder = Runtime::builder().oracles(manager);

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
        let mut runtime = builder.build().await?;
        runtime.set_npc_provider(WaitActionProvider);

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
