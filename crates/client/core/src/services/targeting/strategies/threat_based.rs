//! Threat-based targeting strategy (default).
//!
//! This is the original targeting logic, prioritizing enemies based on:
//! - Proximity (within threat radius)
//! - Health status (finish off wounded enemies)
//! - Speed (faster enemies get slight priority boost)

use crate::services::targeting::{
    TargetingStrategy,
    utils::{health_percentage, manhattan_distance},
};
use crate::view_model::{ViewModel, entities::ActorView};
use game_core::Position;

/// Threat-based targeting strategy.
///
/// **Priority order:**
/// 1. Nearest hostile NPC within threat radius (prioritize low-health)
/// 2. Any other NPC by distance
/// 3. None (caller defaults to player position)
///
/// **Scoring factors:**
/// - **Distance**: Closer targets get higher priority
/// - **Threat radius**: Targets within radius get significant boost (+2000)
/// - **Health**: Lower health enemies get priority (finish them off)
/// - **Speed**: Faster enemies get slight priority boost
///
/// This strategy is designed for tactical combat where you want to:
/// - Focus on nearby threats
/// - Finish off wounded enemies
/// - Prioritize fast-moving threats
#[derive(Debug, Clone)]
pub struct ThreatBasedStrategy {
    /// Radius within which enemies are considered immediate threats.
    pub threat_radius: u32,
}

impl Default for ThreatBasedStrategy {
    fn default() -> Self {
        Self { threat_radius: 5 }
    }
}

impl ThreatBasedStrategy {
    /// Create a new threat-based strategy with custom threat radius.
    pub fn with_radius(threat_radius: u32) -> Self {
        Self { threat_radius }
    }

    /// Calculate target priority score for an NPC.
    ///
    /// Higher score = higher priority for auto-targeting.
    fn calculate_priority(&self, npc: &ActorView, player_pos: Position) -> i32 {
        let distance = manhattan_distance(player_pos, npc.position);

        if distance <= self.threat_radius {
            // Within threat radius: very high priority
            let distance_factor = (self.threat_radius - distance) as i32 * 100;
            let health_factor = self.calculate_health_threat(npc);
            let speed_factor = npc.stats.speed.physical / 10;

            2000 + distance_factor + health_factor + speed_factor
        } else {
            // Outside threat radius: moderate priority (closer = better)
            1000 - distance as i32
        }
    }

    /// Calculate threat/priority factor based on enemy health.
    ///
    /// Lower health = higher priority (finish off wounded enemies).
    /// Returns a score in range [0, 1000].
    fn calculate_health_threat(&self, npc: &ActorView) -> i32 {
        let (current, maximum) = npc.stats.hp();
        let hp_percent = health_percentage(current, maximum);

        // Lower health = higher priority (finish off wounded enemies)
        (100 - hp_percent) as i32 * 10
    }
}

impl TargetingStrategy for ThreatBasedStrategy {
    fn select_target(&self, view_model: &ViewModel) -> Option<Position> {
        let player_pos = view_model.player.position;
        let mut best_target: Option<(Position, i32)> = None;

        // Iterate over NPCs and find highest-priority target
        for npc in view_model.npcs() {
            let priority = self.calculate_priority(npc, player_pos);

            match best_target {
                None => best_target = Some((npc.position, priority)),
                Some((_, current_priority)) if priority > current_priority => {
                    best_target = Some((npc.position, priority));
                }
                _ => {}
            }
        }

        best_target.map(|(pos, _)| pos)
    }

    fn name(&self) -> &'static str {
        "Threat-Based"
    }

    fn description(&self) -> &'static str {
        "Prioritizes nearby enemies, focusing on wounded targets and fast-moving threats"
    }
}
