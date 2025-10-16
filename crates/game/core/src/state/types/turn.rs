use std::collections::HashSet;

use super::{EntityId, Tick};

/// Turn state managing the timeline-based scheduling system.
/// This is the canonical state for ZK proofs - it explicitly tracks which actors are active
/// and the sequential order of all actions executed.
#[derive(Clone, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct TurnState {
    /// Current timeline clock.
    pub clock: Tick,

    /// Set of entities that are currently active (scheduled to act).
    /// This is the authoritative source for ZK proofs to verify all active actors were considered.
    pub active_actors: HashSet<EntityId>,

    /// The entity currently taking their turn.
    /// Updated by prepare_next_turn() before each action.
    pub current_actor: EntityId,

    /// Sequential action identifier that increments with every action executed.
    /// This provides a unique, monotonically increasing ID for each action,
    /// enabling precise tracking and proof generation even when multiple actions
    /// occur within the same clock tick.
    ///
    /// Used by the runtime for:
    /// - Tracking proof generation progress
    /// - Identifying actions uniquely (key in ProofIndex)
    /// - Crash recovery and checkpoint resumption
    #[cfg_attr(feature = "serde", serde(default))]
    pub action_nonce: u64,
}

impl TurnState {
    /// Creates a new turn state.
    pub fn new() -> Self {
        Self {
            clock: 0,
            active_actors: HashSet::new(),
            current_actor: EntityId::PLAYER, // Default to player
            action_nonce: 0,
        }
    }
}

impl Default for TurnState {
    fn default() -> Self {
        Self::new()
    }
}
