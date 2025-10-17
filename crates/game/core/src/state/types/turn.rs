use std::collections::BTreeSet;

use super::{EntityId, Tick};

/// Turn state managing the timeline-based scheduling system.
/// This is the canonical state for ZK proofs - it explicitly tracks which actors are active
/// and the sequential order of all actions executed.
#[derive(Clone, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct TurnState {
    /// Sequential action counter (0, 1, 2, ...).
    /// Increments with every action execution, providing unique action IDs.
    pub nonce: u64,

    /// Current timeline clock.
    pub clock: Tick,

    /// Set of entities that are currently active (scheduled to act).
    /// This is the authoritative source for ZK proofs to verify all active actors were considered.
    /// Using BTreeSet instead of HashSet ensures deterministic serialization order for hash consistency.
    pub active_actors: BTreeSet<EntityId>,

    /// The entity currently taking their turn.
    /// Updated by prepare_next_turn() before each action.
    pub current_actor: EntityId,
}

impl TurnState {
    /// Creates a new turn state.
    pub fn new() -> Self {
        Self {
            nonce: 0,
            clock: 0,
            active_actors: BTreeSet::new(),
            current_actor: EntityId::PLAYER,
        }
    }
}

impl Default for TurnState {
    fn default() -> Self {
        Self::new()
    }
}
