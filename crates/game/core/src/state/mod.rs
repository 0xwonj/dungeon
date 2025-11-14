//! Authoritative game state representation.
//!
//! This module owns the data structures that describe entities, turn
//! bookkeeping, overlays, and initialization helpers. Runtime layers clone or
//! query this state but mutate it exclusively through the engine.
pub mod delta;
pub mod error;
pub mod types;

use crate::config::GameConfig;
use crate::env::MapOracle;
pub use bounded_vector::BoundedVec;
pub use delta::{
    ActorChanges, ActorFields, CollectionChanges, EntitiesChanges, ItemChanges, ItemFields,
    OccupancyChanges, PropChanges, PropFields, StateDelta, TurnChanges, TurnFields, WorldChanges,
};
pub use error::StateError;
pub use types::{
    ActionAbilities, ActionAbility, ActorState, ArmorKind, AttackType, EntitiesState, EntityId,
    Equipment, EquipmentBuilder, InventorySlot, InventoryState, ItemHandle, ItemState,
    PassiveAbilities, PassiveAbility, PassiveKind, Position, PropKind, PropState, StatusEffect,
    StatusEffectKind, StatusEffects, Tick, TileMap, TileView, TurnState, WeaponKind, WorldState,
};

/// Canonical snapshot of the deterministic game state.
#[derive(Clone, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct GameState {
    /// RNG seed for deterministic random generation.
    ///
    /// Set once at game initialization and never modified.
    /// Combined with `turn.nonce` to generate unique seeds for each random event.
    pub game_seed: u64,

    /// Sequential entity ID allocator (monotonically increasing).
    ///
    /// Never reused. IDs 0 (PLAYER) and u32::MAX (SYSTEM) are reserved.
    next_entity_id: u32,

    /// Turn bookkeeping including current phase within the turn.
    pub turn: TurnState,
    /// All entities tracked in the room: actors, props, items.
    pub entities: EntitiesState,
    /// Runtime world data layered on top of the static map commitment.
    pub world: WorldState,
}

impl GameState {
    /// Creates a fresh state from the provided sub-components.
    pub fn new(turn: TurnState, entities: EntitiesState, world: WorldState) -> Self {
        Self {
            game_seed: 0,      // Default seed (should be set explicitly)
            next_entity_id: 1, // Start at 1 (0 is reserved for PLAYER)
            turn,
            entities,
            world,
        }
    }

    /// Creates a fresh state with a specific game seed.
    pub fn with_seed(
        game_seed: u64,
        turn: TurnState,
        entities: EntitiesState,
        world: WorldState,
    ) -> Self {
        Self {
            game_seed,
            next_entity_id: 1,
            turn,
            entities,
            world,
        }
    }

    /// Creates an empty state with no entities (for scenario initialization).
    ///
    /// Unlike `default()`, this does not create a default player.
    /// Use this when you'll be adding all entities explicitly (e.g., from a scenario).
    pub fn empty() -> Self {
        Self {
            game_seed: 0,
            next_entity_id: 1, // Start at 1 (0 is reserved for PLAYER)
            turn: TurnState::default(),
            entities: EntitiesState::empty(),
            world: WorldState::default(),
        }
    }

    /// Returns a merged tile view that combines static map data with runtime occupants.
    pub fn tile_view<M>(&self, map: &M, position: Position) -> Option<TileView>
    where
        M: MapOracle + ?Sized,
    {
        self.world.tile_view(map, position)
    }

    /// Determines whether a tile can be entered considering terrain passability and occupancy.
    pub fn can_enter<M>(&self, map: &M, position: Position) -> bool
    where
        M: MapOracle + ?Sized,
    {
        self.tile_view(map, position)
            .map(|view| view.is_passable() && !view.is_occupied())
            .unwrap_or(false)
    }

    /// Returns the position of an actor by ID.
    ///
    /// Returns `None` if the actor is not found or has no position.
    ///
    /// In debug builds, verifies that the actor's position matches the occupancy map.
    pub fn actor_position(&self, id: EntityId) -> Option<Position> {
        let pos = self.entities.position(id)?;

        // Debug-only: Verify occupancy map consistency
        debug_assert!(
            self.world
                .tile_map
                .occupants(&pos)
                .map(|occupants| occupants.contains(&id))
                .unwrap_or(false),
            "Actor {} has position {:?} but is not in occupancy map at that location",
            id.0,
            pos
        );

        Some(pos)
    }

    /// Allocates a new unique EntityId.
    ///
    /// # Returns
    ///
    /// - `Ok(EntityId)` - A new unique entity ID
    /// - `Err(StateError::EntityIdOverflow)` if all available IDs have been exhausted
    ///
    /// # Notes
    ///
    /// Entity IDs 0 (PLAYER) and u32::MAX (SYSTEM) are reserved and will be skipped.
    pub fn allocate_entity_id(&mut self) -> Result<EntityId, StateError> {
        // Skip reserved IDs (0 = PLAYER, u32::MAX = SYSTEM)
        while self.next_entity_id == EntityId::PLAYER.0 || self.next_entity_id == EntityId::SYSTEM.0
        {
            self.next_entity_id =
                self.next_entity_id
                    .checked_add(1)
                    .ok_or(StateError::EntityIdOverflow {
                        current: self.next_entity_id,
                    })?;
        }

        let id = EntityId(self.next_entity_id);
        self.next_entity_id =
            self.next_entity_id
                .checked_add(1)
                .ok_or(StateError::EntityIdOverflow {
                    current: self.next_entity_id,
                })?;

        Ok(id)
    }

