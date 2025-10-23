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
    /// Maximum number of actors (player + NPCs). Player (id=0) + up to 128 NPCs.
    pub const MAX_ACTORS: usize = 129;
    /// DEPRECATED: Use MAX_ACTORS instead. Kept for compatibility during migration.
    #[deprecated(note = "Use MAX_ACTORS instead")]
    pub const MAX_NPCS: usize = 128;
    pub const MAX_PROPS: usize = 256;
    pub const MAX_WORLD_ITEMS: usize = 512;
    pub const MAX_INVENTORY_SLOTS: usize = 8;
    pub const MAX_OVERLAYS_PER_TILE: usize = 4;
    pub const MAX_OCCUPANTS_PER_TILE: usize = 4;
    pub const MAX_ABILITIES: usize = 16;
    pub const MAX_ACTIONS: usize = 12;
    pub const MAX_PASSIVES: usize = 8;
    pub const MAX_STATUS_EFFECTS: usize = 8;

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
