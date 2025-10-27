//! Configuration oracle for exposing game configuration to the engine.

/// Provides access to runtime configuration values.
pub trait ConfigOracle: Send + Sync {
    /// Returns the activation radius around the player within which NPCs are activated.
    fn activation_radius(&self) -> u32;
}
