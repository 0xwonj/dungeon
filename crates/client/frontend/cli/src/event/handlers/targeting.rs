//! Targeting and entity selection handlers.

use client_frontend_core::EventConsumer;
use game_core::EntityId;

use super::super::EventLoop;
use crate::state::AppMode;

impl<C> EventLoop<C>
where
    C: EventConsumer,
{
    /// Compute auto-target entity in Normal mode using targeting strategy.
    ///
    /// This queries the ViewModel via the pluggable TargetSelector to find the best target
    /// and updates AppState. The highlighted entity is used for both map highlighting and examine panel.
    pub(in crate::event) fn compute_auto_target(&mut self) {
        if self.app_state.mode == AppMode::Normal {
            // Get optimal target position from targeting strategy
            let target_entity =
                if let Some(target_pos) = self.target_selector.select_target(&self.view_model) {
                    // Convert Position → EntityId for entity-based tracking
                    self.view_model
                        .npcs()
                        .find(|npc| npc.position == Some(target_pos))
                        .map(|npc| npc.id)
                } else {
                    // No NPCs - default to player
                    Some(EntityId::PLAYER)
                };

            self.app_state.set_highlighted_entity(target_entity);
        }
    }

    /// Change targeting strategy at runtime (future: keybind like 'T' key).
    ///
    /// This allows players to switch between different targeting behaviors:
    /// - Threat-based (default): prioritize nearby threats with low health
    /// - Nearest: simple closest-enemy targeting
    /// - Lowest Health: finish off wounded enemies
    /// - Fastest: intercept fast-moving threats
    #[allow(dead_code)]
    pub(in crate::event) fn set_targeting_strategy(
        &mut self,
        strategy: Box<dyn client_frontend_core::services::targeting::TargetingStrategy>,
    ) {
        self.target_selector.set_strategy(strategy);
    }

    /// Cycle through all NPCs in Normal mode (Tab key).
    ///
    /// Direction: +1 for next, -1 for previous.
    /// Wraps around using modulo arithmetic.
    pub(in crate::event) fn cycle_highlighted_entity(&mut self, direction: i32) {
        let npcs: Vec<_> = self.view_model.npcs().collect();

        if npcs.is_empty() {
            // No NPCs - highlight player
            self.app_state
                .set_highlighted_entity(Some(EntityId::PLAYER));
            return;
        }

        // Find current highlighted NPC's index
        let current_idx = self
            .app_state
            .highlighted_entity
            .and_then(|id| npcs.iter().position(|npc| npc.id == id))
            .unwrap_or(0);

        // Cycle with wrapping (handles both positive and negative direction)
        let new_idx = (current_idx as i32 + direction).rem_euclid(npcs.len() as i32) as usize;

        self.app_state
            .set_highlighted_entity(Some(npcs[new_idx].id));
    }

    /// Cycle through entities at cursor position in Manual mode (Tab key).
    ///
    /// Direction: +1 for next, -1 for previous.
    /// Cycles through all entity types: NPCs, Items, and Props at the current cursor position.
    pub(in crate::event) fn cycle_entities_at_cursor(&mut self, direction: i32) {
        let Some(cursor_pos) = self.app_state.manual_cursor.as_ref().map(|c| c.position) else {
            return;
        };

        // Collect all entities at cursor position (NPCs, Items, Props)
        let mut entities_here: Vec<EntityId> = Vec::new();

        // Add NPCs
        entities_here.extend(
            self.view_model
                .npcs()
                .filter(|npc| npc.position == Some(cursor_pos))
                .map(|npc| npc.id),
        );

        // Add Items
        entities_here.extend(
            self.view_model
                .items
                .iter()
                .filter(|item| item.position == cursor_pos)
                .map(|item| item.id),
        );

        // Add Props
        entities_here.extend(
            self.view_model
                .props
                .iter()
                .filter(|prop| prop.position == cursor_pos)
                .map(|prop| prop.id),
        );

        if entities_here.is_empty() {
            // No entities at cursor - clear highlight
            self.app_state.set_highlighted_entity(None);
            return;
        }

        // Find current highlighted entity's index (if at this position)
        let current_idx = self
            .app_state
            .highlighted_entity
            .and_then(|id| entities_here.iter().position(|&eid| eid == id))
            .unwrap_or(0);

        // Cycle with wrapping
        let new_idx =
            (current_idx as i32 + direction).rem_euclid(entities_here.len() as i32) as usize;

        self.app_state
            .set_highlighted_entity(Some(entities_here[new_idx]));
    }

    /// Update highlighted entity when cursor moves in Manual mode.
    ///
    /// Highlights the first entity at the new cursor position, prioritizing NPCs > Items > Props.
    /// Returns None if no entities at cursor.
    pub(in crate::event) fn update_highlighted_at_cursor(&mut self) {
        let Some(cursor_pos) = self.app_state.manual_cursor.as_ref().map(|c| c.position) else {
            return;
        };

        // Priority: NPCs first, then Items, then Props
        let entity_at_cursor = self
            .view_model
            .npcs()
            .find(|npc| npc.position == Some(cursor_pos))
            .map(|npc| npc.id)
            .or_else(|| {
                self.view_model
                    .items
                    .iter()
                    .find(|item| item.position == cursor_pos)
                    .map(|item| item.id)
            })
            .or_else(|| {
                self.view_model
                    .props
                    .iter()
                    .find(|prop| prop.position == cursor_pos)
                    .map(|prop| prop.id)
            });

        self.app_state.set_highlighted_entity(entity_at_cursor);
    }

    /// Find all valid target entities within range of player.
    pub(in crate::event) fn find_targets_in_range(&self, range: &u32) -> Vec<EntityId> {
        let Some(player_pos) = self.view_model.player.position else {
            return vec![];
        };

        self.view_model
            .actors
            .iter()
            .filter(|actor| {
                actor.id != EntityId::PLAYER
                    && actor.stats.resource_current.hp > 0
                    && actor
                        .position
                        .is_some_and(|pos| chebyshev_distance(player_pos, pos) <= *range)
            })
            .map(|actor| actor.id)
            .collect()
    }
}

/// Calculate Chebyshev distance (chessboard distance).
fn chebyshev_distance(from: game_core::Position, to: game_core::Position) -> u32 {
    let dx = (from.x - to.x).abs();
    let dy = (from.y - to.y).abs();
    dx.max(dy) as u32
}
