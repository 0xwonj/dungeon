//! Core action types and structures.
//!
//! This module defines the fundamental types for the action system:
//! - `CharacterAction`: The execution instance of a character action
//! - `ActionTargets`: The target(s) of an action
//! - `ActionResult`: The result of action execution

use crate::action::ActionKind;
use crate::state::{EntityId, Position};

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
    pub fn offset(self) -> (i32, i32) {
        match self {
            CardinalDirection::North => (0, -1),
            CardinalDirection::South => (0, 1),
            CardinalDirection::East => (1, 0),
            CardinalDirection::West => (-1, 0),
            CardinalDirection::NorthEast => (1, -1),
            CardinalDirection::NorthWest => (-1, -1),
            CardinalDirection::SouthEast => (1, -1),
            CardinalDirection::SouthWest => (-1, 1),
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

/// Result of action execution.
///
/// This contains detailed information about what happened during action
/// execution, including damage dealt, entities affected, healing done, etc.
#[derive(Clone, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct ActionResult {
    /// Whether the action succeeded.
    pub success: bool,

    /// Total damage dealt (if applicable).
    pub damage: u32,

    /// Total healing done (if applicable).
    pub healed: u32,

    /// Entities affected by this action.
    pub affected_targets: Vec<EntityId>,

    /// Whether this was a critical hit.
    pub critical: bool,

    /// Additional message or description.
    pub message: Option<String>,
}

impl ActionResult {
    /// Creates a new result with default values.
    pub fn new() -> Self {
        Self {
            success: true,
            damage: 0,
            healed: 0,
            affected_targets: Vec::new(),
            critical: false,
            message: None,
        }
    }

    // ========================================================================
    // Chainable methods (consume self, return Self)
    // ========================================================================

    /// Adds damage to this result (chainable).
    pub fn damage(mut self, amount: u32) -> Self {
        self.damage += amount;
        self
    }

    /// Adds healing to this result (chainable).
    pub fn healed(mut self, amount: u32) -> Self {
        self.healed += amount;
        self
    }

    /// Adds an affected target (chainable).
    pub fn affected_targets(mut self, target: EntityId) -> Self {
        if !self.affected_targets.contains(&target) {
            self.affected_targets.push(target);
        }
        self
    }

    /// Marks this result as a critical hit (chainable).
    pub fn critical(mut self, is_critical: bool) -> Self {
        self.critical = is_critical;
        self
    }

    /// Sets a message for this result (chainable).
    pub fn message(mut self, msg: impl Into<String>) -> Self {
        self.message = Some(msg.into());
        self
    }

    /// Sets success status (chainable).
    pub fn success(mut self, succeeded: bool) -> Self {
        self.success = succeeded;
        self
    }

    // ========================================================================
    // Mutable methods (take &mut self)
    // ========================================================================

    /// Adds damage to this result (mutable).
    pub fn add_damage(&mut self, amount: u32) {
        self.damage += amount;
    }

    /// Adds healing to this result (mutable).
    pub fn add_healed(&mut self, amount: u32) {
        self.healed += amount;
    }

    /// Adds an affected target (mutable).
    pub fn add_affected_targets(&mut self, target: EntityId) {
        if !self.affected_targets.contains(&target) {
            self.affected_targets.push(target);
        }
    }

    /// Sets critical hit status (mutable).
    pub fn set_critical(&mut self, is_critical: bool) {
        self.critical = is_critical;
    }

    /// Sets message (mutable).
    pub fn set_message(&mut self, msg: impl Into<String>) {
        self.message = Some(msg.into());
    }

    /// Sets success status (mutable).
    pub fn set_success(&mut self, succeeded: bool) {
        self.success = succeeded;
    }
}

impl Default for ActionResult {
    fn default() -> Self {
        Self::new()
    }
}
