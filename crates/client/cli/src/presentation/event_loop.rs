//! Event loop orchestrating runtime events, user input, and rendering.
//!
//! This module coordinates three main concerns:
//! - Runtime event consumption and ViewModel updates (via ViewModelUpdater)
//! - Keyboard input processing (player actions and UI navigation)
//! - Rendering using ViewModel and auto-target computation

use std::collections::HashMap;

use anyhow::Result;
use crossterm::event::{self as term_event, Event as TermEvent, KeyEvent, KeyEventKind};
use game_core::{Action, EntityId, GameState, env::MapOracle};
use runtime::{Event as RuntimeEvent, RuntimeHandle, Topic};
use tokio::{
    sync::{broadcast, broadcast::error::RecvError, mpsc},
    time::{self, Duration},
};

use crate::{
    cursor::CursorMovement,
    input::{InputHandler, KeyAction},
    presentation::{terminal::Tui, ui},
    state::{AppMode, AppState},
};
use client_core::{
    EventConsumer,
    services::{ViewModelUpdater, targeting::TargetSelector},
    view_model::ViewModel,
};

const FRAME_INTERVAL_MS: u64 = 16;

/// Event loop managing ViewModel state and coordinating UI updates.
///
/// This is the main orchestrator that:
/// - Owns the ViewModel (single source of truth for presentation state)
/// - Uses ViewModelUpdater service to apply runtime events incrementally
/// - Computes auto-targets using pluggable targeting strategies
/// - Handles user input and forwards actions to runtime
pub struct EventLoop<C>
where
    C: EventConsumer,
{
    handle: RuntimeHandle,
    subscriptions: HashMap<Topic, broadcast::Receiver<RuntimeEvent>>,
    tx_action: mpsc::Sender<Action>,
    input: InputHandler,
    consumer: C,
    app_state: AppState,
    /// Owned ViewModel - incrementally updated via ViewModelUpdater
    view_model: ViewModel,
    /// Pluggable targeting strategy for auto-target selection
    target_selector: TargetSelector,
}

