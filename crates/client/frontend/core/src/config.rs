//! Frontend configuration structures and loaders.
//!
//! This module contains UI-specific configuration that is shared across
//! different frontend implementations (CLI, GUI, etc.).

use std::env;

/// Frontend-specific configuration.
///
/// This contains UI-related settings like message filtering and channel buffers.
#[derive(Clone, Debug)]
pub struct FrontendConfig {
    pub channels: ChannelConfig,
    pub messages: MessageConfig,
}

impl Default for FrontendConfig {
    fn default() -> Self {
        Self {
            channels: ChannelConfig::default(),
            messages: MessageConfig::default(),
        }
    }
}

impl FrontendConfig {
    pub const fn new(channels: ChannelConfig, messages: MessageConfig) -> Self {
        Self { channels, messages }
    }

    /// Construct configuration from process environment variables.
    ///
    /// Environment variables:
    /// - `CLI_ACTION_BUFFER` - Action queue size (default: 10)
    /// - `CLI_MESSAGE_CAPACITY` - Message log capacity (default: 64)
    /// - `SHOW_DAMAGE_MESSAGES` - Show damage effect messages (default: true)
    /// - `SHOW_HEALING_MESSAGES` - Show healing effect messages (default: true)
    /// - `SHOW_MOVEMENT_MESSAGES` - Show movement effect messages (default: false)
    /// - `SHOW_STATUS_MESSAGES` - Show status effect messages (default: true)
    /// - `SHOW_RESOURCE_MESSAGES` - Show resource change messages (default: false)
    /// - `SHOW_SUMMON_MESSAGES` - Show summon messages (default: true)
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

        // Effect visibility settings
        if let Some(show) = read_env_bool("SHOW_DAMAGE_MESSAGES") {
            config.messages.effect_visibility.show_damage = show;
        }
        if let Some(show) = read_env_bool("SHOW_HEALING_MESSAGES") {
            config.messages.effect_visibility.show_healing = show;
        }
        if let Some(show) = read_env_bool("SHOW_MOVEMENT_MESSAGES") {
            config.messages.effect_visibility.show_movement = show;
        }
        if let Some(show) = read_env_bool("SHOW_STATUS_MESSAGES") {
            config.messages.effect_visibility.show_status = show;
        }
        if let Some(show) = read_env_bool("SHOW_RESOURCE_MESSAGES") {
            config.messages.effect_visibility.show_resource = show;
        }
        if let Some(show) = read_env_bool("SHOW_SUMMON_MESSAGES") {
            config.messages.effect_visibility.show_summon = show;
        }

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
    pub effect_visibility: EffectVisibility,
}

impl Default for MessageConfig {
    fn default() -> Self {
        Self {
            capacity: 64,
            effect_visibility: EffectVisibility::default(),
        }
    }
}

/// Controls which effect types generate visible messages.
///
/// This is cross-frontend configuration - both CLI and GUI clients
/// can use this to filter which effects to display in their message logs.
#[derive(Clone, Debug)]
pub struct EffectVisibility {
    /// Show damage effects (e.g., "Goblin#5 takes 12 damage").
    pub show_damage: bool,
    /// Show healing effects (e.g., "Player heals 15 HP").
    pub show_healing: bool,
    /// Show movement effects (e.g., "Player moves north").
    pub show_movement: bool,
    /// Show status effects (e.g., "Goblin#5 is poisoned for 3 turns").
    pub show_status: bool,
    /// Show resource changes (e.g., "Player gains 10 MP").
    pub show_resource: bool,
    /// Show summon effects (e.g., "Wizard summons Skeleton#3").
    pub show_summon: bool,
}

impl Default for EffectVisibility {
    fn default() -> Self {
        Self {
            show_damage: true,
            show_healing: true,
            show_movement: false, // Movement is visually obvious on map
            show_status: true,
            show_resource: false, // Resource changes shown in stats panel
            show_summon: true,
        }
    }
}

impl EffectVisibility {
    /// Returns true if messages should be generated for this effect.
    pub fn should_show(&self, applied_value: &game_core::action::AppliedValue) -> bool {
        use game_core::action::AppliedValue;

        match applied_value {
            AppliedValue::Damage { .. } => self.show_damage,
            AppliedValue::Healing { .. } => self.show_healing,
            AppliedValue::Movement { .. } => self.show_movement,
            AppliedValue::StatusApplied { .. } | AppliedValue::StatusRemoved { .. } => {
                self.show_status
            }
            AppliedValue::ResourceChange { .. } => self.show_resource,
            AppliedValue::Summon { .. } => self.show_summon,
            AppliedValue::ItemAcquired { .. } => true, // Show item acquisitions
            AppliedValue::ItemUsed { .. } => true,     // Show item usage
            AppliedValue::None => false,               // Never show empty effects
        }
    }
}

fn read_env<T>(key: &str) -> Option<T>
where
    T: std::str::FromStr,
{
    env::var(key).ok()?.parse().ok()
}

fn read_env_bool(key: &str) -> Option<bool> {
    match env::var(key).ok()?.to_lowercase().as_str() {
        "true" | "1" | "yes" | "on" => Some(true),
        "false" | "0" | "no" | "off" => Some(false),
        _ => None,
    }
}
