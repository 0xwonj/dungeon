//! Checkpoint data structures and references.
//!
//! Provides lightweight index structures that reference full game states
//! stored separately. Enables crash recovery, proof resumption, and game save/load.

use serde::{Deserialize, Serialize};

use crate::types::{ByteOffset, Nonce, SessionId, StateHash, Timestamp};

/// Lightweight checkpoint for game state recovery.
///
/// # Purpose
///
/// Enables crash recovery and save/load by storing:
/// - Reference to persisted game state (State[nonce])
/// - Position in action log to resume from (Action[nonce+1..])
///
/// # Data Layout
///
/// ```text
/// checkpoint_{session}_nonce_{nonce}.json  ← This structure (metadata + references)
/// states/state_{nonce}.json                ← Full GameState (loaded on demand)
/// actions/actions_{session}.log            ← Action log (resume from offset)
/// proof_index_{session}.json               ← Proof tracking (see ProofIndex)
/// ```
///
/// # Recovery
///
/// To recover game state:
/// 1. Load State[nonce] from state repository
/// 2. Seek to action_log_offset in actions.log
/// 3. Replay Action[nonce+1..] to reach current state
///
/// # Proof Information
///
/// Checkpoint does NOT track proof status. For proof information, use ProofIndex:
/// - `proof_index.has_proof(nonce)` - Check if proof exists
/// - `proof_index.get_proof(nonce)` - Get proof metadata
/// - `proof_index.proven_up_to_nonce` - Highest proven nonce
///
/// # Design
///
/// The `nonce` serves as the canonical index:
/// - State[nonce]: The game state saved at this checkpoint
/// - Action[nonce+1..]: Actions to replay from action_log_offset
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Checkpoint {
    /// Session ID this checkpoint belongs to
    pub session_id: SessionId,

    /// Unix timestamp when this checkpoint was created
    pub timestamp: Timestamp,

    /// Optional human-readable label ("Boss defeated", "Before dungeon")
    pub label: Option<String>,

    /// Current state nonce (State[nonce] is saved at this checkpoint)
    pub nonce: Nonce,

    /// Reference to game state at State[nonce]
    pub state_ref: StateReference,

    /// Byte offset in actions.log to resume reading from.
    pub action_log_offset: ByteOffset,
}

/// Reference to a persisted game state.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StateReference {
    /// Hash of State[nonce] for verification
    pub state_hash: StateHash,

    /// Whether full GameState is saved to disk (state_{nonce}.json exists)
    pub is_persisted: bool,
}

impl Checkpoint {
    /// Create a new checkpoint for State[0] (initial state).
    pub fn new(session_id: String) -> Self {
        Self {
            session_id,
            timestamp: current_timestamp(),
            label: None,
            nonce: 0,
            state_ref: StateReference {
                state_hash: String::new(),
                is_persisted: false,
            },
            action_log_offset: 0,
        }
    }

    /// Create a checkpoint with state reference and action log offset.
    pub fn with_state(
        session_id: SessionId,
        nonce: Nonce,
        state_hash: StateHash,
        is_persisted: bool,
        action_log_offset: ByteOffset,
    ) -> Self {
        Self {
            session_id,
            timestamp: current_timestamp(),
            label: None,
            nonce,
            state_ref: StateReference {
                state_hash,
                is_persisted,
            },
            action_log_offset,
        }
    }

    /// Returns the current state nonce (State[n]).
    pub fn current_nonce(&self) -> Nonce {
        self.nonce
    }

    /// Returns the nonce for the next action to process (Action[n+1]).
    ///
    /// The next action will transition State[n] → State[n+1].
    pub fn next_action_nonce(&self) -> Nonce {
        self.nonce + 1
    }

    /// Get the action log byte offset for resuming actions.log.
    pub fn action_log_offset(&self) -> ByteOffset {
        self.action_log_offset
    }

    /// Set action log byte offset.
    pub fn set_action_log_offset(&mut self, offset: ByteOffset) {
        self.action_log_offset = offset;
    }

    /// Check if full game state is available.
    pub fn has_state(&self) -> bool {
        self.state_ref.is_persisted
    }

    /// Set human-readable label.
    pub fn with_label(mut self, label: String) -> Self {
        self.label = Some(label);
        self
    }
}

impl StateReference {
    /// Create a new state reference.
    pub fn new(state_hash: StateHash, is_persisted: bool) -> Self {
        Self {
            state_hash,
            is_persisted,
        }
    }
}

/// Get current unix timestamp in seconds.
fn current_timestamp() -> Timestamp {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs()
}