impl<C> EventLoop<C>
where
    C: EventConsumer,
{
    #[allow(clippy::too_many_arguments)]
    pub fn new<M: MapOracle>(
        handle: RuntimeHandle,
        subscriptions: HashMap<Topic, broadcast::Receiver<RuntimeEvent>>,
        tx_action: mpsc::Sender<Action>,
        player_entity: EntityId,
        consumer: C,
        initial_state: &GameState,
        map_oracle: &M,
        target_selector: Option<TargetSelector>,
    ) -> Self {
        let view_model = ViewModel::from_initial_state(initial_state, map_oracle);

        Self {
            handle,
            subscriptions,
            tx_action,
            input: InputHandler::new(player_entity),
            consumer,
            app_state: AppState::new(),
            view_model,
            target_selector: target_selector.unwrap_or_default(),
        }
    }

    pub async fn run<M: MapOracle>(mut self, terminal: &mut Tui, map: &M) -> Result<C> {
        // Initial render
        self.compute_auto_target();
        self.render(terminal, map)?;

        // Extract receivers from subscriptions
        let mut game_rx = self.subscriptions.remove(&Topic::GameState);
        let mut proof_rx = self.subscriptions.remove(&Topic::Proof);

        loop {
            tokio::select! {
                result = async { game_rx.as_mut().unwrap().recv().await }, if game_rx.is_some() => {
                    if self.handle_runtime_event(result, terminal, map).await? {
                        break;
                    }
                }
                result = async { proof_rx.as_mut().unwrap().recv().await }, if proof_rx.is_some() => {
                    if self.handle_runtime_event(result, terminal, map).await? {
                        break;
                    }
                }
                _ = time::sleep(Duration::from_millis(FRAME_INTERVAL_MS)) => {
                    if self.handle_input_tick(terminal, map).await? {
                        break;
                    }
                }
            }
        }

        Ok(self.consumer)
    }

    /// Handle runtime event and update ViewModel incrementally.
    async fn handle_runtime_event<M: MapOracle>(
        &mut self,
        result: Result<RuntimeEvent, RecvError>,
        terminal: &mut Tui,
        map: &M,
    ) -> Result<bool> {
        match result {
            Ok(event) => {
                // Let consumer process event (message logging, etc.)
                let impact = self.consumer.on_event(&event);

                // Update ViewModel incrementally using ViewModelUpdater service
                if impact.requires_redraw {
                    let state = self.handle.query_state().await?;
                    let scope = ViewModelUpdater::update(&mut self.view_model, &event, &state, map);

                    // Only render if something actually changed
                    if !scope.is_empty() {
                        // Recompute auto-target after state change
                        self.compute_auto_target();
                        self.render(terminal, map)?;
                    }
                }
                Ok(false)
            }
            Err(RecvError::Closed) => {
                tracing::warn!("Event stream closed");
                Ok(true)
            }
            Err(RecvError::Lagged(skipped)) => {
                tracing::warn!("Dropped {} stale events", skipped);
                Ok(false)
            }
        }
    }

    /// Poll for keyboard input and handle UI interactions.
    async fn handle_input_tick<M: MapOracle>(
        &mut self,
        terminal: &mut Tui,
        map: &M,
    ) -> Result<bool> {
        if !term_event::poll(Duration::from_millis(0))? {
            return Ok(false);
        }

        match term_event::read()? {
            TermEvent::Key(key) if key.kind == KeyEventKind::Press => {
                self.handle_key_press(key, terminal, map).await
            }
            TermEvent::Resize(_, _) => {
                self.render(terminal, map)?;
                Ok(false)
            }
            _ => Ok(false),
        }
    }

    /// Handle key press and dispatch to appropriate handler.
    async fn handle_key_press<M: MapOracle>(
        &mut self,
        key: KeyEvent,
        terminal: &mut Tui,
        map: &M,
    ) -> Result<bool> {
        match self.input.handle_key(key) {
            KeyAction::Quit => {
                self.consumer
                    .message_log_mut()
                    .push_text(format!("[{}] Quitting...", self.view_model.turn.clock));

                // Just render the quit message
                self.render(terminal, map)?;
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
                        .map(|a| a.position)
                        .unwrap_or(self.view_model.player.position)
                } else {
                    // No highlighted entity - default to player
                    self.view_model.player.position
                };

                self.app_state.toggle_examine(cursor_pos);
                self.input.set_modal(self.app_state.is_modal());
                self.render(terminal, map)?;
                Ok(false)
            }
            KeyAction::ExitModal => {
                self.app_state.exit_to_normal();
                self.input.set_modal(false);
                self.compute_auto_target();
                self.render(terminal, map)?;
                Ok(false)
            }
            KeyAction::MoveCursor(direction) => {
                if let Some(cursor) = &mut self.app_state.manual_cursor {
                    let (dx, dy) = direction.to_delta();
                    let dimensions = map.dimensions();
                    cursor.move_by(dx, dy, dimensions.width, dimensions.height);

                    // Update highlighted entity to first entity at new cursor position
                    self.update_highlighted_at_cursor();
                    self.render(terminal, map)?;
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
                self.render(terminal, map)?;
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
                self.render(terminal, map)?;
                Ok(false)
            }
            KeyAction::None => Ok(false),
        }
    }

    /// Compute auto-target entity in Normal mode using targeting strategy.
    ///
    /// This queries the ViewModel via the pluggable TargetSelector to find the best target
    /// and updates AppState. The highlighted entity is used for both map highlighting and examine panel.
    fn compute_auto_target(&mut self) {
        if self.app_state.mode == AppMode::Normal {
            // Get optimal target position from targeting strategy
            let target_entity =
                if let Some(target_pos) = self.target_selector.select_target(&self.view_model) {
                    // Convert Position → EntityId for entity-based tracking
                    self.view_model
                        .npcs()
                        .find(|npc| npc.position == target_pos)
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
    fn set_targeting_strategy(
        &mut self,
        strategy: Box<dyn client_core::services::targeting::TargetingStrategy>,
    ) {
        self.target_selector.set_strategy(strategy);
    }

    /// Cycle through all NPCs in Normal mode (Tab key).
    ///
    /// Direction: +1 for next, -1 for previous.
    /// Wraps around using modulo arithmetic.
    fn cycle_highlighted_entity(&mut self, direction: i32) {
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
    /// Only cycles through NPCs at the current cursor position.
    fn cycle_entities_at_cursor(&mut self, direction: i32) {
        let Some(cursor_pos) = self.app_state.manual_cursor.as_ref().map(|c| c.position) else {
            return;
        };

        // Collect all NPCs at cursor position
        let entities_here: Vec<_> = self
            .view_model
            .npcs()
            .filter(|npc| npc.position == cursor_pos)
            .collect();

        if entities_here.is_empty() {
            // No entities at cursor - clear highlight
            self.app_state.set_highlighted_entity(None);
            return;
        }

        // Find current highlighted entity's index (if at this position)
        let current_idx = self
            .app_state
            .highlighted_entity
            .and_then(|id| entities_here.iter().position(|npc| npc.id == id))
            .unwrap_or(0);

        // Cycle with wrapping
        let new_idx =
            (current_idx as i32 + direction).rem_euclid(entities_here.len() as i32) as usize;

        self.app_state
            .set_highlighted_entity(Some(entities_here[new_idx].id));
    }

    /// Update highlighted entity when cursor moves in Manual mode.
    ///
    /// Highlights the first NPC at the new cursor position, or None if no entities.
    fn update_highlighted_at_cursor(&mut self) {
        let Some(cursor_pos) = self.app_state.manual_cursor.as_ref().map(|c| c.position) else {
            return;
        };

        // Find first NPC at cursor position
        let entity_at_cursor = self
            .view_model
            .npcs()
            .find(|npc| npc.position == cursor_pos)
            .map(|npc| npc.id);

        self.app_state.set_highlighted_entity(entity_at_cursor);
    }

    /// Render current state using ViewModel.
    fn render<M: MapOracle>(&mut self, terminal: &mut Tui, map: &M) -> Result<()> {
        self.input.set_player_entity(self.view_model.player.id);

        // ✅ NEW ARCHITECTURE: Direct ViewModel usage with new widgets!
        ui::render_with_view_model(
            terminal,
            &self.view_model,
            self.consumer.message_log(),
            &self.app_state,
            map,
        )
    }
}
