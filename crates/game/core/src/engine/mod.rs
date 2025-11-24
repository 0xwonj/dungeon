//! Turn scheduling and action execution pipeline.
//!
//! The [`GameEngine`] is the authoritative reducer for [`GameState`]. It
//! orchestrates the transition phases and surfaces rich error information
//! for the runtime. All state mutations, including system actions for turn
//! scheduling and cost application, flow through the same execute() pipeline.

mod errors;
mod transition;

pub use errors::{ExecuteError, TransitionPhase, TransitionPhaseError};

use crate::action::{Action, ActionResult};
use crate::env::GameEnv;
use crate::state::{GameState, StateDelta};

/// Complete outcome of action execution.
///
/// Contains both state change metadata (delta) and action-specific results.
#[derive(Clone, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct ExecutionOutcome {
    /// State change metadata (which fields changed).
    pub delta: StateDelta,

    /// Action-specific execution result (combat outcome, item effects, etc.).
    /// `None` for system actions (PrepareTurn, ActionCost, Activation).
    pub action_result: Option<ActionResult>,
}

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
    /// Returns `ExecutionOutcome` containing both state delta and action result.
    /// When running inside zkVM guest (`target_os = "zkvm"`), delta computation is skipped
    /// to reduce proof generation overhead. Runtime/host always computes delta for events.
    pub fn execute(
        &mut self,
        env: GameEnv<'_>,
        action: &Action,
    ) -> Result<ExecutionOutcome, ExecuteError> {
        // Mandatory actor validation
        self.validate_actor(action)?;

        #[cfg(not(target_os = "zkvm"))]
        let before = self.state.clone();

        // Execute the action through transition pipeline and get result
        let action_result = transition::execute_transition(action, self.state, &env)?;

        // Increment nonce after successful execution
        self.state.turn.nonce += 1;

        // Generate delta capturing all state changes (host/runtime only)
        #[cfg(not(target_os = "zkvm"))]
        {
            let delta = StateDelta::from_states(action.clone(), &before, self.state);
            Ok(ExecutionOutcome {
                delta,
                action_result,
            })
        }

        // In zkVM guest, skip delta computation to reduce proof overhead
        #[cfg(target_os = "zkvm")]
        Ok(ExecutionOutcome {
            delta: StateDelta::empty(),
            action_result,
        })
    }

    /// Validates action actor matches turn state.
    fn validate_actor(&self, action: &Action) -> Result<(), ExecuteError> {
        let nonce = self.state.turn.nonce;

        match action {
            Action::System { .. } => {
                // System actions are always valid (actor is implicitly SYSTEM)
                Ok(())
            }
            Action::Character(character_action) => {
                let current_actor = self.state.turn.current_actor;
                if character_action.actor != current_actor {
                    return Err(ExecuteError::actor_not_current(
                        character_action.actor,
                        current_actor,
                        nonce,
                    ));
                }
                Ok(())
            }
        }
    }
}
