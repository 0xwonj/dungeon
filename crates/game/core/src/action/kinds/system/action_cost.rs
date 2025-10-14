//! Action cost application system action.
//!
//! Updates an actor's `ready_at` timestamp based on the cost of their executed action,
//! scaled by their speed stat.

use crate::action::ActionTransition;
use crate::env::GameEnv;
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

    fn pre_validate(&self, state: &GameState, _env: &GameEnv<'_>) -> Result<(), Self::Error> {
        // Verify target actor exists
        let actor = state
            .entities
            .actor(self.target_actor)
            .ok_or(ActionCostError::ActorNotFound(self.target_actor))?;

        // Verify actor has a ready_at (is scheduled)
        if actor.ready_at.is_none() {
            return Err(ActionCostError::ActorNotScheduled(self.target_actor));
        }

        Ok(())
    }

    fn apply(&self, state: &mut GameState, _env: &GameEnv<'_>) -> Result<(), Self::Error> {
        // Get current ready_at
        let current_ready_at = {
            let actor = state
                .entities
                .actor(self.target_actor)
                .ok_or(ActionCostError::ActorNotFound(self.target_actor))?;

            actor
                .ready_at
                .ok_or(ActionCostError::ActorNotScheduled(self.target_actor))?
        };

        // Update ready_at by adding the pre-calculated cost
        if let Some(actor) = state.entities.actor_mut(self.target_actor) {
            actor.ready_at = Some(current_ready_at + self.cost.0);
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

    fn cost(&self) -> Tick {
        // System actions have no time cost
        Tick::ZERO
    }
}

/// Errors that can occur during action cost application.
#[derive(Clone, Copy, Debug, PartialEq, Eq, thiserror::Error)]
pub enum ActionCostError {
    #[error("actor {0} not found in game state")]
    ActorNotFound(EntityId),

    #[error("actor {0} is not scheduled (no ready_at timestamp)")]
    ActorNotScheduled(EntityId),
}
