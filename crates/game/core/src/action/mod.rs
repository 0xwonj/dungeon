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
//! - `error`: Action error types (ActionError, TurnError, ActionCostError, ActivationError)
//! - `formula`: Formula system for dynamic value calculation
//! - `effect`: Effect definitions (Damage, Heal, Status, Movement, etc.)
//! - `targeting`: Targeting modes and filters
//! - `profile`: Action profiles (ActionKind enum, behavior, costs, effects)
//! - `types`: Core types (CharacterAction, ActionInput, ActionResult)
//! - `execute`: Action execution pipeline (resolve targets + apply effects)
//! - `system`: System actions (PrepareTurn, ActionCost, Activation)

pub mod effect;
pub mod error;
pub mod execute;
pub mod formula;
pub mod profile;
pub mod system;
pub mod targeting;
pub mod types;

// Re-export commonly used types
pub use effect::{
    ActionEffect, Condition, Displacement, EffectKind, ExecutionPhase, InteractionType,
};
pub use error::{ActionError, ActivationError, DeactivateError, RemoveFromWorldError, TurnError};
pub use execute::{EffectContext, apply, post_validate, pre_validate};
pub use formula::Formula;
pub use profile::{ActionKind, ActionProfile, ActionTag, Requirement, ResourceCost};
pub use system::{ActivationAction, DeactivateAction, PrepareTurnAction, RemoveFromWorldAction};
pub use targeting::TargetingMode;
pub use types::{
    ActionInput, ActionResult, ActionSummary, AppliedValue, CardinalDirection, CharacterAction,
    DamageType, EffectFlags, EffectResult,
};

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

/// System action variants (turn management, activation, entity lifecycle).
///
/// Note: Action cost application is now integrated into character action execution
/// to avoid the overhead of a separate system action.
#[derive(Clone, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum SystemActionKind {
    PrepareTurn(PrepareTurnAction),
    Activation(ActivationAction),
    Deactivate(DeactivateAction),
    RemoveFromWorld(RemoveFromWorldAction),
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
    ///
    /// # Panics
    ///
    /// Panics if TablesOracle is not available. This should never happen in
    /// normal operation as TablesOracle is required for game execution.
    pub fn cost(&self, snapshot: &crate::stats::StatsSnapshot, env: &GameEnv<'_>) -> Tick {
        use crate::stats::calculate_action_cost;

        let base_cost = match self {
            Action::Character(action) => {
                // Get base cost from action profile
                env.tables()
                    .expect("TablesOracle must be available for action cost calculation")
                    .action_profile(action.kind)
                    .base_cost
            }
            Action::System { kind } => match kind {
                SystemActionKind::PrepareTurn(action) => action.cost(env),
                SystemActionKind::Activation(action) => action.cost(env),
                SystemActionKind::Deactivate(action) => action.cost(env),
                SystemActionKind::RemoveFromWorld(action) => action.cost(env),
            },
        };

        calculate_action_cost(base_cost, snapshot.speed.physical)
    }

    /// Returns the snake_case string representation of the action.
    ///
    /// Used for generating file names, logging, and serialization keys.
    pub fn as_snake_case(&self) -> &'static str {
        match self {
            Action::Character(action) => action.kind.as_snake_case(),
            Action::System { kind } => match kind {
                SystemActionKind::PrepareTurn(_) => "prepare_turn",
                SystemActionKind::Activation(_) => "activation",
                SystemActionKind::Deactivate(_) => "deactivate",
                SystemActionKind::RemoveFromWorld(_) => "remove_from_world",
            },
        }
    }
}

impl From<PrepareTurnAction> for SystemActionKind {
    fn from(action: PrepareTurnAction) -> Self {
        Self::PrepareTurn(action)
    }
}

impl From<ActivationAction> for SystemActionKind {
    fn from(action: ActivationAction) -> Self {
        Self::Activation(action)
    }
}

impl From<DeactivateAction> for SystemActionKind {
    fn from(action: DeactivateAction) -> Self {
        Self::Deactivate(action)
    }
}

impl From<RemoveFromWorldAction> for SystemActionKind {
    fn from(action: RemoveFromWorldAction) -> Self {
        Self::RemoveFromWorld(action)
    }
}

// ============================================================================
// Available Actions Query
// ============================================================================

/// Get all actions currently available to an entity.
///
/// Returns all actions from the entity's ability list that are:
/// - Enabled (`enabled = true`)
/// - Not on cooldown (`cooldown_until <= current_tick`)
///
/// Returns an empty vec if the entity doesn't exist or is not an actor.
pub fn get_available_actions(
    entity: EntityId,
    state: &GameState,
    _env: &GameEnv<'_>,
) -> Vec<ActionKind> {
    let Some(actor) = state.entities.actor(entity) else {
        return Vec::new();
    };

    let current_tick = state.turn.clock;

    actor
        .actions
        .iter()
        .filter(|ability| ability.is_ready(current_tick))
        .map(|ability| ability.kind)
        .collect()
}
