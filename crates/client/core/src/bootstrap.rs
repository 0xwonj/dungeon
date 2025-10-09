//! Builds the runtime, oracles, and config bundle used by front-ends.
use std::sync::Arc;

use anyhow::Result;
use runtime::{Runtime, WaitActionProvider};

use crate::config::CliConfig;
use crate::world::{OracleBundle, OracleFactory, TestOracleFactory};

/// Builder that assembles runtime state, oracles, and configuration for clients.
pub struct ClientBootstrap {
    config: CliConfig,
    oracle_factory: Arc<dyn OracleFactory>,
}

impl ClientBootstrap {
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

        let mut runtime = Runtime::builder().oracles(manager).build().await?;
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
