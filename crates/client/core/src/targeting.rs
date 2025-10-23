//! Target selection for auto-targeting using view model.
//!
//! This module provides target selection logic that works purely with view models,
//! without any dependency on game-core's GameState.

use crate::view_model::{ActorStatsSnapshot, ActorView};
use game_core::Position;

/// Result of target selection with priority score.
#[derive(Clone, Debug)]
pub struct TargetCandidate {
    pub position: Position,
    pub priority: i32,
}

/// Calculate Manhattan distance between two positions.
pub fn manhattan_distance(a: Position, b: Position) -> u32 {
    ((a.x - b.x).abs() + (a.y - b.y).abs()) as u32
}

/// Select the best auto-target position from actors.
///
/// Priority order:
/// 1. Nearest hostile NPC (within threat radius)
/// 2. Any other NPC
/// 3. Player (as fallback)
pub fn select_auto_target(actors: &[ActorView], player_pos: Position) -> Option<Position> {
    let threat_radius = 5;
    let mut best: Option<TargetCandidate> = None;

    for actor in actors {
        let distance = manhattan_distance(player_pos, actor.position);

        // Calculate priority based on actor type and distance
        let priority = if actor.is_player {
            // Player: lowest priority, only as fallback
            500 - distance as i32
        } else {
            // NPC: high priority
            if distance <= threat_radius {
                // Within threat radius: very high priority
                let distance_factor = (threat_radius - distance) as i32 * 100;
                let health_factor = calculate_health_threat(&actor.stats);
                let speed_factor = actor.stats.speed as i32;

                2000 + distance_factor + health_factor + speed_factor
            } else {
                // Outside threat radius: moderate priority
                1000 - distance as i32
            }
        };

        let candidate = TargetCandidate {
            position: actor.position,
            priority,
        };

        match &best {
            None => best = Some(candidate),
            Some(current) if candidate.priority > current.priority => {
                best = Some(candidate);
            }
            _ => {}
        }
    }

    best.map(|c| c.position)
}

/// Calculate threat factor based on health (lower health = higher threat, finish them off).
fn calculate_health_threat(stats: &ActorStatsSnapshot) -> i32 {
    let health_percent = if stats.health.maximum > 0 {
        (stats.health.current * 100) / stats.health.maximum
    } else {
        100
    };

    // Lower health = higher priority (finish off wounded enemies)
    (100 - health_percent) as i32 * 10
}
