//! Simulation worker that owns the authoritative [`game_core::GameState`].
//!
//! Receives commands from [`RuntimeHandle`], executes actions via
//! [`game_core::engine::GameEngine`], and publishes events to the EventBus.

use tokio::sync::{mpsc, oneshot};

use game_core::engine::{ExecuteError, TransitionPhase};
use game_core::{
    Action, EntityId, GameEngine, GameState, PrepareTurnAction, SystemActionKind, Tick,
};
use tracing::{debug, error, warn};

use crate::api::{Result, RuntimeError};
use crate::events::{Event, EventBus, GameStateEvent};
use crate::handlers::HandlerCriticality;
use crate::oracle::OracleBundle;
use crate::providers::SystemActionProvider;

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
///
/// # Design Note
///
/// SimulationWorker is now a pure game logic executor - it does not own
/// providers or handle I/O. Provider orchestration is done by Runtime.
/// This follows the "functional core, imperative shell" principle.
pub struct SimulationWorker {
    state: GameState,
    oracles: OracleBundle,
    command_rx: mpsc::Receiver<Command>,
    event_bus: EventBus,
    system_provider: SystemActionProvider,
}

impl SimulationWorker {
    /// Creates a new simulation worker.
    pub fn new(
        state: GameState,
        oracles: OracleBundle,
        command_rx: mpsc::Receiver<Command>,
        event_bus: EventBus,
        system_provider: SystemActionProvider,
    ) -> Self {
        tracing::info!(
            "SimulationWorker initialized with active_actors: {:?}, total actors: {}",
            state.turn.active_actors,
            state.entities.actors.len()
        );

        Self {
            state,
            oracles,
            command_rx,
            event_bus,
            system_provider,
        }
    }

    /// Main worker loop.
    pub async fn run(mut self) {
        loop {
            tokio::select! {
                Some(cmd) = self.command_rx.recv() => {
                    self.handle_command(cmd).await;
                }
                else => break,
            }
        }
    }

    async fn handle_command(&mut self, cmd: Command) {
        match cmd {
            Command::PrepareNextTurn { reply } => {
                let result = self.handle_turn_preparation();
                if reply.send(result).is_err() {
                    debug!("PrepareNextTurn reply channel closed (caller dropped)");
                }
            }
            Command::ExecuteAction { action, reply } => {
                let result = self.handle_player_action(action);
                if reply.send(result).is_err() {
                    debug!("ExecuteAction reply channel closed (caller dropped)");
                }
            }
            Command::QueryState { reply } => {
                if reply.send(self.state.clone()).is_err() {
                    debug!("QueryState reply channel closed (caller dropped)");
                }
            }
        }
    }

    /// Handles turn preparation workflow.
    ///
    /// Executes PrepareTurn system action and publishes Turn event.
    fn handle_turn_preparation(&mut self) -> Result<(EntityId, GameState)> {
        // Create system action for turn preparation
        let prepare_action = Action::system(SystemActionKind::PrepareTurn(PrepareTurnAction));

        // Execute turn preparation through unified execute_action_impl
        let _delta = Self::execute_action_impl(
            &prepare_action,
            &mut self.state,
            &self.oracles,
            &self.event_bus,
        )
        .map_err(|e| match e {
            ExecuteError::PrepareTurn(phase_error) => match phase_error.error {
                game_core::TurnError::NoActiveEntities { .. } => RuntimeError::NoActiveEntities,
                game_core::TurnError::NotSystemActor { .. } => {
                    unreachable!("PrepareTurnAction is constructed with SYSTEM actor")
                }
            },
            _ => unreachable!("PrepareTurnAction should only return PrepareTurn error"),
        })?;

        // Get the current actor (now set by the system action)
        let entity = self.state.turn.current_actor;

        // Clone the current state for action decision-making
        let state_clone = self.state.clone();

        Ok((entity, state_clone))
    }

    /// Executes any action (player, NPC, or system) and publishes ActionExecuted event.
    ///
    /// This is the ONLY method that should call `GameEngine::execute()`.
    /// All action executions (primary actions, hooks, turn preparation)
    /// must go through this method to ensure events are published consistently.
    ///
    /// # Arguments
    ///
    /// * `action` - The action to execute
    /// * `state` - Mutable reference to the game state to modify
    ///
    /// # Returns
    ///
    /// The state delta computed by the engine, or an error if execution failed.
    fn execute_action(
        &mut self,
        action: &Action,
        state: &mut GameState,
    ) -> std::result::Result<game_core::StateDelta, ExecuteError> {
        Self::execute_action_impl(action, state, &self.oracles, &self.event_bus)
    }

