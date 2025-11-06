//! Input handling (keyboard and directional input).

use anyhow::Result;
use client_core::EventConsumer;
use crossterm::event::{self as term_event, Event as TermEvent, KeyEvent, KeyEventKind};
use game_core::{Action, EntityId, env::MapOracle};
use tokio::time::Duration;

use super::super::EventLoop;
use crate::{
    cursor::CursorMovement,
    input::KeyAction,
    presentation::terminal::Tui,
    state::{AppMode, TargetingInputMode},
};

impl<C> EventLoop<C>
where
    C: EventConsumer,
{
    /// Poll for keyboard input and handle UI interactions.
    pub(in crate::event) async fn handle_input_tick(&mut self, terminal: &mut Tui) -> Result<bool> {
        if !term_event::poll(Duration::from_millis(0))? {
            return Ok(false);
        }

        match term_event::read()? {
            TermEvent::Key(key) if key.kind == KeyEventKind::Press => {
                self.handle_key_press(key, terminal).await
            }
            TermEvent::Resize(_, _) => {
                self.render(terminal)?;
                Ok(false)
            }
            _ => Ok(false),
        }
    }

    /// Handle key press and dispatch to appropriate handler.
    pub(in crate::event) async fn handle_key_press(
        &mut self,
        key: KeyEvent,
        terminal: &mut Tui,
    ) -> Result<bool> {
        match self.input.handle_key(key, &self.app_state.mode) {
            KeyAction::Quit => {
                self.consumer
                    .message_log_mut()
                    .push_text(format!("[{}] Quitting...", self.view_model.turn.clock));

                // Just render the quit message
                self.render(terminal)?;
                Ok(true)
            }
            KeyAction::Submit(action) => {
                if self.tx_action.send(action).await.is_err() {
                    tracing::error!("Action channel closed");
                    return Ok(true);
                }
                Ok(false)
            }
            KeyAction::ToggleExamine => {
                let cursor_pos = if let Some(entity_id) = self.app_state.highlighted_entity {
                    // Place cursor at highlighted entity's position
                    self.view_model
                        .actors
                        .iter()
                        .find(|a| a.id == entity_id)
                        .and_then(|a| a.position)
                        .or(self.view_model.player.position)
                        .unwrap_or_else(|| game_core::Position::new(0, 0))
                } else {
                    // No highlighted entity - default to player
                    self.view_model
                        .player
                        .position
                        .unwrap_or_else(|| game_core::Position::new(0, 0))
                };

                self.app_state.toggle_examine(cursor_pos);
                self.render(terminal)?;
                Ok(false)
            }
            KeyAction::ExitModal => {
                self.app_state.exit_to_normal();
                self.compute_auto_target();
                self.render(terminal)?;
                Ok(false)
            }
            KeyAction::MoveCursor(direction) => {
                // Check if in SelectDirection targeting mode
                if let AppMode::Targeting(targeting_state) = &mut self.app_state.mode
                    && let TargetingInputMode::Direction { selected } =
                        &mut targeting_state.input_mode
                {
                    // Update selected direction
                    *selected = Some(direction);
                    self.render(terminal)?;
                    return Ok(false);
                }

                // Normal cursor movement (ExamineManual mode or SelectPosition targeting)
                if let Some(cursor) = &mut self.app_state.manual_cursor {
                    let (dx, dy) = direction.to_delta();
                    let dimensions = self.oracles.map.dimensions();
                    cursor.move_by(dx, dy, dimensions.width, dimensions.height);

                    // Update highlighted entity to first entity at new cursor position
                    self.update_highlighted_at_cursor();
                    self.render(terminal)?;
                }
                Ok(false)
            }
            KeyAction::NextEntity => {
                if self.app_state.mode == AppMode::Normal {
                    // Normal mode: cycle through all NPCs
                    self.cycle_highlighted_entity(1);
                } else {
                    // Manual mode: cycle through entities at cursor position
                    self.cycle_entities_at_cursor(1);
                }
                self.render(terminal)?;
                Ok(false)
            }
            KeyAction::PrevEntity => {
                if self.app_state.mode == AppMode::Normal {
                    // Normal mode: cycle through all NPCs (backwards)
                    self.cycle_highlighted_entity(-1);
                } else {
                    // Manual mode: cycle through entities at cursor position (backwards)
                    self.cycle_entities_at_cursor(-1);
                }
                self.render(terminal)?;
                Ok(false)
            }
            KeyAction::DirectionalInput(direction) => {
                self.handle_directional_input(direction).await?;
                Ok(false)
            }
            KeyAction::UseSlot(slot) => {
                self.handle_use_slot(slot).await?;
                self.render(terminal)?;
                Ok(false)
            }
            KeyAction::OpenAbilityMenu => {
                self.handle_open_ability_menu().await?;
                self.render(terminal)?;
                Ok(false)
            }
            KeyAction::SelectAbilityForSlot(ability_idx) => {
                self.handle_select_ability(ability_idx)?;
                self.render(terminal)?;
                Ok(false)
            }
            KeyAction::ConfirmTarget => {
                self.handle_confirm_target().await?;
                self.render(terminal)?;
                Ok(false)
            }
            KeyAction::None => Ok(false),
        }
    }

    /// Handle directional input: Bump-to-attack or Move.
    pub(in crate::event) async fn handle_directional_input(
        &mut self,
        direction: game_core::CardinalDirection,
    ) -> Result<()> {
        use game_core::{ActionInput, ActionKind, CharacterAction};

        let Some(player_pos) = self.view_model.player.position else {
            return Ok(());
        };
        let (dx, dy) = direction.offset();
        let target_pos = game_core::Position::new(player_pos.x + dx, player_pos.y + dy);

        // Check if there's an enemy at target position
        let enemy_at_target = self.view_model.actors.iter().find(|actor| {
            actor.id != EntityId::PLAYER
                && actor.position == Some(target_pos)
                && actor.stats.resource_current.hp > 0
        });

        let action = if let Some(enemy) = enemy_at_target {
            // Bump-to-attack: Attack the enemy
            CharacterAction::new(
                EntityId::PLAYER,
                ActionKind::MeleeAttack,
                ActionInput::Entity(enemy.id),
            )
        } else {
            // No enemy: Just move
            CharacterAction::new(
                EntityId::PLAYER,
                ActionKind::Move,
                ActionInput::Direction(direction),
            )
        };

        self.tx_action.send(Action::Character(action)).await?;
        Ok(())
    }
}
