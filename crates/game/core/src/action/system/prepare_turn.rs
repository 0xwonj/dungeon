//! Turn preparation system action.
//!
//! Selects the next entity to act based on turn scheduling rules and advances
//! the game clock to that entity's scheduled time.

use crate::action::ActionTransition;
use crate::env::GameEnv;
use crate::error::{ErrorContext, ErrorSeverity, GameError};
use crate::state::{EntityId, GameState, Tick};

/// System action that prepares the next turn by selecting which entity acts next.
///
/// This action:
/// 1. Finds the active entity with the smallest `ready_at` timestamp
/// 2. Advances the game clock to that timestamp
/// 3. Sets the entity as the current actor
///
/// # Invariants
///
/// - At least one entity must be active (have a `ready_at` value)
/// - The selected entity's `ready_at` must not be before the current clock
/// - Tie-breaking uses entity ID (lower ID acts first)
#[derive(Clone, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct PrepareTurnAction;

impl ActionTransition for PrepareTurnAction {
    type Error = TurnError;
    type Result = ();

    fn actor(&self) -> EntityId {
        EntityId::SYSTEM
    }

    fn pre_validate(&self, state: &GameState, _env: &GameEnv<'_>) -> Result<(), Self::Error> {
        let nonce = state.turn.nonce;

        // Verify this action is executed by the SYSTEM actor
        if self.actor() != EntityId::SYSTEM {
            return Err(TurnError::not_system_actor(nonce));
        }

        // Verify at least one entity is active and ready
        let has_ready_entity = state
            .turn
            .active_actors
            .iter()
            .any(|&id| state.entities.actor(id).and_then(|a| a.ready_at).is_some());

        if !has_ready_entity {
            return Err(TurnError::no_active_entities(nonce));
        }

        Ok(())
    }

    fn apply(&self, state: &mut GameState, _env: &GameEnv<'_>) -> Result<(), Self::Error> {
        let nonce = state.turn.nonce;

        // Find the entity with the earliest ready_at timestamp
        // Tie-breaking: if multiple entities have the same timestamp, choose by entity ID
        let (entity, ready_at) = state
            .turn
            .active_actors
            .iter()
            .filter_map(|&id| {
                let actor = state.entities.actor(id)?;
                actor.ready_at.map(|tick| (id, tick))
            })
            .min_by_key(|&(entity_id, tick)| (tick, entity_id))
            .ok_or_else(|| TurnError::no_active_entities(nonce))?;

        // Advance clock to the scheduled time
        state.turn.clock = ready_at;

        // Set current actor
        state.turn.current_actor = entity;

        Ok(())
    }

    fn post_validate(&self, state: &GameState, _env: &GameEnv<'_>) -> Result<(), Self::Error> {
        // Verify the selected actor is actually in the active set
        debug_assert!(
            state.turn.active_actors.contains(&state.turn.current_actor),
            "current_actor must be in active_actors set"
        );

        // Verify the current actor has a ready_at timestamp
        if let Some(actor) = state.entities.actor(state.turn.current_actor) {
            debug_assert!(
                actor.ready_at.is_some(),
                "current_actor must have a ready_at timestamp"
            );

            // Verify clock matches the actor's ready_at
            if let Some(ready_at) = actor.ready_at {
                debug_assert_eq!(
                    state.turn.clock, ready_at,
                    "clock must match current_actor's ready_at"
                );
            }
        }

        Ok(())
    }

    fn cost(&self, _env: &GameEnv<'_>) -> Tick {
        // System actions have no time cost
        0
    }
}

/// Errors that can occur during turn operations.
#[derive(Clone, Debug, PartialEq, Eq, thiserror::Error)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum TurnError {
    /// System actor validation failed.
    #[error("prepare turn action must be executed by SYSTEM actor")]
    NotSystemActor {
        #[cfg_attr(feature = "serde", serde(skip))]
        context: ErrorContext,
    },

    /// No active entities available for scheduling.
    #[error("no entities are currently active")]
    NoActiveEntities {
        #[cfg_attr(feature = "serde", serde(skip))]
        context: ErrorContext,
    },
}

impl TurnError {
    /// Creates a NotSystemActor error with context.
    pub fn not_system_actor(nonce: u64) -> Self {
        Self::NotSystemActor {
            context: ErrorContext::new(nonce)
                .with_message("system action executed by non-system actor"),
        }
    }

    /// Creates a NoActiveEntities error with context.
    pub fn no_active_entities(nonce: u64) -> Self {
        Self::NoActiveEntities {
            context: ErrorContext::new(nonce).with_message("turn scheduling failed"),
        }
    }
}

impl GameError for TurnError {
    fn severity(&self) -> ErrorSeverity {
        match self {
            Self::NotSystemActor { .. } => ErrorSeverity::Validation,
            Self::NoActiveEntities { .. } => ErrorSeverity::Fatal,
        }
    }

    fn context(&self) -> Option<&ErrorContext> {
        match self {
            Self::NotSystemActor { context } => Some(context),
            Self::NoActiveEntities { context } => Some(context),
        }
    }

    fn error_code(&self) -> &'static str {
        match self {
            Self::NotSystemActor { .. } => "TURN_NOT_SYSTEM_ACTOR",
            Self::NoActiveEntities { .. } => "TURN_NO_ACTIVE_ENTITIES",
        }
    }
}
