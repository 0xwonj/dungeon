//! Event loop orchestrating runtime events, user input, and rendering.
//!
//! This module coordinates three main concerns:
//! - Runtime event consumption and ViewModel updates (via ViewModelUpdater)
//! - Keyboard input processing (player actions and UI navigation)
//! - Rendering using ViewModel and auto-target computation

use std::collections::HashMap;

use anyhow::Result;
use game_core::{Action, EntityId, GameState};
use runtime::{Event as RuntimeEvent, RuntimeHandle, Topic};
use tokio::{
    sync::{broadcast, broadcast::error::RecvError, mpsc},
    time::{self, Duration},
};

use crate::{input::InputHandler, presentation::terminal::Tui, state::AppState};
use client_bootstrap::oracles::OracleBundle;
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
    pub(crate) handle: RuntimeHandle,
    pub(crate) subscriptions: HashMap<Topic, broadcast::Receiver<RuntimeEvent>>,
    pub(crate) tx_action: mpsc::Sender<Action>,
    pub(crate) input: InputHandler,
    pub(crate) consumer: C,
    pub(crate) app_state: AppState,
    /// Owned ViewModel - incrementally updated via ViewModelUpdater
    pub(crate) view_model: ViewModel,
    /// Pluggable targeting strategy for auto-target selection
    pub(crate) target_selector: TargetSelector,
    /// Oracle bundle for accessing game data (maps, items, tables, etc.)
    pub(crate) oracles: OracleBundle,
    /// CLI UI configuration
    pub(crate) cli_config: crate::config::CliConfig,
}

impl<C> EventLoop<C>
where
    C: EventConsumer,
{
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        handle: RuntimeHandle,
        subscriptions: HashMap<Topic, broadcast::Receiver<RuntimeEvent>>,
        tx_action: mpsc::Sender<Action>,
        player_entity: EntityId,
        consumer: C,
        initial_state: &GameState,
        oracles: OracleBundle,
        target_selector: Option<TargetSelector>,
        cli_config: crate::config::CliConfig,
    ) -> Self {
        let view_model = ViewModel::from_initial_state(initial_state, oracles.map.as_ref());

        Self {
            handle,
            subscriptions,
            tx_action,
            input: InputHandler::new(player_entity),
            consumer,
            app_state: AppState::new(),
            view_model,
            target_selector: target_selector.unwrap_or_default(),
            oracles,
            cli_config,
        }
    }

    pub async fn run(mut self, terminal: &mut Tui) -> Result<C> {
        // Initial render
        self.compute_auto_target();
        self.render(terminal)?;

        // Extract receivers from subscriptions
        let mut game_rx = self.subscriptions.remove(&Topic::GameState);
        let mut proof_rx = self.subscriptions.remove(&Topic::Proof);

        loop {
            tokio::select! {
                result = async { game_rx.as_mut().unwrap().recv().await }, if game_rx.is_some() => {
                    if self.handle_runtime_event(result, terminal).await? {
                        break;
                    }
                }
                result = async { proof_rx.as_mut().unwrap().recv().await }, if proof_rx.is_some() => {
                    if self.handle_runtime_event(result, terminal).await? {
                        break;
                    }
                }
                _ = time::sleep(Duration::from_millis(FRAME_INTERVAL_MS)) => {
                    if self.handle_input_tick(terminal).await? {
                        break;
                    }
                }
            }
        }

        Ok(self.consumer)
    }

    /// Handle runtime event and update ViewModel incrementally.
    async fn handle_runtime_event(
        &mut self,
        result: Result<RuntimeEvent, RecvError>,
        terminal: &mut Tui,
    ) -> Result<bool> {
        match result {
            Ok(event) => {
                // Let consumer process event (message logging, etc.)
                let impact = self.consumer.on_event(&event);

                // Update ViewModel incrementally using ViewModelUpdater service
                if impact.requires_redraw {
                    let state = self.handle.query_state().await?;
                    let scope = ViewModelUpdater::update(
                        &mut self.view_model,
                        &event,
                        &state,
                        self.oracles.map.as_ref(),
                    );

                    // Only render if something actually changed
                    if !scope.is_empty() {
                        // Recompute auto-target after state change
                        self.compute_auto_target();
                        self.render(terminal)?;
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
}
