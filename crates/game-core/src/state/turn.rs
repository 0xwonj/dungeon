use std::collections::HashSet;

use super::{EntityId, Tick};

/// Turn state managing the timeline-based scheduling system.
/// This is the canonical state for ZK proofs - it explicitly tracks which actors are active.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TurnState {
    /// Current timeline clock.
    pub clock: Tick,

    /// Set of entities that are currently active (scheduled to act).
    /// This is the authoritative source for ZK proofs to verify all active actors were considered.
    pub active_actors: HashSet<EntityId>,
}

impl TurnState {
    /// Creates a new turn state.
    pub fn new() -> Self {
        Self {
            clock: Tick::ZERO,
            active_actors: HashSet::new(),
        }
    }
}

impl Default for TurnState {
    fn default() -> Self {
        Self::new()
    }
}
