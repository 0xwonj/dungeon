//! Targeting system for actions.
//!
//! Defines how actions select targets. This module contains only the minimal
//! targeting modes needed for basic gameplay:
//! - None: No target
//! - SelfOnly: Caster only
//! - SingleTarget: One entity within range
//! - Directional: Direction-based (for movement)
//!
//! ## Future Extensions
//! When needed, add:
//! - AOE targeting (circle, cone, line)
//! - Multi-target selection
//! - Chain targeting
//! - Target filters (team, type, status)

// ============================================================================
// Targeting Mode
// ============================================================================

/// How an action selects targets.
#[derive(Clone, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum TargetingMode {
    /// No target required.
    ///
    /// Used for actions that don't need a target (e.g., environmental effects).
    None,

    /// Self only.
    ///
    /// Action only targets the caster (e.g., self-buffs, meditation).
    SelfOnly,

    /// Single entity target.
    ///
    /// Action targets one entity within range.
    /// Used for attacks, heals, single-target spells.
    SingleTarget {
        /// Maximum range in tiles (Chebyshev distance).
        range: u32,

        /// Whether line of sight is required.
        /// TODO: Implement LOS checking in validation.
        requires_los: bool,
    },

    /// Direction-based targeting.
    ///
    /// Action requires a cardinal direction.
    /// Used for movement, charges, directional attacks.
    Directional {
        /// Maximum range in tiles.
        range: u32,

        /// Width of the directional area (None = single line).
        /// TODO: Implement width-based targeting.
        width: Option<u32>,
    },
}

impl TargetingMode {
    /// Returns true if this mode requires a target entity.
    pub fn requires_entity_target(&self) -> bool {
        matches!(self, TargetingMode::SingleTarget { .. })
    }

    /// Returns true if this mode requires a direction.
    pub fn requires_direction(&self) -> bool {
        matches!(self, TargetingMode::Directional { .. })
    }

    /// Returns true if this is self-only targeting.
    pub fn is_self_only(&self) -> bool {
        matches!(self, TargetingMode::SelfOnly)
    }

    /// Returns true if this requires no target.
    pub fn requires_no_target(&self) -> bool {
        matches!(self, TargetingMode::None)
    }
}

// ============================================================================
// Common Targeting Mode Constructors
// ============================================================================

impl TargetingMode {
    /// Creates a simple melee attack targeting mode (range 1, no LOS).
    pub fn melee_attack() -> Self {
        Self::SingleTarget {
            range: 1,
            requires_los: false,
        }
    }

    /// Creates a ranged attack targeting mode with LOS requirement.
    pub fn ranged_attack(range: u32) -> Self {
        Self::SingleTarget {
            range,
            requires_los: true,
        }
    }

    /// Creates a heal targeting mode (no LOS required for convenience).
    pub fn heal(range: u32) -> Self {
        Self::SingleTarget {
            range,
            requires_los: false,
        }
    }

    /// Creates a self-buff targeting mode.
    pub fn self_buff() -> Self {
        Self::SelfOnly
    }

    /// Creates a movement targeting mode (direction-based).
    pub fn movement(range: u32) -> Self {
        Self::Directional { range, width: None }
    }
}
