//! Simple nearest-NPC targeting strategy.
//!
//! Always targets the closest NPC, ignoring health, speed, or threat level.
//! Useful for beginners or when you just want to focus on the nearest enemy.

use crate::services::targeting::{TargetingStrategy, utils::manhattan_distance};
use crate::view_model::ViewModel;
use game_core::Position;

/// Simple nearest-NPC targeting strategy.
///
/// **Behavior:**
/// - Always targets the closest NPC by Manhattan distance
/// - Ignores health status, speed, and other factors
/// - Ties are broken arbitrarily (first found)
///
/// **Use cases:**
/// - Beginner-friendly targeting (no complex decision-making)
/// - Exploration mode (just show me the nearest thing)
/// - Testing/debugging (predictable, simple behavior)
#[derive(Debug, Clone, Copy, Default)]
pub struct NearestStrategy;

impl TargetingStrategy for NearestStrategy {
    fn select_target(&self, view_model: &ViewModel) -> Option<Position> {
        let player_pos = view_model.player.position;

        view_model
            .npcs()
            .min_by_key(|npc| manhattan_distance(player_pos, npc.position))
            .map(|npc| npc.position)
    }

    fn name(&self) -> &'static str {
        "Nearest"
    }

    fn description(&self) -> &'static str {
        "Simply targets the closest enemy, ignoring all other factors"
    }
}
