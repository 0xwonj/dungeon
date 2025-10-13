//! Event loop orchestrating runtime events, user input, and rendering.
//!
//! This module coordinates three main concerns:
//! - Runtime event consumption (game state updates)
//! - Keyboard input processing (player actions and UI navigation)
//! - Rendering and auto-target computation
use anyhow::Result;
use crossterm::event::{self, Event, KeyEvent, KeyEventKind};
use game_core::{Action, EntityId, GameState, env::MapOracle};
use runtime::{GameEvent, RuntimeHandle};
use tokio::{
    sync::{broadcast, broadcast::error::RecvError, mpsc},
    time::{self, Duration},
};

use crate::{
    cursor::{ChainSelector, CursorMovement, select_target},
    input::{InputHandler, KeyAction},
    presentation::{terminal::Tui, ui},
    state::{AppMode, AppState},
};
use frontend_core::EventConsumer;

const FRAME_INTERVAL_MS: u64 = 16;

pub struct EventLoop<C>
where
    C: EventConsumer,
{
    handle: RuntimeHandle,
    event_rx: broadcast::Receiver<GameEvent>,
    tx_action: mpsc::Sender<Action>,
    input: InputHandler,
    consumer: C,
    app_state: AppState,
}

impl<C> EventLoop<C>
where
    C: EventConsumer,
{
    pub fn new(
        handle: RuntimeHandle,
        event_rx: broadcast::Receiver<GameEvent>,
        tx_action: mpsc::Sender<Action>,
        player_entity: EntityId,
        consumer: C,
    ) -> Self {
        Self {
            handle,
            event_rx,
            tx_action,
            input: InputHandler::new(player_entity),
            consumer,
            app_state: AppState::new(),
        }
    }

    pub async fn run<M: MapOracle>(
        mut self,
        terminal: &mut Tui,
        map: &M,
        initial_state: GameState,
    ) -> Result<C> {
        self.render_with_state(terminal, map, &initial_state)?;

        loop {
            tokio::select! {
                result = self.event_rx.recv() => {
                    if self.handle_runtime_channel(result, terminal, map).await? {
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

    async fn handle_runtime_channel<M: MapOracle>(
        &mut self,
        result: Result<GameEvent, RecvError>,
        terminal: &mut Tui,
        map: &M,
    ) -> Result<bool> {
        match result {
            Ok(event) => {
                if self.collect_events(event) {
                    self.refresh_view(terminal, map).await?;
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

    fn collect_events(&mut self, initial: GameEvent) -> bool {
        let mut should_render = self.consumer.on_event(&initial).requires_redraw;

        while let Ok(event) = self.event_rx.try_recv() {
            let impact = self.consumer.on_event(&event);
            should_render |= impact.requires_redraw;
        }

        should_render
    }

    async fn handle_input_tick<M: MapOracle>(
        &mut self,
        terminal: &mut Tui,
        map: &M,
    ) -> Result<bool> {
        if !event::poll(Duration::from_millis(0))? {
            return Ok(false);
        }

        match event::read()? {
            Event::Key(key) if key.kind == KeyEventKind::Press => {
                self.handle_key_press(key, terminal, map).await
            }
            Event::Resize(_, _) => {
                self.refresh_view(terminal, map).await?;
                Ok(false)
            }
            _ => Ok(false),
        }
    }

    async fn handle_key_press<M: MapOracle>(
        &mut self,
        key: KeyEvent,
        terminal: &mut Tui,
        map: &M,
    ) -> Result<bool> {
        match self.input.handle_key(key) {
            KeyAction::Quit => {
                let state = self.handle.query_state().await?;
                self.consumer
                    .message_log_mut()
                    .push_text(format!("[{}] Quitting...", state.turn.clock.0));
                self.render_with_state(terminal, map, &state)?;
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
                let state = self.handle.query_state().await?;
                self.app_state
                    .toggle_examine(state.entities.player.position);
                self.input.set_modal(self.app_state.is_modal());
                self.render_with_state(terminal, map, &state)?;
                Ok(false)
            }
            KeyAction::ExitModal => {
                self.app_state.exit_to_normal();
                self.input.set_modal(false);
                self.refresh_view(terminal, map).await?;
                Ok(false)
            }
            KeyAction::MoveCursor(direction) => {
                if let Some(cursor) = &mut self.app_state.manual_cursor {
                    let (dx, dy) = direction.to_delta();
                    let state = self.handle.query_state().await?;
                    let dimensions = map.dimensions();
                    cursor.move_by(dx, dy, dimensions.width, dimensions.height);
                    self.app_state.entity_index = 0;
                    self.render_with_state(terminal, map, &state)?;
                }
                Ok(false)
            }
            KeyAction::NextEntity => {
                self.app_state.next_entity();
                self.refresh_view(terminal, map).await?;
                Ok(false)
            }
            KeyAction::PrevEntity => {
                self.app_state.prev_entity();
                self.refresh_view(terminal, map).await?;
                Ok(false)
            }
            KeyAction::None => Ok(false),
        }
    }

    async fn refresh_view<M: MapOracle>(&mut self, terminal: &mut Tui, map: &M) -> Result<()> {
        let state = self.handle.query_state().await?;
        self.render_with_state(terminal, map, &state)
    }

    fn render_with_state<M: MapOracle>(
        &mut self,
        terminal: &mut Tui,
        map: &M,
        state: &GameState,
    ) -> Result<()> {
        self.input.set_player_entity(state.entities.player.id);

        // Compute auto-target in Normal mode
        if self.app_state.mode == AppMode::Normal {
            let player_pos = state.entities.player.position;
            let selector = ChainSelector::combat_default(player_pos);
            let target_pos = select_target(state, &selector).or(Some(player_pos));
            self.app_state.set_auto_target(target_pos);
        }

        self.render(terminal, map, state)
    }

    fn render<M: MapOracle>(
        &mut self,
        terminal: &mut Tui,
        map: &M,
        state: &GameState,
    ) -> Result<()> {
        let frame = ui::build_frame(map, state, self.consumer.message_log());
        ui::render(terminal, &frame, &self.app_state, state, map)
    }
}
