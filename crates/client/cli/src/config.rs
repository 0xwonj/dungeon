//! CLI-specific configuration for terminal UI.
use std::env;

/// CLI terminal UI configuration.
///
/// This contains settings specific to the terminal interface,
/// separate from cross-frontend client configuration.
#[derive(Clone, Debug, Default)]
pub struct CliConfig {
    pub ui: UiConfig,
}

impl CliConfig {
    /// Construct CLI configuration from environment variables.
    ///
    /// Environment variables:
    /// - `CLI_MESSAGE_PANEL_HEIGHT` - Message panel height in lines (default: 10)
    pub fn from_env() -> Self {
        let mut config = Self::default();

        if let Some(height) = read_env::<u16>("CLI_MESSAGE_PANEL_HEIGHT") {
            config.ui.message_panel_height = height.max(3);
        }

        config
    }
}

/// UI layout and display configuration.
#[derive(Clone, Debug)]
pub struct UiConfig {
    /// Height of message panel in lines (including borders).
    pub message_panel_height: u16,
}

impl Default for UiConfig {
    fn default() -> Self {
        Self {
            message_panel_height: 10,
        }
    }
}

fn read_env<T>(key: &str) -> Option<T>
where
    T: std::str::FromStr,
{
    env::var(key).ok()?.parse().ok()
}
