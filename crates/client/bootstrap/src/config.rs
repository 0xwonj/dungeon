//! CLI runtime configuration structures and loaders.
use std::env;

/// Configuration required to bootstrap a client runtime and UI.
#[derive(Clone, Debug, Default)]
pub struct CliConfig {
    pub channels: ChannelConfig,
    pub messages: MessageConfig,
    pub enable_proving: bool,
    pub enable_persistence: bool,
    pub session_id: Option<String>,
    pub save_data_dir: Option<std::path::PathBuf>,
    pub checkpoint_interval: Option<u64>,
}

impl CliConfig {
    pub const fn new(channels: ChannelConfig, messages: MessageConfig) -> Self {
        Self {
            channels,
            messages,
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
    /// - `CLI_ACTION_BUFFER` - Action queue size (default: 10)
    /// - `CLI_MESSAGE_CAPACITY` - Message log capacity (default: 64)
    pub fn from_env() -> Self {
        let mut config = Self::default();

        // Channel configuration
        if let Some(capacity) = read_env::<usize>("CLI_ACTION_BUFFER") {
            config.channels.action_buffer = capacity.max(1);
        }

        // Message configuration
        if let Some(capacity) = read_env::<usize>("CLI_MESSAGE_CAPACITY") {
            config.messages.capacity = capacity.max(1);
        }

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
        config.save_data_dir = env::var("SAVE_DATA_DIR").ok().map(std::path::PathBuf::from);

        // Checkpoint interval (optional)
        config.checkpoint_interval = read_env::<u64>("CHECKPOINT_INTERVAL");

        config
    }
}

#[derive(Clone, Debug)]
pub struct ChannelConfig {
    pub action_buffer: usize,
}

impl Default for ChannelConfig {
    fn default() -> Self {
        Self { action_buffer: 10 }
    }
}

#[derive(Clone, Debug)]
pub struct MessageConfig {
    pub capacity: usize,
}

impl Default for MessageConfig {
    fn default() -> Self {
        Self { capacity: 64 }
    }
}

fn read_env<T>(key: &str) -> Option<T>
where
    T: std::str::FromStr,
{
    env::var(key).ok()?.parse().ok()
}
