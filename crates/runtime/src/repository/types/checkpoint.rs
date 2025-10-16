//! Checkpoint data structures and references.
//!
//! Provides lightweight index structures that reference full game states
//! stored separately. Enables crash recovery, proof resumption, and game save/load.

use std::collections::HashMap;

use serde::{Deserialize, Serialize};

use crate::events::Topic;

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
    pub session_id: String,

    /// Unix timestamp when this checkpoint was created
    pub timestamp: u64,

    /// Optional human-readable label ("Boss defeated", "Before dungeon")
    pub label: Option<String>,

    /// Current state index - canonical index for all references
    /// State[nonce], Proof[nonce], and events up to Action[nonce-1]
    pub nonce: u64,

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
    pub state_hash: String,

    /// Whether full GameState is saved to disk (state_{nonce}.json exists)
    pub is_persisted: bool,
}

/// Reference to event logs with byte offsets.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EventReference {
    /// Byte offsets per topic (next event to read from each log)
    /// These point to the next unprocessed event after Action[nonce-1]
    #[serde(default)]
    pub topic_offsets: HashMap<Topic, u64>,

    /// Total number of actions executed to reach State[nonce]
    /// This should equal nonce (Action[0..nonce-1] have been processed)
    pub action_count: u64,
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
                topic_offsets: HashMap::new(),
                action_count: 0,
            },
            proof_ref: None,
        }
    }

    /// Create a checkpoint with state and event references.
    pub fn with_state(
        session_id: String,
        nonce: u64,
        state_hash: String,
        is_persisted: bool,
        action_count: u64,
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
                topic_offsets: HashMap::new(),
                action_count,
            },
            proof_ref: None,
        }
    }

    /// Get the byte offset for resuming a specific topic's log (0 if not set).
    pub fn topic_offset(&self, topic: Topic) -> u64 {
        self.event_ref
            .topic_offsets
            .get(&topic)
            .copied()
            .unwrap_or(0)
    }

    /// Update byte offset for a specific topic's log.
    pub fn set_topic_offset(&mut self, topic: Topic, offset: u64) {
        self.event_ref.topic_offsets.insert(topic, offset);
    }

    /// Returns the current state index (State[n]).
    pub fn state_index(&self) -> u64 {
        self.nonce
    }

    /// Returns the nonce for the next action to process (Action[n+1]).
    ///
    /// The next action will transition State[n] → State[n+1].
    pub fn next_action_nonce(&self) -> u64 {
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
    pub fn new(state_hash: String, is_persisted: bool) -> Self {
        Self {
            state_hash,
            is_persisted,
        }
    }
}

impl EventReference {
    /// Create a new event reference.
    pub fn new(action_count: u64) -> Self {
        Self {
            topic_offsets: HashMap::new(),
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
