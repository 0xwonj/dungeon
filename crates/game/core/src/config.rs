/// Game configuration constants and tunable parameters.
#[derive(Clone, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct GameConfig {
    /// Radius around the player within which NPCs are activated and scheduled.
    /// Entities outside this radius are deactivated to save computation.
    pub activation_radius: u32,
}

impl GameConfig {
    // ===== compile-time constants used as type parameters =====
    pub const MAX_NPCS: usize = 128;
    pub const MAX_PROPS: usize = 256;
    pub const MAX_WORLD_ITEMS: usize = 512;
    pub const MAX_INVENTORY_SLOTS: usize = 8;
    pub const MAX_OVERLAYS_PER_TILE: usize = 4;
    pub const MAX_OCCUPANTS_PER_TILE: usize = 4;

    // ===== runtime-tunable defaults =====
    pub const DEFAULT_ACTIVATION_RADIUS: u32 = 5;

    pub fn new() -> Self {
        Self {
            activation_radius: Self::DEFAULT_ACTIVATION_RADIUS,
        }
    }

    pub fn with_activation_radius(activation_radius: u32) -> Self {
        Self { activation_radius }
    }
}

impl Default for GameConfig {
    fn default() -> Self {
        Self::new()
    }
}
