//! Lowest-health targeting strategy.
//!
//! Prioritizes enemies with the lowest HP percentage, optionally within a distance limit.
//! Useful for "finish them off" playstyle.

use crate::services::targeting::{
    TargetingStrategy,
    utils::{health_percentage, manhattan_distance},
};
use crate::view_model::ViewModel;
use game_core::Position;

/// Target the enemy with lowest HP percentage.
///
/// **Behavior:**
/// - Targets NPC with lowest HP% (current/max)
/// - Optional maximum distance filter
/// - Ties broken arbitrarily (first found)
///
/// **Use cases:**
/// - "Execute" playstyle (finish off wounded enemies)
/// - Maximize kill efficiency
/// - Prevent enemies from escaping/healing
#[derive(Debug, Clone, Default)]
pub struct LowestHealthStrategy {
    /// Maximum distance to consider targets (None = unlimited).
    pub max_distance: Option<u32>,
}

impl LowestHealthStrategy {
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

impl TargetingStrategy for LowestHealthStrategy {
    fn select_target(&self, view_model: &ViewModel) -> Option<Position> {
        let player_pos = view_model.player.position?;

        view_model
            .npcs()
            .filter(|npc| {
                // Filter by distance if max_distance is set
                let Some(npc_pos) = npc.position else {
                    return false;
                };
                self.max_distance
                    .is_none_or(|max_dist| manhattan_distance(player_pos, npc_pos) <= max_dist)
            })
            .min_by_key(|npc| {
                let (current, maximum) = npc.stats.hp();
                health_percentage(current, maximum)
            })
            .and_then(|npc| npc.position)
    }

    fn name(&self) -> &'static str {
        "Lowest Health"
    }

    fn description(&self) -> &'static str {
        "Targets the enemy with the lowest HP percentage (finish them off)"
    }
}
