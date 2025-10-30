//! World summary statistics for display.

use game_core::{EntityId, GameState};

/// Aggregate world statistics.
#[derive(Clone, Debug)]
pub struct WorldSummary {
    pub npc_count: usize,
    pub prop_count: usize,
    pub loose_item_count: usize,
}

impl WorldSummary {
    pub fn from_state(state: &GameState) -> Self {
        Self {
            npc_count: state
                .entities
                .all_actors()
                .filter(|a| a.id != EntityId::PLAYER)
                .count(),
            prop_count: state.entities.props.len(),
            loose_item_count: state.entities.items.len(),
        }
    }

    /// Update from game state (for incremental updates).
    pub fn update_from_state(&mut self, state: &GameState) {
        self.npc_count = state
            .entities
            .all_actors()
            .filter(|a| a.id != EntityId::PLAYER)
            .count();
        self.prop_count = state.entities.props.len();
        self.loose_item_count = state.entities.items.len();
    }
}
