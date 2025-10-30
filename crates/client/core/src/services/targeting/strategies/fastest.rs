//! Fastest-enemy targeting strategy.
//!
//! Prioritizes enemies with the highest speed stat, optionally within a distance limit.
//! Useful for intercepting fast-moving threats.

use crate::services::targeting::{TargetingStrategy, utils::manhattan_distance};
use crate::view_model::ViewModel;
use game_core::Position;

/// Target the fastest enemy (by speed stat).
///
/// **Behavior:**
/// - Targets NPC with highest physical speed
/// - Optional maximum distance filter
/// - Ties broken arbitrarily (first found)
///
/// **Use cases:**
/// - Intercept fast-moving enemies
/// - Prevent escapes
/// - Prioritize agile threats (rogues, scouts)
#[derive(Debug, Clone, Default)]
pub struct FastestStrategy {
    /// Maximum distance to consider targets (None = unlimited).
    pub max_distance: Option<u32>,
}

impl FastestStrategy {
    /// Create with distance limit.
    pub fn with_max_distance(max_distance: u32) -> Self {
        Self {
            max_distance: Some(max_distance),
        }
    }

    /// Create with unlimited range.
    pub fn unlimited() -> Self {
        Self::default()
    }
}

impl TargetingStrategy for FastestStrategy {
    fn select_target(&self, view_model: &ViewModel) -> Option<Position> {
        let player_pos = view_model.player.position;

        view_model
            .npcs()
            .filter(|npc| {
                // Filter by distance if max_distance is set
                self.max_distance
                    .is_none_or(|max_dist| manhattan_distance(player_pos, npc.position) <= max_dist)
            })
            .max_by_key(|npc| npc.stats.speed.physical)
            .map(|npc| npc.position)
    }

    fn name(&self) -> &'static str {
        "Fastest"
    }

    fn description(&self) -> &'static str {
        "Targets the fastest enemy (useful for intercepting agile threats)"
    }
}
