//! Simulation worker that owns the authoritative [`game_core::GameState`].
//!
//! Receives commands from [`RuntimeHandle`], executes actions via
//! [`game_core::engine::GameEngine`], and publishes [`GameEvent`] notifications.

use tokio::sync::{broadcast, mpsc, oneshot};

use game_core::engine::{ExecuteError, TransitionPhase};
use game_core::{Action, EntityId, GameEngine, GameState, Tick};
use tracing::{debug, error};

use crate::api::{GameEvent, Result, RuntimeError};
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
}

impl SimulationWorker {
    /// Creates a new simulation worker.
    pub fn new(
        state: GameState,
        oracles: OracleManager,
        command_rx: mpsc::Receiver<Command>,
        event_tx: broadcast::Sender<GameEvent>,
    ) -> Self {
        Self {
            state,
            oracles,
            command_rx,
            event_tx,
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
        let mut engine = GameEngine::new(&mut self.state);

        // Prepare next turn (selects entity and updates clock in game-core)
        engine.prepare_next_turn().map_err(|e| match e {
            game_core::TurnError::NoActiveEntities => RuntimeError::NoActiveEntities,
        })?;

        // Get the current actor
        let entity = engine.current_actor();

        // Clone the current state for action decision-making
        let state_clone = self.state.clone();

        // Publish TurnCompleted event
        let _ = self.event_tx.send(GameEvent::TurnCompleted { entity });

        Ok((entity, state_clone))
    }

    fn execute_action(&mut self, action: Action) -> Result<()> {
        let current_actor = {
            let engine = GameEngine::new(&mut self.state);
            engine.current_actor()
        };
        if action.actor != current_actor {
            return Err(RuntimeError::InvalidActionActor {
                expected: current_actor,
                provided: action.actor,
            });
        }

        // Execute action against a cloned state to avoid partial mutations on failure
        let env = self.oracles.as_game_env();
        let mut working_state = self.state.clone();
        let mut staging_engine = GameEngine::new(&mut working_state);

        let clock = self.state.turn.clock;

        match staging_engine.execute(env, &action) {
            Ok(delta) => {
                // Commit staged changes
                // Note: execute now handles activation updates via hooks
                self.state = working_state;

                // Publish ActionExecuted event with delta
                let _ = self.event_tx.send(GameEvent::ActionExecuted {
                    action: action.clone(),
                    delta: Box::new(delta),
                    clock,
                });
                Ok(())
            }
            Err(error) => {
                self.handle_execute_error(&action, error, clock);
                Ok(())
            }
        }
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
