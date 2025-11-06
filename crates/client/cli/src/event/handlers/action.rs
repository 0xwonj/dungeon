//! Action execution handlers (slots, abilities, targeting).

use anyhow::Result;
use client_core::EventConsumer;
use game_core::{Action, EntityId};

use super::super::EventLoop;
use crate::state::{AppMode, TargetingInputMode, TargetingState};

impl<C> EventLoop<C>
where
    C: EventConsumer,
{
    /// Handle action slot usage (keys 1-9).
    pub(in crate::event) async fn handle_use_slot(&mut self, slot: usize) -> Result<()> {
        // Get action from slot
        let Some(action_kind) = self.app_state.action_slots.get(slot) else {
            return Ok(()); // No action assigned to this slot
        };

        // Check if action is available
        let is_available = self
            .view_model
            .player
            .actions
            .iter()
            .any(|ability| ability.kind == action_kind);

        if !is_available {
            return Ok(()); // Action not available right now
        }

        // Start executing the action (may enter targeting mode)
        self.start_action(action_kind).await?;
        Ok(())
    }

    /// Open ability menu to view/assign actions.
    pub(in crate::event) async fn handle_open_ability_menu(&mut self) -> Result<()> {
        // Enter ability menu mode
        // (Actions are queried from ViewModel during rendering)
        self.app_state.enter_ability_menu();
        Ok(())
    }

    /// Select ability from menu to assign to slot.
    pub(in crate::event) fn handle_select_ability(&mut self, ability_idx: usize) -> Result<()> {
        // Get the selected action from ViewModel (no query needed!)
        let action_kind = self
            .view_model
            .player
            .actions
            .get(ability_idx)
            .map(|ability| ability.kind);

        let Some(action_kind) = action_kind else {
            return Ok(()); // Invalid index
        };

        // For now, assign to the same slot index
        // TODO: Future - let user choose which slot
        self.app_state
            .action_slots
            .set(ability_idx, Some(action_kind));

        // Exit ability menu
        self.app_state.exit_to_normal();
        self.compute_auto_target();
        Ok(())
    }

    /// Confirm target selection in targeting mode.
    pub(in crate::event) async fn handle_confirm_target(&mut self) -> Result<()> {
        use game_core::{ActionInput, CharacterAction};

        let AppMode::Targeting(targeting_state) = &self.app_state.mode else {
            return Ok(()); // Not in targeting mode
        };

        let action_kind = targeting_state.action_kind;

        // Determine ActionInput based on targeting mode
        let input = match &targeting_state.input_mode {
            TargetingInputMode::Position {
                require_entity,
                max_range,
            } => {
                // Get cursor position
                let Some(cursor) = &self.app_state.manual_cursor else {
                    return Ok(()); // No cursor in targeting mode - shouldn't happen
                };

                let cursor_pos = cursor.position;

                // Validate range if specified
                if let Some(range) = max_range {
                    let Some(player_pos) = self.view_model.player.position else {
                        return Ok(());
                    };
                    let distance = chebyshev_distance(player_pos, cursor_pos);
                    if distance > *range {
                        // Out of range - show message and don't execute
                        self.consumer.message_log_mut().push_text(format!(
                            "[{}] Target out of range (max: {})",
                            self.view_model.turn.clock, range
                        ));
                        return Ok(());
                    }
                }

                // Check entity requirement
                if *require_entity {
                    // Must have an entity at cursor position
                    if let Some(entity_id) = self.app_state.highlighted_entity {
                        // Verify entity is not the player and is alive
                        let is_valid = self.view_model.actors.iter().any(|actor| {
                            actor.id == entity_id
                                && actor.id != EntityId::PLAYER
                                && actor.stats.resource_current.hp > 0
                        });

                        if is_valid {
                            Some(ActionInput::Entity(entity_id))
                        } else {
                            // Invalid entity
                            self.consumer.message_log_mut().push_text(format!(
                                "[{}] No valid target at cursor",
                                self.view_model.turn.clock
                            ));
                            return Ok(());
                        }
                    } else {
                        // No entity at cursor
                        self.consumer.message_log_mut().push_text(format!(
                            "[{}] No target at cursor",
                            self.view_model.turn.clock
                        ));
                        return Ok(());
                    }
                } else {
                    // Position-based action (AOE, teleport, etc)
                    Some(ActionInput::Position(cursor_pos))
                }
            }

            TargetingInputMode::Direction { selected } => {
                // Get selected direction
                selected.map(ActionInput::Direction)
            }
        };

        // Execute action if we have valid input
        if let Some(input) = input {
            let action = CharacterAction::new(EntityId::PLAYER, action_kind, input);
            self.tx_action.send(Action::Character(action)).await?;

            // Exit targeting mode
            self.app_state.exit_to_normal();
            self.compute_auto_target();
        }

        Ok(())
    }

    /// Start executing an action (may enter targeting mode).
    pub(in crate::event) async fn start_action(
        &mut self,
        action_kind: game_core::ActionKind,
    ) -> Result<()> {
        use game_core::{ActionInput, CharacterAction, env::TablesOracle};

        // Get targeting mode from action profile via TablesOracle
        let action_profile = self.oracles.tables.action_profile(action_kind);
        let targeting = action_profile.targeting;

        // Check targeting mode
        match &targeting {
            game_core::TargetingMode::None => {
                // No targeting - execute immediately
                let action = CharacterAction::new(EntityId::PLAYER, action_kind, ActionInput::None);
                self.tx_action.send(Action::Character(action)).await?;
            }

            game_core::TargetingMode::SelfOnly => {
                // Self-targeting - execute immediately
                let action = CharacterAction::new(
                    EntityId::PLAYER,
                    action_kind,
                    ActionInput::Entity(EntityId::PLAYER),
                );
                self.tx_action.send(Action::Character(action)).await?;
            }

            game_core::TargetingMode::SingleTarget { range, .. } => {
                // Entity targeting - enter position-based targeting mode
                if let Some(input_mode) = TargetingInputMode::from_targeting_mode(&targeting) {
                    // Find nearest valid target to place cursor
                    let valid_targets = self.find_targets_in_range(range);
                    let cursor_pos = valid_targets
                        .first()
                        .and_then(|&id| self.view_model.actors.iter().find(|a| a.id == id))
                        .and_then(|a| a.position)
                        .or(self.view_model.player.position)
                        .unwrap_or_else(|| game_core::Position::new(0, 0));

                    self.app_state.enter_targeting(
                        TargetingState {
                            action_kind,
                            input_mode,
                        },
                        cursor_pos,
                    );

                    // Set highlighted entity to first valid target
                    self.update_highlighted_at_cursor();
                }
            }

            game_core::TargetingMode::Directional { .. } => {
                // Direction targeting - enter targeting mode
                if let Some(input_mode) = TargetingInputMode::from_targeting_mode(&targeting) {
                    let player_pos = self
                        .view_model
                        .player
                        .position
                        .unwrap_or_else(|| game_core::Position::new(0, 0));
                    self.app_state.enter_targeting(
                        TargetingState {
                            action_kind,
                            input_mode,
                        },
                        player_pos,
                    );
                }
            }
        }

        Ok(())
    }
}

/// Calculate Chebyshev distance (chessboard distance).
fn chebyshev_distance(from: game_core::Position, to: game_core::Position) -> u32 {
    let dx = (from.x - to.x).abs();
    let dy = (from.y - to.y).abs();
    dx.max(dy) as u32
}