    /// Add the player actor to the game.
    ///
    /// Player is always assigned EntityId::PLAYER, set as active, and ready to act immediately.
    ///
    /// # Arguments
    ///
    /// * `template` - Actor template defining stats, equipment, abilities
    /// * `position` - Starting position on the map
    ///
    /// # Returns
    ///
    /// - `Ok(())` if player was added successfully
    /// - `Err(StateError::ActorListFull)` if the actors list is at maximum capacity
    pub fn add_player(
        &mut self,
        template: &crate::env::ActorTemplate,
        position: Position,
    ) -> Result<(), StateError> {
        // Create actor from template with PLAYER id
        let mut actor = template.to_actor(EntityId::PLAYER, position);
        actor.ready_at = Some(0); // Ready to act immediately

        // Add to actors list
        self.entities
            .actors
            .push(actor)
            .map_err(|_| StateError::ActorListFull {
                max: GameConfig::MAX_ACTORS,
                current: self.entities.actors.len(),
            })?;

        // Add to active_actors
        self.turn.active_actors.insert(EntityId::PLAYER);

        // Update occupancy map
        self.world.tile_map.add_occupant(position, EntityId::PLAYER);

        Ok(())
    }

    /// Add an NPC actor to the game with automatic ID allocation.
    ///
    /// NPCs start inactive and will be activated by the ActivationHook when
    /// the player moves within activation radius.
    ///
    /// # Arguments
    ///
    /// * `template` - Actor template defining stats, equipment, abilities
    /// * `position` - Starting position on the map
    ///
    /// # Returns
    ///
    /// - `Ok(EntityId)` - The allocated entity ID for this NPC
    /// - `Err(StateError::ActorListFull)` if the actors list is at maximum capacity
    pub fn add_npc(
        &mut self,
        template: &crate::env::ActorTemplate,
        position: Position,
    ) -> Result<EntityId, StateError> {
        // Allocate new entity ID
        let id = self.allocate_entity_id()?;

        // Create actor from template with allocated id
        let mut actor = template.to_actor(id, position);
        actor.ready_at = None; // Inactive by default

        // Add to actors list
        self.entities
            .actors
            .push(actor)
            .map_err(|_| StateError::ActorListFull {
                max: GameConfig::MAX_ACTORS,
                current: self.entities.actors.len(),
            })?;

        // Don't add to active_actors - ActivationHook will handle this

        // Update occupancy map
        self.world.tile_map.add_occupant(position, id);

        Ok(id)
    }
}

impl GameState {
    /// Create a default game state with a player.
    pub fn with_player() -> Self {
        let mut state = Self {
            game_seed: 0,      // Default seed
            next_entity_id: 1, // Start at 1 (0 is reserved for PLAYER)
            turn: TurnState::default(),
            entities: EntitiesState::with_player(),
            world: WorldState::default(),
        };

        // IMPORTANT: Activate the default player so they can act
        // EntitiesState::with_player() creates a player actor, but doesn't add to active_actors
        // We need to ensure player is ready to act
        if let Some(player) = state.entities.actors.first_mut() {
            player.ready_at = Some(0);
        }
        state.turn.active_actors.insert(EntityId::PLAYER);

        state
    }

    /// Returns the current action nonce (sequential action counter).
    pub fn nonce(&self) -> u64 {
        self.turn.nonce
    }

    /// Computes a deterministic SHA-256 hash of the entire game state.
    ///
    /// This is used as the "state root" for ZK proofs, providing a cryptographic
    /// commitment to the complete game state at a specific point in time.
    ///
    /// # Design Choice: Simple Hash vs Merkle Tree
    ///
    /// For zkVM environments (RISC0, SP1), a simple SHA-256 hash is optimal because:
    /// - zkVM verifies the entire state transition, not partial state access
    /// - Merkle trees provide no performance benefit in this context
    /// - Simpler = fewer constraints = faster proving
    /// - SHA-256 is hardware-accelerated in RISC0 zkVM
    ///
    /// For future custom circuits (Arkworks), Merkle trees may become valuable
    /// to enable partial state updates and reduce circuit size.
    ///
    /// # Serialization
    ///
    /// Requires the `serde` feature. In zkVM environments, ensure both `zkvm` and
    /// `serde` features are enabled on the `game-core` dependency.
    #[cfg(feature = "serde")]
    pub fn compute_state_root(&self) -> [u8; 32] {
        use sha2::{Digest, Sha256};

        let mut hasher = Sha256::new();

        // Hash all state components in deterministic order
        // Using bincode for consistent binary serialization
        // Note: BTreeSet in TurnState ensures deterministic active_actors order

        // 1. Game seed (deterministic RNG source)
        hasher.update(self.game_seed.to_le_bytes());

        // 2. Entity ID allocator state
        hasher.update(self.next_entity_id.to_le_bytes());

        // 3. Turn state (nonce, clock, active_actors, current_actor)
        if let Ok(turn_bytes) = bincode::serialize(&self.turn) {
            hasher.update(&turn_bytes);
        }

        // 4. Entities state (actors, items, props)
        if let Ok(entities_bytes) = bincode::serialize(&self.entities) {
            hasher.update(&entities_bytes);
        }

        // 5. World state (tile_map occupancy)
        if let Ok(world_bytes) = bincode::serialize(&self.world) {
            hasher.update(&world_bytes);
        }

        hasher.finalize().into()
    }
}
