use arrayvec::ArrayVec;
use bounded_vector::BoundedVec;

use super::{ActionAbility, EntityId, Equipment, PassiveAbility, Position, StatusEffects, Tick};
use crate::config::GameConfig;
use crate::stats::{
    ActorBonuses, CoreStats, ResourceCurrent, StatsSnapshot, compute_actor_bonuses,
};

/// Aggregate state for every entity in the map.
#[derive(Clone, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct EntitiesState {
    /// All actors (including player). Player is typically at index 0 with EntityId::PLAYER.
    /// Minimum size is 1 to guarantee player exists.
    pub actors: BoundedVec<ActorState, 1, { GameConfig::MAX_ACTORS }>,
    pub props: BoundedVec<PropState, 0, { GameConfig::MAX_PROPS }>,
    pub items: BoundedVec<ItemState, 0, { GameConfig::MAX_WORLD_ITEMS }>,
}

impl EntitiesState {
    pub fn new(
        actors: BoundedVec<ActorState, 1, { GameConfig::MAX_ACTORS }>,
        props: BoundedVec<PropState, 0, { GameConfig::MAX_PROPS }>,
        items: BoundedVec<ItemState, 0, { GameConfig::MAX_WORLD_ITEMS }>,
    ) -> Self {
        Self {
            actors,
            props,
            items,
        }
    }

    /// Creates an empty EntitiesState with no actors (minimum constraint temporarily violated).
    ///
    /// # Safety
    ///
    /// This violates the MIN=1 constraint for actors. The caller MUST add at least one actor
    /// (typically the player) before using this state in gameplay logic.
    ///
    /// Use this only for scenario initialization where entities will be added immediately.
    pub fn empty() -> Self {
        Self {
            actors: unsafe { BoundedVec::from_vec_unchecked(vec![]) },
            props: BoundedVec::new(),
            items: BoundedVec::new(),
        }
    }

    /// Returns a reference to an actor by ID.
    pub fn actor(&self, id: EntityId) -> Option<&ActorState> {
        self.actors.iter().find(|a| a.id == id)
    }

    /// Returns a mutable reference to an actor by ID.
    pub fn actor_mut(&mut self, id: EntityId) -> Option<&mut ActorState> {
        self.actors.iter_mut().find(|a| a.id == id)
    }

    /// Returns a reference to the player actor.
    ///
    /// # Panics
    ///
    /// Panics if no actor with EntityId::PLAYER exists (should never happen if invariants are maintained).
    pub fn player(&self) -> &ActorState {
        self.actor(EntityId::PLAYER)
            .expect("Player must exist in EntitiesState")
    }

    /// Returns a mutable reference to the player actor.
    ///
    /// # Panics
    ///
    /// Panics if no actor with EntityId::PLAYER exists (should never happen if invariants are maintained).
    pub fn player_mut(&mut self) -> &mut ActorState {
        self.actor_mut(EntityId::PLAYER)
            .expect("Player must exist in EntitiesState")
    }

    /// Returns an iterator over all actors.
    pub fn all_actors(&self) -> impl Iterator<Item = &ActorState> {
        self.actors.iter()
    }

    /// Returns a mutable iterator over all actors.
    pub fn all_actors_mut(&mut self) -> impl Iterator<Item = &mut ActorState> {
        self.actors.iter_mut()
    }
}

