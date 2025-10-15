//! Simulation worker that owns the authoritative [`game_core::GameState`].
//!
//! Receives commands from [`RuntimeHandle`], executes actions via
//! [`game_core::engine::GameEngine`], and publishes events to the EventBus.

use tokio::sync::{mpsc, oneshot};

use game_core::engine::{ExecuteError, TransitionPhase};
use game_core::{Action, ActionKind, EntityId, GameEngine, GameState, PrepareTurnAction, Tick};
use tracing::{debug, error};

use crate::api::{Result, RuntimeError};
use crate::events::{Event, EventBus, GameStateEvent, TurnEvent};
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
    event_bus: EventBus,
    hooks: HookRegistry,
}

impl SimulationWorker {
    /// Creates a new simulation worker.
    pub fn new(
        state: GameState,
        oracles: OracleManager,
        command_rx: mpsc::Receiver<Command>,
        event_bus: EventBus,
        hooks: HookRegistry,
    ) -> Self {
        Self {
            state,
            oracles,
            command_rx,
            event_bus,
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
        let prepare_action =
            Action::new(EntityId::SYSTEM, ActionKind::PrepareTurn(PrepareTurnAction));

        // Execute turn preparation through unified execute_action_impl
        let _delta = Self::execute_action_impl(
            &prepare_action,
            &mut self.state,
            &self.oracles,
            &self.event_bus,
        )
        .map_err(|e| match e {
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

        // Publish Turn event to Turn topic
        let clock = self.state.turn.clock;
        self.event_bus
            .publish(Event::Turn(TurnEvent { entity, clock }));

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
        oracles: &OracleManager,
        event_bus: &EventBus,
    ) -> std::result::Result<game_core::StateDelta, ExecuteError> {
        // Capture state before execution
        let before_state = state.clone();
        let env = oracles.as_game_env();

        // Execute action through GameEngine
        let mut engine = GameEngine::new(state);
        let delta = engine.execute(env, action)?;

        // Capture state after execution
        let after_state = state.clone();
        let clock = state.turn.clock;

        // Publish ActionExecuted event for ALL actions (player, NPC, system)
        // This ensures ProverWorker can generate proofs for every state transition
        event_bus.publish(Event::GameState(GameStateEvent::ActionExecuted {
            action: action.clone(),
            delta: Box::new(delta.clone()),
            clock,
            before_state: Box::new(before_state),
            after_state: Box::new(after_state),
        }));

        Ok(delta)
    }

    /// Handles player/NPC action with full workflow:
    /// validation → execute → hooks → commit
    fn handle_player_action(&mut self, action: Action) -> Result<()> {
        // Validate that action is from current actor
        self.validate_current_actor(&action)?;

        let clock = self.state.turn.clock;

        // Execute primary action on staging state
        let mut working_state = self.state.clone();
        let delta = match self.execute_action(&action, &mut working_state) {
            Ok(delta) => delta,
            Err(error) => {
                self.handle_execute_error(&action, error, clock);
                return Ok(());
            }
        };

        // Apply post-execution hooks
        // If a critical hook fails, abort the action
        if let Err(error) = self.apply_hooks(&delta, &mut working_state) {
            self.handle_execute_error(&action, error, clock);
            return Ok(());
        }

        // Commit the final state
        self.state = working_state;

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

    /// Applies all registered post-execution hooks to the working state.
    ///
    /// Hooks execute system actions via the unified execute_action method.
    /// Returns Ok(()) if all critical hooks succeeded, or Err if a critical hook failed.
    fn apply_hooks(
        &mut self,
        delta: &game_core::StateDelta,
        working_state: &mut GameState,
    ) -> std::result::Result<(), ExecuteError> {
        // Clone hooks to avoid borrow conflicts
        // (hooks are Arc-wrapped, so this is cheap)
        let hooks: Vec<_> = self.hooks.root_hooks().to_vec();

        // Execute all root hooks in priority order
        for hook in hooks {
            if let Err(e) = self.execute_hook_with_chaining(hook.as_ref(), delta, working_state, 0)
            {
                self.handle_hook_error(hook.as_ref(), e)?;
            }
        }

        Ok(())
    }

    /// Executes a single hook with chaining support.
    ///
    /// This is the core hook execution logic that:
    /// 1. Checks if the hook should trigger
    /// 2. Creates actions from the hook
    /// 3. Executes each action via execute_action
    /// 4. Recursively executes next hooks in the chain
    fn execute_hook_with_chaining(
        &mut self,
        hook: &dyn crate::hooks::PostExecutionHook,
        delta: &game_core::StateDelta,
        state: &mut GameState,
        depth: usize,
    ) -> std::result::Result<(), ExecuteError> {
        const MAX_DEPTH: usize = 50;
        if depth > MAX_DEPTH {
            return Err(ExecuteError::HookChainTooDeep {
                hook_name: hook.name().to_string(),
                depth,
            });
        }

        // 1. Check trigger condition
        let ctx = crate::hooks::HookContext {
            delta,
            state,
            oracles: &self.oracles,
        };

        if !hook.should_trigger(&ctx) {
            return Ok(());
        }

        // 2. Create actions from the hook
        let actions = hook.create_actions(&ctx);
        if actions.is_empty() {
            return Ok(());
        }

        // 3. Execute each action and chain to next hooks after the last one
        for (idx, action) in actions.iter().enumerate() {
            // Execute action via the unified execute_action method
            let new_delta = self.execute_action(action, state)?;

            // 4. Execute next hooks in chain only after the last action
            if idx == actions.len() - 1 {
                self.execute_next_hooks(hook, &new_delta, state, depth + 1)?;
            }
        }

        Ok(())
    }

    /// Executes the next hooks in the chain.
    fn execute_next_hooks(
        &mut self,
        hook: &dyn crate::hooks::PostExecutionHook,
        delta: &game_core::StateDelta,
        state: &mut GameState,
        depth: usize,
    ) -> std::result::Result<(), ExecuteError> {
        // Collect next hooks to avoid borrow conflicts
        let next_hooks: Vec<_> = hook
            .next_hook_names()
            .iter()
            .filter_map(|name| self.hooks.find(name).cloned())
            .collect();

        for next_hook in next_hooks {
            self.execute_hook_with_chaining(next_hook.as_ref(), delta, state, depth)?;
        }
        Ok(())
    }

    /// Handles hook execution errors based on criticality level.
    ///
    /// Returns Ok(()) for Important/Optional hooks, Err for Critical hooks.
    fn handle_hook_error(
        &self,
        hook: &dyn crate::hooks::PostExecutionHook,
        error: ExecuteError,
    ) -> std::result::Result<(), ExecuteError> {
        use crate::hooks::HookCriticality;

        let (level, message) = match hook.criticality() {
            HookCriticality::Critical => {
                error!(
                    target: "runtime::worker",
                    hook = hook.name(),
                    criticality = "critical",
                    error = ?error,
                    "Critical hook failed, aborting action"
                );
                return Err(error);
            }
            HookCriticality::Important => ("important", "Hook failed, continuing"),
            HookCriticality::Optional => ("optional", "Optional hook failed"),
        };

        match hook.criticality() {
            HookCriticality::Important => error!(
                target: "runtime::worker",
                hook = hook.name(),
                criticality = level,
                error = ?error,
                "{}", message
            ),
            HookCriticality::Optional => debug!(
                target: "runtime::worker",
                hook = hook.name(),
                criticality = level,
                error = ?error,
                "{}", message
            ),
            HookCriticality::Critical => unreachable!(),
        }

        Ok(())
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

        // Publish ActionFailed event to GameState topic
        self.event_bus
            .publish(Event::GameState(GameStateEvent::ActionFailed {
                action: action.clone(),
                phase,
                error: message,
                clock,
            }));
    }
}
