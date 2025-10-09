//! Shared bootstrap utilities for client front-ends.
//!
//! Provides configuration loading, oracle assembly, and runtime setup that can
//! be reused by CLI, UI, or other front-end crates.
pub mod bootstrap;
pub mod config;
pub mod world;

pub use bootstrap::{ClientBootstrap, RuntimeSetup};
pub use config::CliConfig;
