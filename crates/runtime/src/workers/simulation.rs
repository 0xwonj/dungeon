//! Simulation worker that owns the authoritative [`game_core::GameState`].
//!
//! Receives commands from [`RuntimeHandle`], executes actions via
//! [`game_core::engine::GameEngine`], and publishes [`GameEvent`] notifications.

use tokio::sync::{broadcast, mpsc, oneshot};

use game_core::engine::{ExecuteError, TransitionPhase};
use game_core::{Action, ActionKind, EntityId, GameEngine, GameState, PrepareTurnAction, Tick};
use tracing::{debug, error};

use crate::api::{GameEvent, Result, RuntimeError};
use crate::hooks::HookRegistry;
use crate::oracle::OracleManager;

/// Commands that can be sent to the simulation worker
pub enum Command {
    /// Prepare the next turn by selecting which entity acts next.
    /// Returns the entity and a clone of the game state for action decision-making.
    PrepareNextTurn {
        reply: oneshot::Sender<Result<(EntityId, GameState)>>,
    },
    /// Execute an action (turn must already be prepared).
    ExecuteAction {
        action: Action,
        reply: oneshot::Sender<Result<()>>,
    },
    /// Query the current game state (read-only).
    QueryState { reply: oneshot::Sender<GameState> },
}

/// Background task that processes gameplay commands.
pub struct SimulationWorker {
    state: GameState,
    oracles: OracleManager,
    command_rx: mpsc::Receiver<Command>,
    event_tx: broadcast::Sender<GameEvent>,
    hooks: HookRegistry,
}

impl SimulationWorker {
    /// Creates a new simulation worker.
    pub fn new(
        state: GameState,
        oracles: OracleManager,
        command_rx: mpsc::Receiver<Command>,
        event_tx: broadcast::Sender<GameEvent>,
        hooks: HookRegistry,
    ) -> Self {
        Self {
            state,
            oracles,
            command_rx,
            event_tx,
            hooks,
        }
    }

    /// Main worker loop.
    pub async fn run(mut self) {
        loop {
            tokio::select! {
                Some(cmd) = self.command_rx.recv() => {
                    self.handle_command(cmd);
                }
                else => break,
            }
        }
    }

    fn handle_command(&mut self, cmd: Command) {
        match cmd {
            Command::PrepareNextTurn { reply } => {
                let result = self.prepare_next_turn();
                let _ = reply.send(result);
            }
            Command::ExecuteAction { action, reply } => {
                let result = self.execute_action(action);
                let _ = reply.send(result);
            }
            Command::QueryState { reply } => {
                let _ = reply.send(self.state.clone());
            }
        }
    }

    fn prepare_next_turn(&mut self) -> Result<(EntityId, GameState)> {
        // Create system action for turn preparation
        let prepare_action =
            Action::new(EntityId::SYSTEM, ActionKind::PrepareTurn(PrepareTurnAction));

        let env = self.oracles.as_game_env();
        let mut engine = GameEngine::new(&mut self.state);

        // Execute turn preparation as a system action
        let _delta = engine.execute(env, &prepare_action).map_err(|e| match e {
            ExecuteError::PrepareTurn(phase_error) => match phase_error.error {
                game_core::TurnError::NoActiveEntities => RuntimeError::NoActiveEntities,
                game_core::TurnError::NotSystemActor => {
                    unreachable!("PrepareTurnAction is constructed with SYSTEM actor")
                }
            },
            _ => unreachable!("PrepareTurnAction should only return PrepareTurn error"),
        })?;

        // Get the current actor (now set by the system action)
        let entity = self.state.turn.current_actor;

        // Clone the current state for action decision-making
        let state_clone = self.state.clone();

        // Publish TurnCompleted event
        let _ = self.event_tx.send(GameEvent::TurnCompleted { entity });

        Ok((entity, state_clone))
    }

