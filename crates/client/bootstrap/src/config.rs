//! Runtime configuration structures and loaders.
//!
//! This module contains runtime-specific configuration (proving, persistence, etc.)
//! that is shared across all client types.

use std::env;
use std::path::PathBuf;

/// Configuration for runtime initialization.
///
/// This contains only runtime-related settings (ZK proving, persistence, session management).
/// UI-specific configuration has been moved to `client-frontend-core::FrontendConfig`.
#[derive(Clone, Debug, Default)]
pub struct RuntimeConfig {
    pub enable_proving: bool,
    pub enable_persistence: bool,
    pub session_id: Option<String>,
    pub save_data_dir: Option<PathBuf>,
    pub checkpoint_interval: Option<u64>,
}

impl RuntimeConfig {
    pub const fn new() -> Self {
        Self {
            enable_proving: false,
            enable_persistence: false,
            session_id: None,
            save_data_dir: None,
            checkpoint_interval: None,
        }
    }

    /// Construct configuration from process environment variables.
    ///
    /// Environment variables:
    /// - `ENABLE_ZK_PROVING` - Enable ZK proof generation (default: false)
    /// - `ENABLE_PERSISTENCE` - Enable state persistence (default: false)
    /// - `GAME_SESSION_ID` - Session identifier for save files (default: auto-generated)
    /// - `SAVE_DATA_DIR` - Directory for save data (default: platform-specific)
    /// - `CHECKPOINT_INTERVAL` - Actions between checkpoints (default: 10)
    pub fn from_env() -> Self {
        let mut config = Self::default();

        // Enable ZK proving if environment variable is set
        if let Some(enable) = read_env::<bool>("ENABLE_ZK_PROVING") {
            config.enable_proving = enable;
        } else if env::var("ENABLE_ZK_PROVING").is_ok() {
            // Also accept just setting the variable without value as "true"
            config.enable_proving = true;
        }

        // Enable persistence if environment variable is set
        if let Some(enable) = read_env::<bool>("ENABLE_PERSISTENCE") {
            config.enable_persistence = enable;
        } else if env::var("ENABLE_PERSISTENCE").is_ok() {
            config.enable_persistence = true;
        }

        // Session ID (optional)
        config.session_id = env::var("GAME_SESSION_ID").ok();

        // Save data directory (optional)
        config.save_data_dir = env::var("SAVE_DATA_DIR").ok().map(PathBuf::from);

        // Checkpoint interval (optional)
        config.checkpoint_interval = read_env::<u64>("CHECKPOINT_INTERVAL");

        config
    }
}

fn read_env<T>(key: &str) -> Option<T>
where
    T: std::str::FromStr,
{
    env::var(key).ok()?.parse().ok()
}
