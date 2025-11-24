//! Shared bootstrap utilities for client front-ends.
//!
//! Provides configuration loading, oracle assembly, and runtime setup that can
//! be reused by CLI, UI, or other front-end crates.
pub mod builder;
pub mod config;
pub mod oracles;
pub mod session;

pub use builder::{RuntimeBuilder, RuntimeSetup};
pub use config::RuntimeConfig;
pub use oracles::{ContentOracleFactory, OracleBundle, OracleFactory};
pub use session::{SessionInfo, find_latest_session, list_sessions, load_latest_state};
