//! Authoritative game state representation.
//!
//! This module owns the data structures that describe entities, turn
//! bookkeeping, overlays, and initialization helpers. Runtime layers clone or
//! query this state but mutate it exclusively through the engine.
pub mod delta;
pub mod types;

use crate::env::MapOracle;
pub use bounded_vector::BoundedVec;
pub use delta::{
    ActorChanges, ActorFields, CollectionChanges, EntitiesChanges, ItemChanges, ItemFields,
    OccupancyChanges, PropChanges, PropFields, StateDelta, TurnChanges, TurnFields, WorldChanges,
};
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

    /// Allocates a new unique EntityId.
    ///
    /// # Returns
    ///
    /// A new EntityId that has never been used before.
    ///
    /// # Panics
    ///
    /// Panics if we've exhausted all available IDs.
    pub fn allocate_entity_id(&mut self) -> EntityId {
        // Skip reserved IDs (0 = PLAYER, u32::MAX = SYSTEM)
        while self.next_entity_id == EntityId::PLAYER.0 || self.next_entity_id == EntityId::SYSTEM.0
        {
            self.next_entity_id = self
                .next_entity_id
                .checked_add(1)
                .expect("EntityId overflow");
        }

        let id = EntityId(self.next_entity_id);
        self.next_entity_id = self
            .next_entity_id
            .checked_add(1)
            .expect("EntityId overflow");

        id
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
    /// - `Err` if the actors list is full
    pub fn add_player(
        &mut self,
        template: &crate::env::ActorTemplate,
        position: Position,
    ) -> Result<(), &'static str> {
        // Create actor from template with PLAYER id
        let mut actor = template.to_actor(EntityId::PLAYER, position);
        actor.ready_at = Some(0); // Ready to act immediately

        // Add to actors list
        self.entities
            .actors
            .push(actor)
            .map_err(|_| "Failed to add player (actors list full)")?;

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
    /// - `Err` if the actors list is full
    pub fn add_npc(
        &mut self,
        template: &crate::env::ActorTemplate,
        position: Position,
    ) -> Result<EntityId, &'static str> {
        // Allocate new entity ID
        let id = self.allocate_entity_id();

        // Create actor from template with allocated id
        let mut actor = template.to_actor(id, position);
        actor.ready_at = None; // Inactive by default

        // Add to actors list
        self.entities
            .actors
            .push(actor)
            .map_err(|_| "Failed to add NPC (actors list full)")?;

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
}