    fn execute_action(&mut self, action: Action) -> Result<()> {
        // Validate that action is from current actor
        self.validate_current_actor(&action)?;

        let clock = self.state.turn.clock;

        // Capture before state for proof generation
        let before_state = self.state.clone();

        // Execute primary action on staging state
        let (delta, mut working_state) = match self.execute_primary_action(action.clone()) {
            Ok(result) => result,
            Err(_) => {
                // Error already handled in execute_primary_action
                return Ok(());
            }
        };

        // Apply post-execution hooks
        // If a critical hook fails, abort the action
        if let Err(error) = self.apply_hooks(&delta, &mut working_state) {
            self.handle_execute_error(&action, error, clock);
            return Ok(());
        }

        // Commit and publish
        self.commit_and_publish(action, delta, clock, before_state, working_state);

        Ok(())
    }

    /// Validates that the action actor matches the current turn actor.
    fn validate_current_actor(&self, action: &Action) -> Result<()> {
        let current_actor = self.state.turn.current_actor;

        if action.actor != current_actor {
            return Err(RuntimeError::InvalidActionActor {
                expected: current_actor,
                provided: action.actor,
            });
        }

        Ok(())
    }

    /// Executes the primary player/NPC action on a staging state.
    ///
    /// Returns the delta and updated state on success, or handles errors internally.
    fn execute_primary_action(
        &self,
        action: Action,
    ) -> std::result::Result<(game_core::StateDelta, GameState), ()> {
        let env = self.oracles.as_game_env();
        let mut working_state = self.state.clone();
        let mut staging_engine = GameEngine::new(&mut working_state);
        let clock = self.state.turn.clock;

        match staging_engine.execute(env, &action) {
            Ok(delta) => Ok((delta, working_state)),
            Err(error) => {
                self.handle_execute_error(&action, error, clock);
                Err(())
            }
        }
    }

    /// Applies all registered post-execution hooks to the working state.
    ///
    /// Returns Ok(()) if all critical hooks succeeded, or Err if a critical hook failed.
    fn apply_hooks(
        &self,
        delta: &game_core::StateDelta,
        working_state: &mut GameState,
    ) -> std::result::Result<(), ExecuteError> {
        self.hooks
            .execute_hooks(delta, working_state, &self.oracles)
    }

    /// Commits the working state and publishes the action executed event.
    fn commit_and_publish(
        &mut self,
        action: Action,
        delta: game_core::StateDelta,
        clock: Tick,
        before_state: GameState,
        working_state: GameState,
    ) {
        let after_state = working_state.clone();
        self.state = working_state;

        let _ = self.event_tx.send(GameEvent::ActionExecuted {
            action,
            delta: Box::new(delta),
            clock,
            before_state: Box::new(before_state),
            after_state: Box::new(after_state),
        });
    }

    fn handle_execute_error(&self, action: &Action, error: ExecuteError, clock: Tick) {
        let (phase, message) = match &error {
            ExecuteError::Move(phase_error) => (phase_error.phase, phase_error.error.to_string()),
            ExecuteError::Attack(phase_error) => (phase_error.phase, phase_error.error.to_string()),
            ExecuteError::UseItem(phase_error) => {
                (phase_error.phase, phase_error.error.to_string())
            }
            ExecuteError::Interact(phase_error) => {
                (phase_error.phase, phase_error.error.to_string())
            }
            ExecuteError::PrepareTurn(phase_error) => {
                (phase_error.phase, phase_error.error.to_string())
            }
            ExecuteError::ActionCost(phase_error) => {
                (phase_error.phase, phase_error.error.to_string())
            }
            ExecuteError::Activation(phase_error) => {
                (phase_error.phase, phase_error.error.to_string())
            }
            ExecuteError::HookChainTooDeep { hook_name, depth } => {
                error!(
                    target: "runtime::worker",
                    hook_name = %hook_name,
                    depth = %depth,
                    "Hook chain exceeded maximum depth"
                );
                // For hook chain errors, return a dummy phase since this doesn't fit the normal pattern
                return;
            }
        };

        if phase == TransitionPhase::PreValidate {
            debug!(
                target: "runtime::worker",
                action = ?action,
                phase = phase.as_str(),
                error = %message,
                "Action rejected during pre-validate"
            );
        } else {
            error!(
                target: "runtime::worker",
                action = ?action,
                phase = phase.as_str(),
                error = %message,
                "Action execution failed"
            );
        }

        let _ = self.event_tx.send(GameEvent::ActionFailed {
            action: action.clone(),
            phase,
            error: message,
            clock,
        });
    }
}
