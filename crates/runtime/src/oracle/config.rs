//! Config oracle implementation for runtime.

use game_core::{GameConfig, env::ConfigOracle};

/// Runtime implementation of ConfigOracle that wraps GameConfig
pub struct ConfigOracleImpl {
    config: GameConfig,
}

impl ConfigOracleImpl {
    pub fn new(config: GameConfig) -> Self {
        Self { config }
    }
}

impl ConfigOracle for ConfigOracleImpl {
    fn activation_radius(&self) -> u32 {
        self.config.activation_radius
    }
}