    /// Core action execution logic that can be used without mutable self reference.
    ///
    /// This static implementation allows hooks to execute actions without borrowing conflicts.
    fn execute_action_impl(
        action: &Action,
        state: &mut GameState,
        oracles: &OracleBundle,
        event_bus: &EventBus,
    ) -> std::result::Result<game_core::StateDelta, ExecuteError> {
        // Capture state before execution
        let before_state = state.clone();
        let nonce = before_state.turn.nonce; // The nonce for this action
        let clock = before_state.turn.clock;
        let env = oracles.as_game_env();

        // Execute action through GameEngine (this will increment nonce)
        let mut engine = GameEngine::new(state);
        let outcome = engine.execute(env, action)?;

        // Capture state after execution
        let after_state = state.clone();

        // Destructure outcome to avoid cloning delta
        let delta = outcome.delta;
        let action_result = outcome.action_result.unwrap_or_default();

        // Publish ActionExecuted event for ALL actions (player, NPC, system)
        // This ensures ProverWorker can generate proofs for every state transition
        event_bus.publish(Event::GameState(GameStateEvent::ActionExecuted {
            nonce,
            action: action.clone(),
            delta: Box::new(delta.clone()),
            clock,
            before_state: Box::new(before_state),
            after_state: Box::new(after_state),
            action_result,
        }));

        Ok(delta)
    }

    /// Handles player/NPC action with full workflow:
    /// execute → cascading system actions
    ///
    /// Actor validation is performed by GameEngine::execute (game-core).
    ///
    /// If the action fails due to ActorDead, just skip the turn without fallback.
    fn handle_player_action(&mut self, action: Action) -> Result<()> {
        let clock = self.state.turn.clock;

        // Capture state before action
        let state_before = self.state.clone();

        // Execute primary action
        // We need to clone state temporarily to satisfy borrow checker
        let mut working_state = self.state.clone();
        let delta = match self.execute_action(&action, &mut working_state) {
            Ok(delta) => {
                // Commit working state
                self.state = working_state;
                delta
            }
            Err(error) => {
                // Check if actor is dead
                if matches!(
                    error,
                    ExecuteError::Character(ref e) if matches!(e.error, game_core::ActionError::ActorDead)
                ) {
                    debug!(
                        target: "runtime::worker",
                        actor = ?action.actor(),
                        "Dead actor attempted action, skipping turn"
                    );
                    return Ok(());
                }

                // For other errors, try Wait fallback
                debug!(
                    target: "runtime::worker",
                    actor = ?action.actor(),
                    error = %error.message(),
                    "Action failed, attempting Wait fallback"
                );

                self.handle_execute_error(&action, error, clock);

                // Try Wait action
                match self.execute_wait_fallback(action.actor(), &mut working_state) {
                    Ok(delta) => {
                        // Commit working state
                        self.state = working_state;
                        delta
                    }
                    Err(_) => {
                        // Wait also failed (probably dead actor), just skip
                        return Ok(());
                    }
                }
            }
        };

        // Process cascading system actions
        if let Err(error) = self.process_cascading(delta, state_before) {
            error!(target: "runtime::worker", error = ?error, "Cascading system actions failed");
            return Ok(());
        }

        Ok(())
    }

