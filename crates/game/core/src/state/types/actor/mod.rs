//! Actor-related state types.
//!
//! This module contains all types and systems specific to actors:
//! - ActorState: Main actor structure
//! - Abilities: Active and passive abilities
//! - Equipment: Weapon and armor system
//! - Inventory: Item storage for actors
//! - Status: Status effects and conditions

pub mod abilities;
pub mod equipment;
pub mod inventory;
pub mod status;

use arrayvec::ArrayVec;

pub use abilities::{
    ActionAbilities, ActionAbility, PassiveAbilities, PassiveAbility, PassiveKind,
};
pub use equipment::{Equipment, EquipmentBuilder};
pub use inventory::{InventorySlot, InventoryState};
pub use status::{StatusEffect, StatusEffectKind, StatusEffects};

use super::{EntityId, Position, Tick};
use crate::action::ActionKind;
use crate::config::GameConfig;
use crate::provider::ProviderKind;
use crate::stats::{ActorBonuses, CoreStats, ResourceCurrent, StatsSnapshot};
use crate::traits::{Faction, Species, TraitProfile};

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
    /// Position on the map. None means the actor is not on the map
    /// (dead, in inventory, summoning, etc.)
    pub position: Option<Position>,

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

    // === Provider (for challenge verification) ===
    /// Provider kind that generates actions for this actor.
    pub provider_kind: ProviderKind,

    /// Behavioral trait profile for AI decision making.
    ///
    /// Stored in state for challenge verification
    pub trait_profile: TraitProfile,

    // === Identity (immutable/mutable attributes) ===
    /// Species - biological/existential identity.
    pub species: Species,

    /// Faction - relationship/allegiance (mutable).
    pub faction: Faction,

    // === Scheduling ===
    /// When this actor is scheduled to act next. None means not currently scheduled.
    pub ready_at: Option<Tick>,
}

impl ActorState {
    /// Create complete stats snapshot with all bonuses applied.
    ///
    /// This is the primary way external code should access actor stats.
    /// Ensures consistency between stored state and computed values.
    ///
    /// Create a complete stats snapshot
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
    pub fn has_action(&self, kind: ActionKind) -> bool {
        self.actions.iter().any(|a| a.kind == kind)
    }

    /// Checks if this actor can use a specific action ability right now.
    ///
    /// Returns true if the action exists, is enabled, and not on cooldown.
    pub fn can_use_action(&self, kind: ActionKind, current_tick: Tick) -> bool {
        self.actions
            .iter()
            .any(|a| a.kind == kind && a.is_ready(current_tick))
    }

    /// Sets the cooldown for a specific action ability.
    pub fn set_action_cooldown(&mut self, kind: ActionKind, until: Tick) {
        if let Some(action) = self.actions.iter_mut().find(|a| a.kind == kind) {
            action.cooldown_until = until;
        }
    }

    /// Enables or disables a specific action ability.
    pub fn set_action_enabled(&mut self, kind: ActionKind, enabled: bool) {
        if let Some(action) = self.actions.iter_mut().find(|a| a.kind == kind) {
            action.enabled = enabled;
        }
    }

    // ========================================================================
    // Passive Ability Helpers
    // ========================================================================

    /// Checks if this actor has a specific passive ability that is enabled.
    pub fn has_passive(&self, kind: PassiveKind) -> bool {
        self.passives.iter().any(|p| p.kind == kind && p.enabled)
    }

    /// Enables or disables a specific passive ability.
    pub fn set_passive_enabled(&mut self, kind: PassiveKind, enabled: bool) {
        if let Some(passive) = self.passives.iter_mut().find(|p| p.kind == kind) {
            passive.enabled = enabled;
        }
    }
}

// Note: ActorState no longer has Default impl because it requires TablesOracle.
// Use ActorState::new() with appropriate oracle instead.
