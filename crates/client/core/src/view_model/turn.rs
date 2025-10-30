//! Turn state view for presentation.

use game_core::{EntityId, GameState};

/// Turn information for display.
#[derive(Clone, Debug)]
pub struct TurnView {
    pub clock: u64,
    pub current_actor: EntityId,
    pub active_actors: Vec<EntityId>,
}

impl TurnView {
    pub fn from_state(state: &GameState) -> Self {
        let mut active: Vec<_> = state.turn.active_actors.iter().copied().collect();
        active.sort();
        Self {
            clock: state.turn.clock,
            current_actor: state.turn.current_actor,
            active_actors: active,
        }
    }

    /// Update from game state (for incremental updates).
    pub fn update_from_state(&mut self, state: &GameState) {
        self.clock = state.turn.clock;
        self.current_actor = state.turn.current_actor;
        self.active_actors = state.turn.active_actors.iter().copied().collect();
        self.active_actors.sort();
    }
}
