//! Events emitted during simulation for front-ends to observe.
//!
//! Consumers subscribe to [`GameEvent`] to react to state changes without
//! blocking the worker loop.
use game_core::{Action, EntityId, GameState, StateDelta, Tick, engine::TransitionPhase};

// Re-export ProofData from zk crate
pub use zk::{ProofBackend, ProofData};

/// Events emitted by the runtime during game simulation
#[derive(Debug, Clone)]
pub enum GameEvent {
    /// A turn was completed by an entity
    TurnCompleted { entity: EntityId },

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

    /// ZK proof generation started for an action
    ProofStarted { action: Action, clock: Tick },

    /// ZK proof successfully generated
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