impl Default for EntitiesState {
    fn default() -> Self {
        // Create default player actor
        let player = ActorState::new(
            EntityId::PLAYER,
            Position::default(),
            CoreStats::default(),
            InventoryState::default(),
        );

        // SAFETY: We're creating a Vec with exactly 1 element, which satisfies MIN=1 constraint
        let actors = unsafe { BoundedVec::from_vec_unchecked(vec![player]) };

        Self {
            actors,
            props: BoundedVec::new(),
            items: BoundedVec::new(),
        }
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
/// - `bonuses` must always reflect current `equipment`, `status_effects`, and `abilities`
/// - Update `bonuses` whenever any of these change
/// - Use helper methods (`equip_weapon`, `add_status`, etc.) to maintain invariants
#[derive(Clone, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct ActorState {
    pub id: EntityId,
    pub position: Position,

    // === SSOT: Core Stats ===
    pub core_stats: CoreStats,
    pub resources: ResourceCurrent,

    // === State affecting bonuses ===
    /// Equipped items (weapons, armor).
    pub equipment: Equipment,

    /// Active status effects (buffs, debuffs, crowd control).
    pub status_effects: StatusEffects,

    // === Abilities ===
    /// Active abilities that can be used (Move, Attack, Fireball, etc.).
    pub actions: ArrayVec<ActionAbility, { GameConfig::MAX_ACTIONS }>,

    /// Passive abilities that provide automatic benefits (Flight, Regeneration, etc.).
    pub passives: ArrayVec<PassiveAbility, { GameConfig::MAX_PASSIVES }>,

    // === Cached: Derived from equipment + status_effects + actions + passives ===
    /// Cached bonuses from equipment, status effects, and abilities.
    ///
    /// Must be recomputed whenever `equipment`, `status_effects`, `actions`, or `passives` change.
    pub bonuses: ActorBonuses,

    // === Inventory (independent) ===
    pub inventory: InventoryState,

    // === Scheduling ===
    /// When this actor is scheduled to act next. None means not currently scheduled.
    pub ready_at: Option<Tick>,
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
        let equipment = Equipment::empty();
        let status_effects = StatusEffects::empty();
        let actions = ArrayVec::new();
        let passives = ArrayVec::new();
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
            equipment,
            status_effects,
            actions,
            passives,
            bonuses,
            inventory,
            ready_at: None,
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

    // ========================================================================
    // Action Ability Helpers
    // ========================================================================

    /// Checks if this actor has a specific action ability (regardless of enabled state).
    pub fn has_action(&self, kind: super::ActionKind) -> bool {
        self.actions.iter().any(|a| a.kind == kind)
    }

    /// Checks if this actor can use a specific action ability right now.
    ///
    /// Returns true if the action exists, is enabled, and not on cooldown.
    pub fn can_use_action(&self, kind: super::ActionKind, current_tick: Tick) -> bool {
        self.actions
            .iter()
            .any(|a| a.kind == kind && a.is_ready(current_tick))
    }

    /// Sets the cooldown for a specific action ability.
    pub fn set_action_cooldown(&mut self, kind: super::ActionKind, until: Tick) {
        if let Some(action) = self.actions.iter_mut().find(|a| a.kind == kind) {
            action.cooldown_until = until;
        }
    }

    /// Enables or disables a specific action ability.
    pub fn set_action_enabled(&mut self, kind: super::ActionKind, enabled: bool) {
        if let Some(action) = self.actions.iter_mut().find(|a| a.kind == kind) {
            action.enabled = enabled;
        }
    }

    // ========================================================================
    // Passive Ability Helpers
    // ========================================================================

    /// Checks if this actor has a specific passive ability that is enabled.
    pub fn has_passive(&self, kind: super::PassiveKind) -> bool {
        self.passives.iter().any(|p| p.kind == kind && p.enabled)
    }

    /// Enables or disables a specific passive ability.
    pub fn set_passive_enabled(&mut self, kind: super::PassiveKind, enabled: bool) {
        if let Some(passive) = self.passives.iter_mut().find(|p| p.kind == kind) {
            passive.enabled = enabled;
        }
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

/// Inventory slot containing an item and its quantity.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct InventorySlot {
    pub handle: ItemHandle,
    pub quantity: u16,
}

impl InventorySlot {
    pub fn new(handle: ItemHandle, quantity: u16) -> Self {
        Self { handle, quantity }
    }
}

/// Simplified inventory snapshot; expand as item systems mature.
#[derive(Clone, Debug, PartialEq, Eq, Default)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct InventoryState {
    pub items: ArrayVec<InventorySlot, { GameConfig::MAX_INVENTORY_SLOTS }>,
}

impl InventoryState {
    pub fn new(items: ArrayVec<InventorySlot, { GameConfig::MAX_INVENTORY_SLOTS }>) -> Self {
        Self { items }
    }

    pub fn empty() -> Self {
        Self {
            items: ArrayVec::new(),
        }
    }
}

/// Non-actor entities such as doors, switches, or hazards.
#[derive(Clone, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
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
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum PropKind {
    Door,
    Switch,
    Hazard,
    Other,
}

/// Items that exist on the ground (not inside inventories).
#[derive(Clone, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct ItemState {
    pub id: EntityId,
    pub position: Position,
    pub handle: ItemHandle,
    pub quantity: u16,
}

impl ItemState {
    pub fn new(id: EntityId, position: Position, handle: ItemHandle, quantity: u16) -> Self {
        Self {
            id,
            position,
            handle,
            quantity,
        }
    }
}

/// Reference to an item definition stored outside the core (lookup via Env).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct ItemHandle(pub u32);
