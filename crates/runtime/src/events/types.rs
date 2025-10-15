//! Event types for different topics.

use game_core::{Action, EntityId, GameState, StateDelta, Tick, engine::TransitionPhase};

// Re-export ProofData from zk crate
pub use zk::{ProofBackend, ProofData};

/// Events related to game state changes (actions, failures)
#[derive(Debug, Clone)]
pub enum GameStateEvent {
    /// An action was executed with resulting state changes
    ActionExecuted {
        action: Action,
        delta: Box<StateDelta>,
        clock: Tick,
        /// State before action execution (for ZK proof generation)
        before_state: Box<GameState>,
        /// State after action execution (for ZK proof generation)
        after_state: Box<GameState>,
    },

    /// An action failed during execution pipeline
    ActionFailed {
        action: Action,
        phase: TransitionPhase,
        error: String,
        clock: Tick,
    },
}

/// Events related to ZK proof generation
#[derive(Debug, Clone)]
pub enum ProofEvent {
    /// ZK proof generation started for an action
    ProofStarted { action: Action, clock: Tick },

    /// ZK proof successfully generated (already verified by zkVM)
    ProofGenerated {
        action: Action,
        clock: Tick,
        proof_data: ProofData,
        generation_time_ms: u64,
    },

    /// ZK proof generation failed
    ProofFailed {
        action: Action,
        clock: Tick,
        error: String,
    },
}

/// Events related to turn management (lightweight)
#[derive(Debug, Clone)]
pub struct TurnEvent {
    /// Entity that will act in this turn
    pub entity: EntityId,
    /// Current game clock
    pub clock: Tick,
}
