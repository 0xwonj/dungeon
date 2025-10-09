//! CLI runtime configuration structures and loaders.
use std::env;

/// Configuration required to bootstrap a client runtime and UI.
#[derive(Clone, Debug, Default)]
pub struct CliConfig {
    pub world: WorldConfig,
    pub channels: ChannelConfig,
    pub messages: MessageConfig,
}

impl CliConfig {
    pub const fn new(world: WorldConfig, channels: ChannelConfig, messages: MessageConfig) -> Self {
        Self {
            world,
            channels,
            messages,
        }
    }

    /// Construct configuration from process environment variables.
    ///
    /// - `CLI_MAP_WIDTH` / `CLI_MAP_HEIGHT`
    /// - `CLI_ACTION_BUFFER`
    /// - `CLI_MESSAGE_CAPACITY`
    pub fn from_env() -> Self {
        let mut config = Self::default();

        if let (Some(width), Some(height)) = (
            read_env::<u32>("CLI_MAP_WIDTH"),
            read_env::<u32>("CLI_MAP_HEIGHT"),
        ) {
            config.world.size = MapSize { width, height };
        }

        if let Some(capacity) = read_env::<usize>("CLI_ACTION_BUFFER") {
            config.channels.action_buffer = capacity.max(1);
        }

        if let Some(capacity) = read_env::<usize>("CLI_MESSAGE_CAPACITY") {
            config.messages.capacity = capacity.max(1);
        }

        config
    }
}

#[derive(Clone, Debug)]
pub struct WorldConfig {
    pub size: MapSize,
}

impl Default for WorldConfig {
    fn default() -> Self {
        Self {
            size: MapSize {
                width: 10,
                height: 10,
            },
        }
    }
}

#[derive(Clone, Copy, Debug)]
pub struct MapSize {
    pub width: u32,
    pub height: u32,
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
