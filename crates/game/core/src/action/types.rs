//! Core action types and structures.
//!
//! This module defines the fundamental types for the action system:
//! - `CharacterAction`: The execution instance of a character action
//! - `DamageType`: Type of damage for resistances
//! - `ActionResult`: The result of action execution

use crate::action::ActionKind;
use crate::state::{EntityId, Position};

// ============================================================================
// Damage Type
// ============================================================================

/// Damage type for resistances and damage calculation.
///
/// Different damage types may have different resistance values on actors.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum DamageType {
    /// Physical damage (melee, projectiles).
    Physical,
    /// Fire damage (burns, explosions).
    Fire,
    /// Cold damage (ice, frost).
    Cold,
    /// Lightning damage (electricity, storms).
    Lightning,
    /// Poison damage (toxins, venom).
    Poison,
    /// Arcane damage (pure magic).
    Arcane,
    /// True damage (ignores all resistances).
    True,
}

// ============================================================================
// Cardinal Direction (for movement)
// ============================================================================

/// Cardinal direction for directional actions (Move, Dash, etc.).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum CardinalDirection {
    North,
    South,
    East,
    West,
    NorthEast,
    NorthWest,
    SouthEast,
    SouthWest,
}

impl CardinalDirection {
    /// Returns the offset (dx, dy) for this direction.
    ///
    /// Coordinate system: Y-axis increases upward (north), X-axis increases rightward (east).
    pub fn offset(self) -> (i32, i32) {
        match self {
            CardinalDirection::North => (0, 1),
            CardinalDirection::South => (0, -1),
            CardinalDirection::East => (1, 0),
            CardinalDirection::West => (-1, 0),
            CardinalDirection::NorthEast => (1, 1),
            CardinalDirection::NorthWest => (-1, 1),
            CardinalDirection::SouthEast => (1, -1),
            CardinalDirection::SouthWest => (-1, -1),
        }
    }

    /// Returns all 8 cardinal directions.
    pub fn all() -> [CardinalDirection; 8] {
        [
            CardinalDirection::North,
            CardinalDirection::South,
            CardinalDirection::East,
            CardinalDirection::West,
            CardinalDirection::NorthEast,
            CardinalDirection::NorthWest,
            CardinalDirection::SouthEast,
            CardinalDirection::SouthWest,
        ]
    }

    /// Returns the 4 orthogonal directions (no diagonals).
    pub fn orthogonal() -> [CardinalDirection; 4] {
        [
            CardinalDirection::North,
            CardinalDirection::South,
            CardinalDirection::East,
            CardinalDirection::West,
        ]
    }
}

// ============================================================================
// Action Input
// ============================================================================

/// User/AI input for an action.
///
/// This represents the input provided when creating an action:
/// - From player: Mouse clicks, keyboard direction input, position selection
/// - From AI: Computed target selection, tactical direction choice
///
/// The input is validated against the action's `TargetingMode` and then
/// used during effect execution (e.g., effects read direction for movement).
#[derive(Clone, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum ActionInput {
    /// No input required (self only).
    None,

    /// Target a specific entity.
    Entity(EntityId),

    /// Target a specific position.
    Position(Position),

    /// Target a cardinal direction.
    Direction(CardinalDirection),

    /// Target multiple entities.
    Entities(Vec<EntityId>),
}

// ============================================================================
// Character Action
// ============================================================================

/// A concrete action instance ready for execution.
///
/// This represents a specific action being performed by a specific actor
/// with specific input. The action system uses this as the fundamental
/// unit of execution.
///
/// # Structure
/// - `actor`: Who is performing the action
/// - `kind`: What action is being performed (e.g., MeleeAttack, Move)
/// - `input`: User/AI provided input (e.g., target entity, direction)
#[derive(Clone, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct CharacterAction {
    /// The entity performing this action.
    pub actor: EntityId,

    /// The type of action being performed.
    pub kind: ActionKind,

    /// User/AI input for this action.
    pub input: ActionInput,
}

impl CharacterAction {
    /// Creates a new action.
    pub fn new(actor: EntityId, kind: ActionKind, input: ActionInput) -> Self {
        Self { actor, kind, input }
    }
}

// ============================================================================
// Action Result
// ============================================================================

/// Result of an individual effect application.
///
/// Each effect applied to a target produces one `EffectResult`.
/// This provides granular tracking of what actually happened.
#[derive(Clone, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct EffectResult {
    /// The target entity this effect was applied to.
    pub target: EntityId,

    /// The actual value that was applied.
    pub applied_value: AppliedValue,

    /// Whether the effect succeeded.
    pub success: bool,

    /// Additional flags (critical, resisted, etc.).
    pub flags: EffectFlags,
}

