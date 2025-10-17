//! Turn scheduling and action execution pipeline.
//!
//! The [`GameEngine`] is the authoritative reducer for [`GameState`]. It
//! orchestrates the transition phases and surfaces rich error information
//! for the runtime. All state mutations, including system actions for turn
//! scheduling and cost application, flow through the same execute() pipeline.

mod errors;
mod transition;

pub use errors::{ExecuteError, TransitionPhase, TransitionPhaseError};

use crate::action::Action;
use crate::env::GameEnv;
use crate::state::{GameState, StateDelta};

/// Game engine that manages action execution, turn scheduling, and game logic.
///
/// All state mutations flow through the three-phase action pipeline:
/// pre_validate → apply → post_validate
///
/// Both player/NPC actions and system actions (turn scheduling, cost application,
/// entity activation) use the same execution path, ensuring complete auditability
/// and proof generation for all state changes.
pub struct GameEngine<'a> {
    state: &'a mut GameState,
}

impl<'a> GameEngine<'a> {
    /// Creates a new game engine with the given state.
    pub fn new(state: &'a mut GameState) -> Self {
        Self { state }
    }

    /// Executes an action by routing it through the appropriate transition pipeline.
    ///
    /// Enforces mandatory actor validation before execution:
    /// - System actions must be from `EntityId::SYSTEM`
    /// - Non-system actions must be from `state.turn.current_actor`
    ///
    /// When `zkvm` feature is enabled, delta computation is skipped.
    pub fn execute(
        &mut self,
        env: GameEnv<'_>,
        action: &Action,
    ) -> Result<StateDelta, ExecuteError> {
        // Mandatory actor validation
        self.validate_actor(action)?;

        #[cfg(not(feature = "zkvm"))]
        let before = self.state.clone();

        // Execute the action through transition pipeline
        transition::execute_transition(action, self.state, &env)?;

        // Increment nonce after successful execution
        self.state.turn.nonce += 1;

        // Generate delta capturing all state changes
        #[cfg(not(feature = "zkvm"))]
        {
            let delta = StateDelta::from_states(action.clone(), &before, self.state);
            Ok(delta)
        }

        // In zkvm mode, skip delta computation and return empty delta
        #[cfg(feature = "zkvm")]
        Ok(StateDelta::empty())
    }

    /// Validates action actor matches turn state.
    fn validate_actor(&self, action: &Action) -> Result<(), ExecuteError> {
        match action {
            Action::System { .. } => {
                // System actions are always valid (actor is implicitly SYSTEM)
                Ok(())
            }
            Action::Character { actor, .. } => {
                let current_actor = self.state.turn.current_actor;
                if *actor != current_actor {
                    return Err(ExecuteError::ActorNotCurrent {
                        actor: *actor,
                        current_actor,
                    });
                }
                Ok(())
            }
        }
    }
}
