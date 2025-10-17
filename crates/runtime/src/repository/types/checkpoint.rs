//! Checkpoint data structures and references.
//!
//! Provides lightweight index structures that reference full game states
//! stored separately. Enables crash recovery, proof resumption, and game save/load.

use serde::{Deserialize, Serialize};

use crate::types::{ByteOffset, Nonce, SessionId, StateHash, Timestamp};

/// Lightweight checkpoint with references to external data.
///
/// # Data Layout
///
/// checkpoint_{session}.json     ← This structure (metadata + indices)
/// state_{nonce}.json            ← Full GameState (loaded on demand)
/// proof_{nonce}.bin             ← ZK proof data (loaded on demand)
/// events/{topic}_{session}.log  ← Event logs (already separate)
///
/// # Design
///
/// The `nonce` serves as the canonical index for all references:
/// - State[nonce]: The current game state
/// - Proof[nonce]: ZK proof for State[nonce] (if exists)
/// - Events: Actions 0..nonce-1 have been processed
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Checkpoint {
    /// Session ID this checkpoint belongs to
    pub session_id: SessionId,

    /// Unix timestamp when this checkpoint was created
    pub timestamp: Timestamp,

    /// Optional human-readable label ("Boss defeated", "Before dungeon")
    pub label: Option<String>,

    /// Current state index - canonical index for all references
    /// State[nonce], Proof[nonce], and events up to Action[nonce-1]
    pub nonce: Nonce,

    /// Reference to game state at State[nonce]
    pub state_ref: StateReference,

    /// References to event logs (actions 0..nonce-1 have been processed)
    pub event_ref: EventReference,

    /// Reference to ZK proof for State[nonce] (if exists)
    pub proof_ref: Option<ProofReference>,
}

/// Reference to a persisted game state.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StateReference {
    /// Hash of State[nonce] for verification
    pub state_hash: StateHash,

    /// Whether full GameState is saved to disk (state_{nonce}.json exists)
    pub is_persisted: bool,
}

/// Reference to event logs with byte offset.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EventReference {
    /// Byte offset in events.log (next event to read)
    /// Points to the next unprocessed event after Action[nonce-1]
    pub offset: ByteOffset,

    /// Total number of actions executed to reach State[nonce]
    /// This should equal nonce (Action[0..nonce-1] have been processed)
    pub action_count: Nonce,
}

/// Reference to a ZK proof.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProofReference {
    /// Whether proof file exists on disk (proof_{nonce}.bin exists)
    pub is_persisted: bool,

    /// Proof verification status
    pub is_verified: bool,
}

impl Checkpoint {
    /// Create a new checkpoint for State[0] (initial state).
    pub fn new(session_id: String) -> Self {
        Self {
            session_id,
            timestamp: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_secs(),
            label: None,
            nonce: 0,
            state_ref: StateReference {
                state_hash: String::new(),
                is_persisted: false,
            },
            event_ref: EventReference {
                offset: 0,
                action_count: 0,
            },
            proof_ref: None,
        }
    }

    /// Create a checkpoint with state and event references.
    pub fn with_state(
        session_id: SessionId,
        nonce: Nonce,
        state_hash: StateHash,
        is_persisted: bool,
        action_count: Nonce,
    ) -> Self {
        Self {
            session_id,
            timestamp: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_secs(),
            label: None,
            nonce,
            state_ref: StateReference {
                state_hash,
                is_persisted,
            },
            event_ref: EventReference {
                offset: 0,
                action_count,
            },
            proof_ref: None,
        }
    }

    /// Get the byte offset for resuming events.log.
    pub fn event_offset(&self) -> ByteOffset {
        self.event_ref.offset
    }

    /// Update byte offset for events.log.
    pub fn set_event_offset(&mut self, offset: ByteOffset) {
        self.event_ref.offset = offset;
    }

    /// Returns the current state index (State[n]).
    pub fn state_index(&self) -> Nonce {
        self.nonce
    }

    /// Returns the nonce for the next action to process (Action[n+1]).
    ///
    /// The next action will transition State[n] → State[n+1].
    pub fn next_action_nonce(&self) -> Nonce {
        self.nonce + 1
    }

    /// Check if this checkpoint has been verified with a ZK proof.
    pub fn is_verified(&self) -> bool {
        self.proof_ref.as_ref().is_some_and(|p| p.is_verified)
    }

    /// Check if full game state is available.
    pub fn has_full_state(&self) -> bool {
        self.state_ref.is_persisted
    }

    /// Check if ZK proof exists.
    pub fn has_proof(&self) -> bool {
        self.proof_ref.as_ref().is_some_and(|p| p.is_persisted)
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

impl EventReference {
    /// Create a new event reference.
    pub fn new(action_count: Nonce) -> Self {
        Self {
            offset: 0,
            action_count,
        }
    }
}

impl ProofReference {
    /// Create a new proof reference.
    pub fn new(is_persisted: bool, is_verified: bool) -> Self {
        Self {
            is_persisted,
            is_verified,
        }
    }
}
