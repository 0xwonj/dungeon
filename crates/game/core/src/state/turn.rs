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

    /// The entity currently taking their turn.
    /// Updated by prepare_next_turn() before each action.
    pub current_actor: EntityId,
}

impl TurnState {
    /// Creates a new turn state.
    pub fn new() -> Self {
        Self {
            clock: Tick::ZERO,
            active_actors: HashSet::new(),
            current_actor: EntityId::PLAYER, // Default to player
        }
    }
}

impl Default for TurnState {
    fn default() -> Self {
        Self::new()
    }
}