    /// Process cascading system actions via SystemActionProvider.
    ///
    /// This is the core of the reactive system action generation:
    /// 1. Provider analyzes delta and generates system actions
    /// 2. Execute each system action individually
    /// 3. Each action may generate new deltas that trigger more system actions (cascading)
    /// 4. Repeat until no new actions are generated
    fn process_cascading(
        &mut self,
        initial_delta: game_core::StateDelta,
        initial_state_before: GameState,
    ) -> std::result::Result<(), ExecuteError> {
        const MAX_PASSES: usize = 10;

        // Track (delta, state_before) pairs for provider
        let mut current_deltas = vec![(initial_delta, initial_state_before)];

        for pass in 0..MAX_PASSES {
            let mut next_deltas = vec![];

            // Process all deltas from this pass
            for (delta, state_before) in current_deltas {
                // Provider generates system actions from delta
                let reactive_actions = self.system_provider.generate_actions(
                    &delta,
                    &state_before,
                    &self.state,
                    &self.oracles,
                );

                tracing::debug!(
                    target: "runtime::worker",
                    pass = pass,
                    action_count = reactive_actions.len(),
                    delta_empty = delta.is_empty(),
                    action = ?delta.action.as_snake_case(),
                    "Cascading: generated {} system actions",
                    reactive_actions.len()
                );

                // Execute each action individually
                for (action, handler_name, criticality) in reactive_actions {
                    // Capture state before this action
                    let action_state_before = self.state.clone();

                    // Execute action
                    match Self::execute_action_impl(
                        &action,
                        &mut self.state,
                        &self.oracles,
                        &self.event_bus,
                    ) {
                        Ok(action_delta) => {
                            // If action produced changes, queue for next pass
                            if !action_delta.is_empty() {
                                next_deltas.push((action_delta, action_state_before));
                            }
                        }
                        Err(e) => {
                            // Handle based on criticality
                            match criticality {
                                HandlerCriticality::Critical => {
                                    error!(
                                        target: "runtime::worker",
                                        handler = handler_name,
                                        error = ?e,
                                        "Critical system action failed - aborting cascading"
                                    );
                                    return Err(e);
                                }
                                HandlerCriticality::Important => {
                                    error!(
                                        target: "runtime::worker",
                                        handler = handler_name,
                                        error = ?e,
                                        "Important system action failed - continuing cascading"
                                    );
                                }
                                HandlerCriticality::Optional => {
                                    debug!(
                                        target: "runtime::worker",
                                        handler = handler_name,
                                        "Optional system action failed - continuing cascading"
                                    );
                                }
                            }
                        }
                    }
                }
            }

            // No more reactions, we're done
            if next_deltas.is_empty() {
                debug!(target: "runtime::worker", passes = pass + 1, "Cascading complete");
                break;
            }

            if pass == MAX_PASSES - 1 {
                warn!(
                    target: "runtime::worker",
                    "Cascading hit max passes limit (possible infinite loop)"
                );
            }

            current_deltas = next_deltas;
        }

        Ok(())
    }

    fn handle_execute_error(&self, action: &Action, error: ExecuteError, clock: Tick) {
        let (phase, message) = match &error {
            ExecuteError::Character(phase_error) => {
                (phase_error.phase, phase_error.error.to_string())
            }
            ExecuteError::PrepareTurn(phase_error) => {
                (phase_error.phase, phase_error.error.to_string())
            }
            ExecuteError::Activation(phase_error) => {
                (phase_error.phase, phase_error.error.to_string())
            }
            ExecuteError::Deactivate(phase_error) => {
                (phase_error.phase, phase_error.error.to_string())
            }
            ExecuteError::RemoveFromWorld(phase_error) => {
                (phase_error.phase, phase_error.error.to_string())
            }
            ExecuteError::HookChainTooDeep {
                hook_name, depth, ..
            } => {
                error!(
                    target: "runtime::worker",
                    hook_name = %hook_name,
                    depth = %depth,
                    "Hook chain exceeded maximum depth"
                );
                return;
            }
            ExecuteError::SystemActionNotFromSystem { actor, .. } => {
                error!(
                    target: "runtime::worker",
                    actor = ?actor,
                    "System action attempted by non-system actor"
                );
                return;
            }
            ExecuteError::ActorNotCurrent {
                actor,
                current_actor,
                ..
            } => {
                error!(
                    target: "runtime::worker",
                    actor = ?actor,
                    current_actor = ?current_actor,
                    "Action attempted by wrong actor"
                );
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

        // Publish ActionFailed event to GameState topic
        let nonce = self.state.turn.nonce;
        self.event_bus
            .publish(Event::GameState(GameStateEvent::ActionFailed {
                nonce,
                action: action.clone(),
                phase,
                error: message,
                clock,
            }));
    }

    /// Execute a Wait action as a fallback when an action fails.
    ///
    /// This ensures the turn is consumed even when the primary action fails,
    /// preventing infinite loops where the same entity keeps retrying the same
    /// invalid action.
    ///
    /// Wait should never fail because it has no effects and minimal validation.
    fn execute_wait_fallback(
        &mut self,
        actor: EntityId,
        working_state: &mut GameState,
    ) -> std::result::Result<game_core::StateDelta, ExecuteError> {
        use game_core::{ActionInput, ActionKind, CharacterAction};

        let wait_action = Action::character(CharacterAction::new(
            actor,
            ActionKind::Wait,
            ActionInput::None,
        ));

        Self::execute_action_impl(&wait_action, working_state, &self.oracles, &self.event_bus)
    }
}
