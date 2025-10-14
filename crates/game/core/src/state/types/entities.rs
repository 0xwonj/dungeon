use arrayvec::ArrayVec;
use bounded_vector::BoundedVec;

use super::{EntityId, Position, Tick};
use crate::config::GameConfig;
use crate::stats::{
    ActorBonuses, CoreStats, ResourceCurrent, StatsSnapshot, compute_actor_bonuses,
};

/// Aggregate state for every entity in the map.
#[derive(Clone, Debug, PartialEq, Eq, Default)]
pub struct EntitiesState {
    pub player: ActorState,
    pub npcs: BoundedVec<ActorState, 0, { GameConfig::MAX_NPCS }>,
    pub props: BoundedVec<PropState, 0, { GameConfig::MAX_PROPS }>,
    pub items: BoundedVec<ItemState, 0, { GameConfig::MAX_WORLD_ITEMS }>,
}

impl EntitiesState {
    pub fn new(
        player: ActorState,
        npcs: BoundedVec<ActorState, 0, { GameConfig::MAX_NPCS }>,
        props: BoundedVec<PropState, 0, { GameConfig::MAX_PROPS }>,
        items: BoundedVec<ItemState, 0, { GameConfig::MAX_WORLD_ITEMS }>,
    ) -> Self {
        Self {
            player,
            npcs,
            props,
            items,
        }
    }

    /// Returns a reference to an actor by ID (player or NPC).
    pub fn actor(&self, id: EntityId) -> Option<&ActorState> {
        if self.player.id == id {
            return Some(&self.player);
        }
        self.npcs.iter().find(|actor| actor.id == id)
    }

    /// Returns a mutable reference to an actor by ID (player or NPC).
    pub fn actor_mut(&mut self, id: EntityId) -> Option<&mut ActorState> {
        if self.player.id == id {
            return Some(&mut self.player);
        }
        self.npcs.iter_mut().find(|actor| actor.id == id)
    }

    /// Returns an iterator over all actors (player + NPCs).
    pub fn all_actors(&self) -> impl Iterator<Item = &ActorState> {
        std::iter::once(&self.player).chain(self.npcs.iter())
    }

    /// Returns a mutable iterator over all actors (player + NPCs).
    pub fn all_actors_mut(&mut self) -> impl Iterator<Item = &mut ActorState> {
        std::iter::once(&mut self.player).chain(self.npcs.iter_mut())
    }
}

/// Complete actor state including stats and computed bonuses.
///
/// # Design Principles
///
/// 1. **SSOT (Single Source of Truth)**: Only `core_stats` and `resources` are stored
/// 2. **Cached Bonuses**: `bonuses` field amortizes ZK proof costs
/// 3. **Snapshot Pattern**: External code uses `snapshot()` for complete stats
///
/// # Invariants
///
/// - `bonuses` must always reflect current `inventory` state
/// - Update `bonuses` whenever `inventory` changes
/// - Never expose mutable `inventory` without recomputing `bonuses`
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ActorState {
    pub id: EntityId,
    pub position: Position,

    // SSOT: Stored state
    pub core_stats: CoreStats,
    pub resources: ResourceCurrent,

    // Cached: Derived from inventory/effects
    pub bonuses: ActorBonuses,

    pub inventory: InventoryState,

    /// When this actor is scheduled to act next. None means not currently scheduled.
    pub ready_at: Option<Tick>,

    /// NPC template ID (0 for player).
    pub template_id: u16,
}

impl ActorState {
    /// Create new actor with given core stats.
    ///
    /// Resources are initialized to maximum based on core stats.
    /// Bonuses start empty (no equipment/effects).
    pub fn new(
        id: EntityId,
        position: Position,
        core_stats: CoreStats,
        inventory: InventoryState,
    ) -> Self {
        let bonuses = compute_actor_bonuses();

        // Compute initial resource maximums
        let snapshot = StatsSnapshot::create(
            &core_stats,
            &bonuses,
            &ResourceCurrent::new(0, 0, 0), // Dummy
        );

        let resources = ResourceCurrent::at_max(&snapshot.resource_max);

        Self {
            id,
            position,
            core_stats,
            resources,
            bonuses,
            inventory,
            ready_at: None,
            template_id: 0,
        }
    }

    /// Create complete stats snapshot with all bonuses applied.
    ///
    /// This is the primary way external code should access actor stats.
    /// Ensures consistency between stored state and computed values.
    pub fn snapshot(&self) -> StatsSnapshot {
        StatsSnapshot::create(&self.core_stats, &self.bonuses, &self.resources)
    }

    /// Quick check if actor is alive (without full snapshot).
    #[inline]
    pub fn is_alive(&self) -> bool {
        self.resources.hp > 0
    }

    pub fn with_ready_at(mut self, ready_at: Tick) -> Self {
        self.ready_at = Some(ready_at);
        self
    }

    pub fn with_template_id(mut self, template_id: u16) -> Self {
        self.template_id = template_id;
        self
    }
}

impl Default for ActorState {
    fn default() -> Self {
        Self::new(
            EntityId::default(),
            Position::default(),
            CoreStats::default(),
            InventoryState::default(),
        )
    }
}

/// Simplified inventory snapshot; expand as item systems mature.
#[derive(Clone, Debug, PartialEq, Eq, Default)]
pub struct InventoryState {
    pub items: ArrayVec<ItemHandle, { GameConfig::MAX_INVENTORY_SLOTS }>,
}

impl InventoryState {
    pub fn new(items: ArrayVec<ItemHandle, { GameConfig::MAX_INVENTORY_SLOTS }>) -> Self {
        Self { items }
    }
}

/// Non-actor entities such as doors, switches, or hazards.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PropState {
    pub id: EntityId,
    pub position: Position,
    pub kind: PropKind,
    pub is_active: bool,
}

impl PropState {
    pub fn new(id: EntityId, position: Position, kind: PropKind, is_active: bool) -> Self {
        Self {
            id,
            position,
            kind,
            is_active,
        }
    }
}

/// Enumerates the basic prop categories. Extend as needed by gameplay.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum PropKind {
    Door,
    Switch,
    Hazard,
    Other,
}

/// Items that exist on the ground (not inside inventories).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ItemState {
    pub id: EntityId,
    pub position: Position,
    pub handle: ItemHandle,
}

impl ItemState {
    pub fn new(id: EntityId, position: Position, handle: ItemHandle) -> Self {
        Self {
            id,
            position,
            handle,
        }
    }
}

/// Reference to an item definition stored outside the core (lookup via Env).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct ItemHandle(pub u32);