/// The actual value applied by an effect.
///
/// This captures both the planned value and what actually happened,
/// allowing for resistance, overkill, blocking, etc.
#[derive(Clone, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum AppliedValue {
    /// Damage was dealt.
    Damage {
        /// Planned damage amount.
        planned: u32,
        /// Actual damage dealt (after resistance, etc.).
        actual: u32,
    },

    /// Healing was applied.
    Healing {
        /// Planned healing amount.
        planned: u32,
        /// Actual healing done (capped at max HP).
        actual: u32,
    },

    /// Resource was changed (MP, Lucidity, etc.).
    ResourceChange {
        /// Which resource was affected.
        resource: crate::stats::ResourceKind,
        /// Actual change (positive = restore, negative = drain).
        delta: i32,
    },

    /// Entity moved.
    Movement {
        /// Starting position.
        from: Position,
        /// Ending position.
        to: Position,
    },

    /// Status effect applied.
    StatusApplied {
        /// Which status was applied.
        status: crate::state::StatusEffectKind,
        /// Duration in ticks.
        duration: crate::state::Tick,
    },

    /// Status effect removed.
    StatusRemoved {
        /// Which status was removed.
        status: crate::state::StatusEffectKind,
    },

    /// Entity was summoned.
    Summon {
        /// The newly created entity ID.
        entity_id: EntityId,
    },

    /// No value (for effects like Wait, or failed effects).
    None,
}

/// Flags for effect results.
///
/// These capture boolean states that modify how the effect is interpreted.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct EffectFlags {
    /// Whether this was a critical hit.
    pub critical: bool,

    /// Whether the effect was resisted.
    pub resisted: bool,

    /// Whether the effect was blocked.
    pub blocked: bool,

    /// Whether healing exceeded max (overheal).
    pub overheal: bool,
}

/// Result of action execution.
///
/// This contains the complete list of all effects applied, plus a summary
/// for quick access to totals.
#[derive(Clone, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct ActionResult {
    /// Individual effect results (in execution order).
    pub effects: Vec<EffectResult>,

    /// Summary of overall action results (for UI/logging).
    pub summary: ActionSummary,
}

/// Summary of action execution results.
///
/// This aggregates all effect results for easy access to totals.
/// Useful for UI, logging, and quick checks without iterating effects.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct ActionSummary {
    /// Whether the overall action succeeded.
    pub success: bool,

    /// Total damage dealt across all effects.
    pub total_damage: u32,

    /// Total healing done across all effects.
    pub total_healing: u32,

    /// All entities affected by any effect.
    pub affected_entities: Vec<EntityId>,

    /// Whether any effect was a critical hit.
    pub any_critical: bool,

    /// Optional message for the entire action.
    pub message: Option<String>,
}

impl EffectResult {
    /// Creates a new effect result.
    pub fn new(target: EntityId, applied_value: AppliedValue) -> Self {
        Self {
            target,
            applied_value,
            success: true,
            flags: EffectFlags::default(),
        }
    }

    /// Marks this effect as a critical hit.
    pub fn with_critical(mut self) -> Self {
        self.flags.critical = true;
        self
    }

    /// Marks this effect as resisted.
    pub fn with_resisted(mut self) -> Self {
        self.flags.resisted = true;
        self
    }

    /// Marks this effect as failed.
    pub fn with_failure(mut self) -> Self {
        self.success = false;
        self
    }
}

impl ActionResult {
    /// Creates a new empty action result.
    pub fn new() -> Self {
        Self {
            effects: Vec::new(),
            summary: ActionSummary::default(),
        }
    }

    /// Creates an action result with pre-computed summary.
    pub fn with_summary(effects: Vec<EffectResult>, summary: ActionSummary) -> Self {
        Self { effects, summary }
    }

    /// Builds summary from effect results.
    ///
    /// This computes aggregated values from all individual effect results.
    pub fn build_summary(effects: &[EffectResult]) -> ActionSummary {
        let mut summary = ActionSummary {
            success: effects.iter().any(|e| e.success),
            ..Default::default()
        };

        for effect in effects {
            // Aggregate damage
            if let AppliedValue::Damage { actual, .. } = effect.applied_value {
                summary.total_damage += actual;
            }

            // Aggregate healing
            if let AppliedValue::Healing { actual, .. } = effect.applied_value {
                summary.total_healing += actual;
            }

            // Collect affected entities
            if !summary.affected_entities.contains(&effect.target) {
                summary.affected_entities.push(effect.target);
            }

            // Track critical hits
            if effect.flags.critical {
                summary.any_critical = true;
            }
        }

        summary
    }

    /// Creates a complete action result from effects.
    ///
    /// This automatically builds the summary from the provided effects.
    pub fn from_effects(effects: Vec<EffectResult>) -> Self {
        let summary = Self::build_summary(&effects);
        Self { effects, summary }
    }
}

impl Default for ActionResult {
    fn default() -> Self {
        Self::new()
    }
}
