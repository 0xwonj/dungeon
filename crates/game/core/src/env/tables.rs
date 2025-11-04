//! Game balance tables and oracle.
//!
//! This module defines all game balance parameters and the oracle trait
//! for accessing them. All values are centralized here to enable:
//! - Runtime balancing without recompilation
//! - On-chain governance and balance commits
//! - Version-controlled balance updates
//! - ZK-provable game rules (via TablesSnapshot)

use crate::state::Tick;

// ============================================================================
// Balance Data Structures
// ============================================================================

/// Action base costs (before speed scaling)
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct ActionCosts {
    pub attack: Tick,
    pub move_action: Tick, // "move" is a keyword
    pub wait: Tick,
    pub interact: Tick,
    pub use_item: Tick,
    pub activation: Tick,
}

/// Hit chance formula parameters
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct HitChanceParams {
    /// Base hit chance before modifiers
    pub base: i32,
    /// Minimum hit chance (floor)
    pub min: u32,
    /// Maximum hit chance (ceiling)
    pub max: u32,
}

/// Damage calculation parameters
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct DamageParams {
    /// AC reduction divisor: damage reduced by (ac / divisor)
    pub ac_divisor: u32,
    /// Critical hit multiplier
    pub crit_multiplier: u32,
    /// Minimum damage per hit
    pub minimum: u32,
}

/// Combat balance parameters
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct CombatParams {
    pub hit_chance: HitChanceParams,
    pub damage: DamageParams,
}

/// Speed system parameters
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct SpeedParams {
    /// Multiplier for cost calculation: (base * multiplier) / speed
    pub cost_multiplier: u64,
    /// Minimum allowed speed value
    pub min: i32,
    /// Maximum allowed speed value
    pub max: i32,
}

// ============================================================================
// TablesOracle Trait
// ============================================================================

/// Oracle providing game balance tables and formulas.
///
/// This oracle centralizes all game balance values to enable:
/// - Runtime balancing without recompilation
/// - On-chain governance and balance commits
/// - Version-controlled balance updates
/// - ZK-provable game rules (via TablesSnapshot)
pub trait TablesOracle: Send + Sync {
    /// Returns action base costs (before speed scaling)
    fn action_costs(&self) -> ActionCosts;

    /// Returns combat calculation parameters
    fn combat(&self) -> CombatParams;

    /// Returns speed system parameters
    fn speed(&self) -> SpeedParams;

    /// Returns the action profile for a given action kind.
    ///
    /// Action profiles define behavior, costs, targeting, and effects for each action.
    fn action_profile(&self, kind: crate::action::ActionKind) -> crate::action::ActionProfile;
}
