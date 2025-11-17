//! CLI frontend implementation.
//!
//! Pure UI layer that communicates with the game via RuntimeHandle only.
//! Does NOT own the Runtime - receives a handle for communication.

use anyhow::Result;
use async_trait::async_trait;
use tokio::sync::mpsc;

use game_core::{Action, EntityId};
use runtime::{InteractiveKind, ProviderKind, RuntimeHandle, Topic};

use crate::event::{CliEventConsumer, EventLoop};
use crate::input::CliActionProvider;
use crate::presentation::terminal;
use client_bootstrap::oracles::OracleBundle;
use client_frontend_core::{FrontendConfig, message::MessageLog};

/// CLI frontend (pure UI layer).
///
/// This struct handles:
/// - Terminal rendering
/// - User input collection
/// - Event consumption from runtime
/// - Action submission to runtime
///
/// It does NOT:
/// - Own the Runtime
/// - Manage Runtime lifecycle
/// - Configure Runtime workers
///
/// All communication with the game happens via RuntimeHandle.
pub struct CliFrontend {
    config: FrontendConfig,
    cli_config: crate::config::CliConfig,
    oracles: OracleBundle,
}

impl CliFrontend {
    /// Create a new CLI frontend.
    ///
    /// # Parameters
    ///
    /// - `config`: Frontend configuration (channels, messages)
    /// - `cli_config`: CLI-specific configuration (keybindings, etc.)
    /// - `oracles`: Oracle bundle for static game content
    pub fn new(
        config: FrontendConfig,
        cli_config: crate::config::CliConfig,
        oracles: OracleBundle,
    ) -> Self {
        Self {
            config,
            cli_config,
            oracles,
        }
    }
}

#[async_trait]
impl client_frontend_core::Frontend for CliFrontend {
    async fn run(&mut self, handle: RuntimeHandle) -> Result<()> {
        tracing::info!("CLI frontend starting...");

        // Setup CLI-specific action provider (interactive input)
        let (tx_action, rx_action) = mpsc::channel::<Action>(self.config.channels.action_buffer);

        let cli_kind = ProviderKind::Interactive(InteractiveKind::CliInput);

        // Register CLI input provider for player
        handle.register_provider(cli_kind, CliActionProvider::new(rx_action))?;

        // Bind player to CLI input
        handle.bind_entity_provider(EntityId::PLAYER, cli_kind)?;

        // Subscribe to events
        let subscriptions = handle.subscribe_multiple(&[Topic::GameState, Topic::Proof]);
        let initial_state = handle.query_state().await?;

        // Initialize message log
        let mut messages = MessageLog::new(self.config.messages.capacity);
        messages.push_text(format!(
            "[{}] Welcome to the dungeon.",
            initial_state.turn.clock
        ));

        // Create event consumer
        let consumer =
            CliEventConsumer::new(messages, self.config.messages.effect_visibility.clone());

        // Create event loop
        let event_loop = EventLoop::new(
            subscriptions,
            tx_action,
            initial_state.entities.player().id,
            consumer,
            &initial_state,
            self.oracles.clone(),
            None, // Use default targeting strategy (ThreatBased)
            self.cli_config.clone(),
        );

        // Initialize terminal
        let mut terminal = terminal::init()?;
        let _guard = terminal::TerminalGuard;

        // Run event loop (blocks until user quits)
        let _consumer = event_loop.run(&mut terminal).await?;

        // Cleanup
        terminal::restore()?;
        tracing::info!("CLI frontend exiting");

        Ok(())
    }
}
