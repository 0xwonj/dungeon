//! Next-to-act targeting strategy.
//!
//! Prioritizes enemies that will act soonest (lowest ready_at value).
//! Useful for tactical turn management and pre-emptive strikes.

use crate::services::targeting::{TargetingStrategy, utils::manhattan_distance};
use crate::view_model::ViewModel;
use game_core::Position;

/// Target the enemy that will act next (soonest ready_at).
///
/// **Behavior:**
/// - Targets NPC with lowest `ready_at` value (will act soonest)
/// - Ignores NPCs with `ready_at = None` (not currently scheduled)
/// - Optional maximum distance filter
/// - Ties broken by distance (closer first)
///
/// **Use cases:**
/// - Pre-emptive strikes (attack before they act)
/// - Tactical turn management (disrupt enemy actions)
/// - Advanced players who plan turns ahead
/// - Interrupt fast-casting enemies
#[derive(Debug, Clone, Default)]
pub struct NextToActStrategy {
    /// Maximum distance to consider targets (None = unlimited).
    pub max_distance: Option<u32>,
}

impl NextToActStrategy {
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

impl TargetingStrategy for NextToActStrategy {
    fn select_target(&self, view_model: &ViewModel) -> Option<Position> {
        let player_pos = view_model.player.position;

        view_model
            .npcs()
            .filter(|npc| {
                // Only consider scheduled NPCs (ready_at is Some)
                if npc.ready_at.is_none() {
                    return false;
                }

                // Filter by distance if max_distance is set
                self.max_distance
                    .is_none_or(|max_dist| manhattan_distance(player_pos, npc.position) <= max_dist)
            })
            .min_by_key(|npc| {
                // Primary sort: by ready_at (soonest first)
                // Secondary sort: by distance (closer first for tie-breaking)
                let ready_at = npc.ready_at.unwrap(); // Safe: filtered out None above
                let distance = manhattan_distance(player_pos, npc.position);
                (ready_at, distance)
            })
            .map(|npc| npc.position)
    }

    fn name(&self) -> &'static str {
        "Next to Act"
    }

    fn description(&self) -> &'static str {
        "Targets the enemy that will act soonest (for pre-emptive strikes)"
    }
}
