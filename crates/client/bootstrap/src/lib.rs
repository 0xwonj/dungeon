//! Shared bootstrap utilities for client front-ends.
//!
//! Provides configuration loading, oracle assembly, and runtime setup that can
//! be reused by CLI, UI, or other front-end crates.
pub mod builder;
pub mod config;
pub mod oracles;

pub use builder::{RuntimeBuilder, RuntimeSetup};
pub use config::RuntimeConfig;
pub use oracles::{ContentOracleFactory, OracleBundle, OracleFactory};
