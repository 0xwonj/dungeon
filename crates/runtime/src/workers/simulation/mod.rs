//! Simulation worker that owns the authoritative [`game_core::GameState`].
//!
//! Receives commands from [`RuntimeHandle`], executes actions via
//! [`game_core::engine::GameEngine`], and publishes [`GameEvent`] notifications.

use tokio::sync::{broadcast, mpsc, oneshot};

use game_core::engine::{ExecuteError, TransitionPhase};
use game_core::{
    Action, ActionCostAction, ActionKind, ActivationAction, EntityId, GameEngine, GameState,
    PrepareTurnAction, Tick,
};
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
        // Create system action for turn preparation
        let prepare_action = Action::new(EntityId::SYSTEM, ActionKind::PrepareTurn(PrepareTurnAction));

        let env = self.oracles.as_game_env();
        let mut engine = GameEngine::new(&mut self.state);

        // Execute turn preparation as a system action
        let _delta = engine.execute(env, &prepare_action).map_err(|e| match e {
            ExecuteError::PrepareTurn(phase_error) => match phase_error.error {
                game_core::TurnError::NoActiveEntities => RuntimeError::NoActiveEntities,
            },
            _ => unreachable!("PrepareTurnAction should only return PrepareTurn error"),
        })?;

        // Get the current actor (now set by the system action)
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

        // Execute the player/NPC action
        let result = staging_engine.execute(env, &action);
        drop(staging_engine); // Release borrow on working_state

        match result {
            Ok(delta) => {
                // Calculate action cost for the cost system action
                let actor_stats = working_state
                    .entities
                    .actor(action.actor)
                    .expect("actor must exist after successful action")
                    .stats
                    .clone();
                let action_cost = action.cost(&actor_stats);

                // Apply action cost via system action
                let cost_action = Action::new(
                    EntityId::SYSTEM,
                    ActionKind::ActionCost(ActionCostAction::new(action.actor, action_cost)),
                );

                let env = self.oracles.as_game_env();
                let mut staging_engine = GameEngine::new(&mut working_state);
                if let Err(error) = staging_engine.execute(env, &cost_action) {
                    error!(
                        target: "runtime::worker",
                        action = ?action,
                        error = ?error,
                        "ActionCost system action failed (should never happen)"
                    );
                    // Continue anyway as this is a system invariant violation
                }
                drop(staging_engine);

                // If player moved, update entity activation
                let player_moved = action.actor == EntityId::PLAYER
                    && delta
                        .entities
                        .player
                        .as_ref()
                        .and_then(|p| p.position)
                        .is_some();

                if player_moved {
                    let player_position = working_state.entities.player.position;
                    let activation_action = Action::new(
                        EntityId::SYSTEM,
                        ActionKind::Activation(ActivationAction::new(player_position)),
                    );

                    let env = self.oracles.as_game_env();
                    let mut staging_engine = GameEngine::new(&mut working_state);
                    if let Err(error) = staging_engine.execute(env, &activation_action) {
                        error!(
                            target: "runtime::worker",
                            action = ?action,
                            error = ?error,
                            "Activation system action failed (should never happen)"
                        );
                        // Continue anyway
                    }
                }

                // Commit all staged changes (player action + system actions)
                self.state = working_state;

                // Publish ActionExecuted event with the player/NPC action delta
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
