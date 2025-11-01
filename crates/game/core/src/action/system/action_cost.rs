//! Action cost application system action.
//!
//! Updates an actor's `ready_at` timestamp based on the cost of their executed action,
//! scaled by their speed stat.

use crate::action::ActionTransition;
use crate::env::GameEnv;
use crate::error::{ErrorContext, ErrorSeverity, GameError};
use crate::state::{EntityId, GameState, Tick};

/// System action that applies the time cost of an executed action to an actor.
///
/// This action updates the target actor's `ready_at` timestamp by adding the
/// specified cost value. The cost should be pre-calculated and speed-scaled
/// before creating this action.
///
/// # Invariants
///
/// - Target actor must exist in the game state
/// - Target actor must have a `ready_at` timestamp (be in active set)
/// - The cost is non-negative
#[derive(Clone, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct ActionCostAction {
    /// The actor whose ready_at should be updated
    pub target_actor: EntityId,
    /// Pre-calculated, speed-scaled cost to add to ready_at
    pub cost: Tick,
}

impl ActionCostAction {
    /// Creates a new action cost application for the given actor and cost.
    pub fn new(target_actor: EntityId, cost: Tick) -> Self {
        Self { target_actor, cost }
    }
}

impl ActionTransition for ActionCostAction {
    type Error = ActionCostError;
    type Result = ();

    fn actor(&self) -> EntityId {
        EntityId::SYSTEM
    }

    fn pre_validate(&self, state: &GameState, _env: &GameEnv<'_>) -> Result<(), Self::Error> {
        let nonce = state.turn.nonce;

        // Verify this action is executed by the SYSTEM actor
        if self.actor() != EntityId::SYSTEM {
            return Err(ActionCostError::not_system_actor(nonce));
        }

        // Verify target actor exists
        let actor = state
            .entities
            .actor(self.target_actor)
            .ok_or_else(|| ActionCostError::actor_not_found(self.target_actor, nonce))?;

        // Verify actor has a ready_at (is scheduled)
        if actor.ready_at.is_none() {
            return Err(ActionCostError::actor_not_scheduled(
                self.target_actor,
                nonce,
            ));
        }

        Ok(())
    }

    fn apply(&self, state: &mut GameState, _env: &GameEnv<'_>) -> Result<(), Self::Error> {
        let nonce = state.turn.nonce;

        // Get current ready_at
        let current_ready_at = {
            let actor = state
                .entities
                .actor(self.target_actor)
                .ok_or_else(|| ActionCostError::actor_not_found(self.target_actor, nonce))?;

            actor
                .ready_at
                .ok_or_else(|| ActionCostError::actor_not_scheduled(self.target_actor, nonce))?
        };

        // Update ready_at by adding the pre-calculated cost
        if let Some(actor) = state.entities.actor_mut(self.target_actor) {
            actor.ready_at = Some(current_ready_at + self.cost);
        }

        Ok(())
    }

    fn post_validate(&self, state: &GameState, _env: &GameEnv<'_>) -> Result<(), Self::Error> {
        // Verify ready_at was actually updated (should never fail if apply succeeded)
        if let Some(actor) = state.entities.actor(self.target_actor) {
            debug_assert!(
                actor.ready_at.is_some(),
                "actor should still have ready_at after cost application"
            );
        }

        Ok(())
    }

    fn cost(&self, _env: &GameEnv<'_>) -> Tick {
        // System actions have no time cost
        0
    }
}

/// Errors that can occur during action cost application.
#[derive(Clone, Debug, PartialEq, Eq, thiserror::Error)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum ActionCostError {
    /// System actor validation failed.
    #[error("action cost action must be executed by SYSTEM actor")]
    NotSystemActor {
        #[cfg_attr(feature = "serde", serde(skip))]
        context: ErrorContext,
    },

    /// Actor not found in game state.
    #[error("actor {actor} not found in game state")]
    ActorNotFound {
        actor: EntityId,
        #[cfg_attr(feature = "serde", serde(skip))]
        context: ErrorContext,
    },

    /// Actor is not scheduled (missing ready_at timestamp).
    #[error("actor {actor} is not scheduled (no ready_at timestamp)")]
    ActorNotScheduled {
        actor: EntityId,
        #[cfg_attr(feature = "serde", serde(skip))]
        context: ErrorContext,
    },
}

impl ActionCostError {
    /// Creates a NotSystemActor error with context.
    pub fn not_system_actor(nonce: u64) -> Self {
        Self::NotSystemActor {
            context: ErrorContext::new(nonce)
                .with_message("system action executed by non-system actor"),
        }
    }

    /// Creates an ActorNotFound error with context.
    pub fn actor_not_found(actor: EntityId, nonce: u64) -> Self {
        Self::ActorNotFound {
            actor,
            context: ErrorContext::new(nonce)
                .with_actor(actor)
                .with_message("target actor not found"),
        }
    }

    /// Creates an ActorNotScheduled error with context.
    pub fn actor_not_scheduled(actor: EntityId, nonce: u64) -> Self {
        Self::ActorNotScheduled {
            actor,
            context: ErrorContext::new(nonce)
                .with_actor(actor)
                .with_message("actor has no ready_at timestamp"),
        }
    }
}

impl GameError for ActionCostError {
    fn severity(&self) -> ErrorSeverity {
        match self {
            Self::NotSystemActor { .. } => ErrorSeverity::Validation,
            Self::ActorNotFound { .. } => ErrorSeverity::Validation,
            Self::ActorNotScheduled { .. } => ErrorSeverity::Internal,
        }
    }

    fn context(&self) -> Option<&ErrorContext> {
        match self {
            Self::NotSystemActor { context } => Some(context),
            Self::ActorNotFound { context, .. } => Some(context),
            Self::ActorNotScheduled { context, .. } => Some(context),
        }
    }

    fn error_code(&self) -> &'static str {
        match self {
            Self::NotSystemActor { .. } => "ACTION_COST_NOT_SYSTEM_ACTOR",
            Self::ActorNotFound { .. } => "ACTION_COST_ACTOR_NOT_FOUND",
            Self::ActorNotScheduled { .. } => "ACTION_COST_ACTOR_NOT_SCHEDULED",
        }
    }
}
