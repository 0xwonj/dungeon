use tokio::sync::{broadcast, mpsc, oneshot};

use game_core::{Action, GameConfig, GameEngine, GameState, ScheduledTurn};

use crate::error::Result;
use crate::event::GameEvent;
use crate::oracle::OracleManager;

/// Commands that can be sent to the simulation worker
pub enum Command {
    /// Execute a player action
    ExecuteAction {
        action: Action,
        reply: oneshot::Sender<Result<()>>,
    },
    /// Advance simulation by one turn (pop next entity, decide action, execute)
    Step { reply: oneshot::Sender<Result<StepResult>> },
}

/// Result of a single step execution
#[derive(Debug, Clone)]
pub struct StepResult {
    pub scheduled: ScheduledTurn,
    pub action: Action,
}

/// Simulation worker that owns GameEngine and processes commands
pub struct SimWorker {
    state: GameState,
    config: GameConfig,
    oracles: OracleManager,
    command_rx: mpsc::Receiver<Command>,
    event_tx: broadcast::Sender<GameEvent>,
}

impl SimWorker {
    /// Creates a new simulation worker
    pub fn new(
        state: GameState,
        config: GameConfig,
        oracles: OracleManager,
        command_rx: mpsc::Receiver<Command>,
        event_tx: broadcast::Sender<GameEvent>,
    ) -> Self {
        Self {
            state,
            config,
            oracles,
            command_rx,
            event_tx,
        }
    }

    /// Main worker loop
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
            Command::ExecuteAction { action, reply } => {
                let result = self.execute_action(action);
                let _ = reply.send(result);
            }
            Command::Step { reply } => {
                let result = self.step();
                let _ = reply.send(result);
            }
        }
    }

    fn execute_action(&mut self, action: Action) -> Result<()> {
        let mut engine = GameEngine::new(&mut self.state, &self.config);
        let env = self.oracles.as_game_env();

        engine
            .execute(env, &action)
            .map_err(|e| crate::error::RuntimeError::ExecuteFailed(format!("{:?}", e)))?;

        let _ = self
            .event_tx
            .send(GameEvent::ActionExecuted { action });

        Ok(())
    }

    fn step(&mut self) -> Result<StepResult> {
        // Select next turn (off-chain logic)
        let scheduled = self.select_next_turn()?;

        // For now, we'll implement a placeholder action (Wait)
        // NPC AI will be added next
        let action = Action::new(scheduled.entity, game_core::ActionKind::Wait);

        // Execute action with game engine
        let mut engine = GameEngine::new(&mut self.state, &self.config);

        // Update clock to the scheduled tick
        engine.set_clock(scheduled.ready_at);

        let env = self.oracles.as_game_env();
        engine
            .execute(env, &action)
            .map_err(|e| crate::error::RuntimeError::ExecuteFailed(format!("{:?}", e)))?;

        // Publish events
        let _ = self.event_tx.send(GameEvent::TurnCompleted {
            entity: scheduled.entity,
        });
        let _ = self.event_tx.send(GameEvent::ActionExecuted {
            action: action.clone(),
        });

        Ok(StepResult { scheduled, action })
    }

    /// Selects the next entity to act by finding the one with the smallest ready_at tick.
    /// This is off-chain logic and does not need to be proven.
    fn select_next_turn(&self) -> Result<ScheduledTurn> {
        self.state
            .turn
            .active_actors
            .iter()
            .filter_map(|&id| {
                let actor = self.state.entities.actor(id)?;
                actor.ready_at.map(|tick| (tick, id))
            })
            .min_by_key(|(tick, _)| *tick)
            .map(|(ready_at, entity)| ScheduledTurn { entity, ready_at })
            .ok_or(crate::error::RuntimeError::NoActiveEntities)
    }

    /// Maintains the active entity set based on proximity to the player.
    /// Entities within the activation radius are activated (if not already active).
    /// Entities outside the radius are deactivated.
    pub fn maintain_active_set(&mut self) -> Result<()> {
        let player_position = self.state.entities.player.position;
        let activation_radius = self.config.activation_radius;

        // Get nearby entities from map repository via oracle
        let nearby_entities = self
            .oracles
            .map
            .repo
            .get_nearby_entities(player_position, activation_radius)
            .unwrap_or_default();

        let mut newly_active = std::collections::HashSet::new();

        // Collect active entities before borrowing engine
        let current_active: Vec<_> = self.state.turn.active_actors.iter().copied().collect();
        let current_clock = self.state.turn.clock;

        // Create engine for state mutations
        let mut engine = GameEngine::new(&mut self.state, &self.config);

        for (entity, position) in nearby_entities {
            if !is_within_activation_region(player_position, position, activation_radius) {
                continue;
            }

            newly_active.insert(entity);

            if !current_active.contains(&entity) {
                // Newly activated - initialize with current clock
                engine.activate(entity, position, current_clock);
            }
            // Position updates happen through activate() for new entities
            // For already active entities, position is tracked in GameState
        }

        // Deactivate entities outside the activation radius
        let to_deactivate: Vec<_> = current_active
            .into_iter()
            .filter(|id| !newly_active.contains(id))
            .collect();

        for entity in to_deactivate {
            engine.deactivate(entity);
        }

        Ok(())
    }
}

fn is_within_activation_region(
    player_position: game_core::Position,
    entity_position: game_core::Position,
    activation_radius: u32,
) -> bool {
    let dx = (entity_position.x - player_position.x).abs() as u32;
    let dy = (entity_position.y - player_position.y).abs() as u32;
    dx <= activation_radius && dy <= activation_radius
}
