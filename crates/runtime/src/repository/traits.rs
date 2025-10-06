use game_core::{EntityId, GameState, Position, StaticTile};

use crate::error::Result;

/// Repository for game state persistence and loading
/// Note: Sync for MVP (in-memory). Can be made async later for DB.
pub trait StateRepository: Send + Sync {
    /// Load the current game state
    fn load(&self) -> Result<GameState>;

    /// Save the game state
    fn save(&self, state: &GameState) -> Result<()>;
}

/// Repository for map data
/// Note: Sync for MVP (in-memory). Can be made async later for DB.
pub trait MapRepository: Send + Sync {
    /// Get tile at position
    fn get_tile(&self, pos: Position) -> Result<Option<StaticTile>>;

    /// Get entities near a position within radius
    fn get_nearby_entities(&self, center: Position, radius: u32)
        -> Result<Vec<(EntityId, Position)>>;
}

/// Repository for NPC data and AI configuration
/// Note: Sync for MVP (in-memory). Can be made async later.
pub trait NpcRepository: Send + Sync {
    /// List all NPC entity IDs
    fn list_npcs(&self) -> Result<Vec<EntityId>>;
}
