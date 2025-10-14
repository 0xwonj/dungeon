//! Smart target selection system for auto-tracking entities.
//!
//! This module provides a flexible, priority-based system for automatically
//! selecting which entity to display in the Examine panel during normal gameplay.

use game_core::{EntityId, GameState, Position};

use super::movement::manhattan_distance;

/// Result of target selection with priority score.
#[derive(Clone, Debug)]
pub struct TargetCandidate {
    pub position: Position,
    pub priority: i32,
}

/// Trait for implementing target selection strategies.
///
/// Implement this trait to create custom auto-target behaviors.
/// Higher priority values are preferred.
pub trait TargetSelector: Send + Sync {
    /// Evaluates an entity and returns a candidate with priority score.
    /// Return None if the entity should not be considered.
    fn evaluate(
        &self,
        state: &GameState,
        entity_id: EntityId,
        position: Position,
    ) -> Option<TargetCandidate>;

    /// Optional: filter entities before evaluation (optimization).
    fn filter(&self, _state: &GameState, _entity_id: EntityId) -> bool {
        true // Include by default
    }
}

/// Selects the nearest hostile NPC.
pub struct NearestHostileSelector {
    pub origin: Position,
}

impl TargetSelector for NearestHostileSelector {
    fn evaluate(
        &self,
        state: &GameState,
        entity_id: EntityId,
        position: Position,
    ) -> Option<TargetCandidate> {
        // Only consider NPCs
        if !state.entities.npcs.iter().any(|npc| npc.id == entity_id) {
            return None;
        }

        let distance = manhattan_distance(self.origin, position);
        // Priority: inverse of distance (closer = higher priority)
        // Base priority 1000 for hostile NPCs
        let priority = 1000 - distance as i32;

        Some(TargetCandidate { position, priority })
    }

    fn filter(&self, state: &GameState, entity_id: EntityId) -> bool {
        // Only NPCs (in the future, check hostility flag)
        state.entities.npcs.iter().any(|npc| npc.id == entity_id)
    }
}

/// Selects the nearest entity of any type.
pub struct NearestAnySelector {
    pub origin: Position,
}

impl TargetSelector for NearestAnySelector {
    fn evaluate(
        &self,
        _state: &GameState,
        _entity_id: EntityId,
        position: Position,
    ) -> Option<TargetCandidate> {
        let distance = manhattan_distance(self.origin, position);
        // Base priority 500 for any entity
        let priority = 500 - distance as i32;

        Some(TargetCandidate { position, priority })
    }
}

/// Selects based on threat level (health, proximity, etc).
pub struct ThreatPrioritySelector {
    pub origin: Position,
    pub threat_radius: u32,
}

impl TargetSelector for ThreatPrioritySelector {
    fn evaluate(
        &self,
        state: &GameState,
        entity_id: EntityId,
        position: Position,
    ) -> Option<TargetCandidate> {
        let npc = state.entities.npcs.iter().find(|n| n.id == entity_id)?;

        let distance = manhattan_distance(self.origin, position);
        if distance > self.threat_radius {
            return None;
        }

        // Threat calculation:
        // - Closer = more threatening
        // - Lower health = prioritize (finish off wounded enemies)
        let distance_factor = (self.threat_radius - distance) as i32 * 100;
        let health_factor = 1000 - npc.stats.resources.hp as i32; // Lower HP = higher priority
        let speed_factor = npc.stats.speed_physical(); // Faster = more threatening

        let priority = 2000 + distance_factor + health_factor + speed_factor;

        Some(TargetCandidate { position, priority })
    }

    fn filter(&self, state: &GameState, entity_id: EntityId) -> bool {
        state.entities.npcs.iter().any(|npc| npc.id == entity_id)
    }
}

/// Composite selector that tries multiple strategies in order.
pub struct ChainSelector {
    selectors: Vec<Box<dyn TargetSelector>>,
}

impl ChainSelector {
    pub fn new(selectors: Vec<Box<dyn TargetSelector>>) -> Self {
        Self { selectors }
    }

    /// Convenience builder for default combat strategy.
    pub fn combat_default(player_pos: Position) -> Self {
        Self::new(vec![
            Box::new(ThreatPrioritySelector {
                origin: player_pos,
                threat_radius: 5,
            }),
            Box::new(NearestHostileSelector { origin: player_pos }),
            Box::new(NearestAnySelector { origin: player_pos }),
        ])
    }
}

impl TargetSelector for ChainSelector {
    fn evaluate(
        &self,
        state: &GameState,
        entity_id: EntityId,
        position: Position,
    ) -> Option<TargetCandidate> {
        // Try each selector and return the first valid candidate
        for selector in &self.selectors {
            if let Some(candidate) = selector.evaluate(state, entity_id, position) {
                return Some(candidate);
            }
        }
        None
    }

    fn filter(&self, state: &GameState, entity_id: EntityId) -> bool {
        // Pass if any selector passes
        self.selectors.iter().any(|s| s.filter(state, entity_id))
    }
}

/// Selects the best target position from game state using the given selector.
///
/// This function iterates through all entities, evaluates them with the selector,
/// and returns the position of the highest-priority target.
pub fn select_target(state: &GameState, selector: &dyn TargetSelector) -> Option<Position> {
    let mut candidates = Vec::new();

    // Collect candidates from player
    if selector.filter(state, state.entities.player.id)
        && let Some(candidate) = selector.evaluate(
            state,
            state.entities.player.id,
            state.entities.player.position,
        )
    {
        candidates.push(candidate);
    }

    // Collect candidates from NPCs
    for npc in state.entities.npcs.iter() {
        if selector.filter(state, npc.id)
            && let Some(candidate) = selector.evaluate(state, npc.id, npc.position)
        {
            candidates.push(candidate);
        }
    }

    // Collect candidates from props
    for prop in state.entities.props.iter() {
        if selector.filter(state, prop.id)
            && let Some(candidate) = selector.evaluate(state, prop.id, prop.position)
        {
            candidates.push(candidate);
        }
    }

    // Collect candidates from loose items
    for item in state.entities.items.iter() {
        if selector.filter(state, item.id)
            && let Some(candidate) = selector.evaluate(state, item.id, item.position)
        {
            candidates.push(candidate);
        }
    }

    // Return the highest priority candidate
    candidates
        .into_iter()
        .max_by_key(|c| c.priority)
        .map(|c| c.position)
}
