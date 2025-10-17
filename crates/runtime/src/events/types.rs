//! Event types for different topics.

use game_core::{
    Action, ActionKind, EntityId, GameState, StateDelta, Tick, engine::TransitionPhase,
};
use serde::{Deserialize, Serialize};

// Re-export ProofData from zk crate
pub use zk::{ProofBackend, ProofData};

/// Events related to game state changes (actions, failures)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum GameStateEvent {
    /// An action was executed with resulting state changes
    ActionExecuted {
        nonce: u64,
        action: Action,
        delta: Box<StateDelta>,
        clock: Tick,
        before_state: Box<GameState>,
        after_state: Box<GameState>,
    },

    /// An action failed during execution pipeline
    ActionFailed {
        nonce: u64,
        action: Action,
        phase: TransitionPhase,
        error: String,
        clock: Tick,
    },
}

/// Events related to ZK proof generation
#[derive(Debug, Clone, Serialize, Deserialize)]
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

/// Reference to an executed action in the actions.log file.
///
/// This is a lightweight reference stored in the events.log to maintain the
/// complete event timeline without duplicating the full action data.
/// The full `ActionLogEntry` can be retrieved from actions.log using the offset.
///
/// # Purpose
///
/// - Keeps events.log small and fast to scan
/// - Maintains chronological event ordering
/// - Allows filtering events by actor without loading full action data
/// - Provides unique nonce for tracking proof generation
///
/// # Example
///
/// ```rust,ignore
/// // In events.log
/// let action_ref = ActionRef {
///     nonce: 42,  // Unique action identifier
///     action_offset: 12345,  // Byte offset in actions.log
///     clock: 10,
///     actor: EntityId::PLAYER,
///     action_kind: Some(ActionKind::Move),
/// };
///
/// // To get full data:
/// let full_entry = actions_log.read_at_offset(action_ref.action_offset)?;
/// ```
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActionRef {
    /// Unique sequential action identifier from GameState.turn.action_nonce
    ///
    /// This provides a monotonically increasing ID (0, 1, 2, ...) that uniquely
    /// identifies each action execution. Used as the key in ProofIndex for tracking
    /// which actions have been proven.
    pub nonce: u64,

    /// Byte offset in actions.log where the full ActionLogEntry is stored
    pub action_offset: u64,

    /// Game clock (turn number) when this action was executed
    pub clock: Tick,

    /// Entity that executed this action
    pub actor: EntityId,

    /// Type of action (optional, for quick filtering without loading full data)
    pub action_kind: Option<ActionKind>,
}
