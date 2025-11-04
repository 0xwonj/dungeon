//! Action domain - effect-based action system.
//!
//! # New Action System (Effect-Based)
//!
//! The action system uses a data-driven, effect-based architecture:
//! - `ActionKind`: Enum of all action types
//! - `CharacterAction`: Execution instance (actor + kind + input)
//! - `ActionProfile`: Complete specification loaded from data
//! - `ActionEffect`: Atomic effects that compose into actions
//! - `TargetingMode`: How actions select and filter targets
//!
//! See docs/action-system.md for detailed design documentation.
//!
//! # Module Structure
//!
//! - `kind`: ActionKind enum (all action types)
//! - `effect`: Effect definitions (Damage, Heal, Status, Movement, etc.)
//! - `targeting`: Targeting modes and filters
//! - `profile`: Action profiles (behavior + costs + effects)
//! - `types`: Core types (CharacterAction, ActionInput, ActionResult)
//! - `execute`: Action execution pipeline (resolve targets + apply effects)
//! - `system`: System actions (PrepareTurn, ActionCost, Activation)

pub mod available;
pub mod effect;
pub mod execute;
pub mod kind;
pub mod profile;
pub mod system;
pub mod targeting;
pub mod types;

// Re-export commonly used types
pub use available::get_available_actions;
pub use effect::{
    ActionEffect, Condition, Displacement, EffectKind, ExecutionPhase, Formula, InteractionType,
};
pub use execute::{ActionError, EffectContext, apply, post_validate, pre_validate};
pub use kind::ActionKind;
pub use profile::{ActionProfile, ActionTag, Requirement, ResourceCost};
pub use system::{
    ActionCostAction, ActionCostError, ActivationAction, ActivationError, PrepareTurnAction,
    TurnError,
};
pub use targeting::TargetingMode;
pub use types::{ActionInput, ActionResult, CardinalDirection, CharacterAction};

use crate::env::GameEnv;
use crate::state::{EntityId, GameState, Tick};

/// Defines how a concrete action variant mutates game state.
///
/// System actions (PrepareTurn, ActionCost, Activation) implement this trait.
/// Character actions use the new effect-based execution in `execute.rs`.
pub trait ActionTransition {
    type Error;
    type Result;

    /// Returns the entity performing this action.
    fn actor(&self) -> EntityId;

    /// Returns the base time cost of this action in ticks (before speed scaling).
    fn cost(&self, env: &GameEnv<'_>) -> Tick;

    /// Validates pre-conditions using the state **before** mutation.
    fn pre_validate(&self, _state: &GameState, _env: &GameEnv<'_>) -> Result<(), Self::Error> {
        Ok(())
    }

    /// Applies the action by mutating the game state directly.
    fn apply(&self, state: &mut GameState, env: &GameEnv<'_>) -> Result<Self::Result, Self::Error>;

    /// Validates post-conditions using the state **after** mutation.
    fn post_validate(&self, _state: &GameState, _env: &GameEnv<'_>) -> Result<(), Self::Error> {
        Ok(())
    }
}

/// System action variants (turn management, cost application, activation).
#[derive(Clone, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum SystemActionKind {
    PrepareTurn(PrepareTurnAction),
    ActionCost(ActionCostAction),
    Activation(ActivationAction),
}

/// Top-level action enum that can be either a character action or system action.
#[derive(Clone, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum Action {
    /// Character action using the new effect-based system.
    Character(CharacterAction),

    /// System action (turn scheduling, cost application, activation).
    System { kind: SystemActionKind },
}

impl Action {
    /// Creates a new character action.
    pub fn character(action: CharacterAction) -> Self {
        Self::Character(action)
    }

    /// Creates a new system action.
    pub fn system(kind: SystemActionKind) -> Self {
        Self::System { kind }
    }

    /// Returns the entity ID performing this action.
    pub fn actor(&self) -> EntityId {
        match self {
            Action::Character(action) => action.actor,
            Action::System { .. } => EntityId::SYSTEM,
        }
    }

    /// Returns the time cost (in ticks) for this action.
    ///
    /// Cost is scaled by the actor's speed stat (from snapshot).
    /// Base costs are retrieved from TablesOracle.
    pub fn cost(&self, snapshot: &crate::stats::StatsSnapshot, env: &GameEnv<'_>) -> Tick {
        use crate::stats::calculate_action_cost;

        let base_cost = match self {
            Action::Character(action) => {
                // Get base cost from action profile
                match env.tables() {
                    Ok(tables) => tables.action_profile(action.kind).base_cost,
                    Err(_) => 100, // Default cost on error
                }
            }
            Action::System { kind } => match kind {
                SystemActionKind::PrepareTurn(action) => action.cost(env),
                SystemActionKind::ActionCost(action) => action.cost(env),
                SystemActionKind::Activation(action) => action.cost(env),
            },
        };

        calculate_action_cost(base_cost, snapshot.speed.physical)
    }

    /// Returns the snake_case string representation of the action.
    pub fn as_snake_case(&self) -> &str {
        match self {
            Action::Character(action) => {
                // TODO: Convert ActionKind to snake_case
                // For now, return a placeholder
                match action.kind {
                    ActionKind::Move => "move",
                    ActionKind::Wait => "wait",
                    ActionKind::MeleeAttack => "melee_attack",
                    ActionKind::Heal => "heal",
                    _ => "unknown",
                }
            }
            Action::System { kind } => match kind {
                SystemActionKind::PrepareTurn(_) => "prepare_turn",
                SystemActionKind::ActionCost(_) => "action_cost",
                SystemActionKind::Activation(_) => "activation",
            },
        }
    }
}

impl From<PrepareTurnAction> for SystemActionKind {
    fn from(action: PrepareTurnAction) -> Self {
        Self::PrepareTurn(action)
    }
}

impl From<ActionCostAction> for SystemActionKind {
    fn from(action: ActionCostAction) -> Self {
        Self::ActionCost(action)
    }
}

impl From<ActivationAction> for SystemActionKind {
    fn from(action: ActivationAction) -> Self {
        Self::Activation(action)
    }
}
