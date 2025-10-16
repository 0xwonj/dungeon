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

/// Events related to turn management (lightweight)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TurnEvent {
    /// Entity that will act in this turn
    pub entity: EntityId,
    /// Current game clock
    pub clock: Tick,
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
///
/// # Example
///
/// ```rust,ignore
/// // In events.log
/// let action_ref = ActionRef {
///     action_offset: 12345,  // Byte offset in actions.log
///     clock: 42,
///     actor: EntityId::PLAYER,
///     action_kind: Some(ActionKind::Move),
///     flags: ActionFlags::empty(),
/// };
///
/// // To get full data:
/// let full_entry = actions_log.read_at_offset(action_ref.action_offset)?;
/// ```
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActionRef {
    /// Byte offset in actions.log where the full ActionLogEntry is stored
    pub action_offset: u64,

    /// Game clock (turn number) when this action was executed
    pub clock: Tick,

    /// Entity that executed this action
    pub actor: EntityId,

    /// Type of action (optional, for quick filtering without loading full data)
    pub action_kind: Option<ActionKind>,

    /// Status flags for this action (proven, submitted, etc.)
    ///
    /// This allows quick status checks without loading the full ActionLogEntry.
    /// Updated by PersistenceWorker as proof generation and submission progresses.
    pub flags: ActionFlags,
}

/// Status flags for action tracking.
///
/// These flags are stored in ActionRef to enable quick status queries
/// without loading full action data from actions.log.
///
/// # Future Extensions
///
/// When proof submission is implemented, additional flags can be added:
/// - `SUBMITTED`: Proof submitted to blockchain
/// - `CONFIRMED`: Proof confirmed on-chain
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct ActionFlags(u8);

impl ActionFlags {
    /// No flags set
    pub const NONE: u8 = 0b0000_0000;

    /// Proof generation has been started for this action
    pub const PROOF_STARTED: u8 = 0b0000_0001;

    /// Proof has been successfully generated for this action
    pub const PROOF_GENERATED: u8 = 0b0000_0010;

    /// Proof generation failed for this action
    pub const PROOF_FAILED: u8 = 0b0000_0100;

    // Reserved bits for future use:
    // 0b0000_1000 - PROOF_SUBMITTED
    // 0b0001_0000 - PROOF_CONFIRMED
    // 0b0010_0000 - PROOF_VERIFIED
    // 0b0100_0000 - (reserved)
    // 0b1000_0000 - (reserved)

    /// Create empty flags
    pub const fn empty() -> Self {
        Self(Self::NONE)
    }

    /// Create flags from raw bits
    pub const fn from_bits(bits: u8) -> Self {
        Self(bits)
    }

    /// Get raw bits
    pub const fn bits(self) -> u8 {
        self.0
    }

    /// Check if a flag is set
    pub const fn contains(self, flag: u8) -> bool {
        (self.0 & flag) != 0
    }

    /// Set a flag
    pub fn insert(&mut self, flag: u8) {
        self.0 |= flag;
    }

    /// Clear a flag
    pub fn remove(&mut self, flag: u8) {
        self.0 &= !flag;
    }

    /// Check if proof has been generated
    pub const fn is_proven(self) -> bool {
        self.contains(Self::PROOF_GENERATED)
    }

    /// Check if proof generation failed
    pub const fn has_failed(self) -> bool {
        self.contains(Self::PROOF_FAILED)
    }

    /// Check if proof generation is in progress
    pub const fn is_proving(self) -> bool {
        self.contains(Self::PROOF_STARTED) && !self.contains(Self::PROOF_GENERATED)
    }
}
