//! Glue code tying the runtime, oracles, and terminal UI together.
use anyhow::Result;
use async_trait::async_trait;
use tokio::sync::mpsc;

use game_core::{Action, EntityId};
use runtime::{InteractiveKind, ProviderKind, Runtime, Topic};

use crate::input::CliActionProvider;
use crate::presentation::{CliEventConsumer, EventLoop, terminal};
use client_bootstrap::{
    builder::{RuntimeBuilder, RuntimeSetup},
    config::CliConfig,
    oracles::OracleBundle,
};
use client_core::{frontend::FrontendApp, message::MessageLog};

pub struct CliApp {
    config: CliConfig,
    oracles: OracleBundle,
    runtime: Runtime,
}

pub struct CliAppBuilder {
    bootstrap: RuntimeBuilder,
}

impl CliAppBuilder {
    pub fn new(config: CliConfig) -> Self {
        Self {
            bootstrap: RuntimeBuilder::new(config),
        }
    }

    pub async fn build(self) -> Result<CliApp> {
        let setup = self.bootstrap.build().await?;
        Ok(CliApp::from_runtime_setup(setup))
    }
}

impl CliApp {
    pub fn builder(config: CliConfig) -> CliAppBuilder {
        CliAppBuilder::new(config)
    }

    fn from_runtime_setup(setup: RuntimeSetup) -> Self {
        let RuntimeSetup {
            config,
            oracles,
            runtime,
        } = setup;

        Self {
            config,
            oracles,
            runtime,
        }
    }

    pub async fn execute(self) -> Result<()> {
        tracing::info!("CLI client starting...");

        // Setup CLI-specific provider (interactive input)
        let (tx_action, rx_action) = mpsc::channel::<Action>(self.config.channels.action_buffer);

        let handle = self.runtime.handle();
        let cli_kind = ProviderKind::Interactive(InteractiveKind::CliInput);

        // Register CLI input provider for player
        handle.register_provider(cli_kind, CliActionProvider::new(rx_action))?;

        // Bind player to CLI input
        handle.bind_entity_provider(EntityId::PLAYER, cli_kind)?;

        // Note: AI providers and default are already set up in RuntimeBuilder
        // NPCs will use the AggressiveAiProvider configured in bootstrap

        let mut terminal = terminal::init()?;
        let _guard = terminal::TerminalGuard;

        let CliApp {
            config,
            oracles,
            mut runtime,
        } = self;

        // Subscribe to topics that CLI needs (GameState and Proof)
        let handle = runtime.handle();
        let subscriptions = handle.subscribe_multiple(&[Topic::GameState, Topic::Proof]);
        let initial_state = handle.query_state().await?;

        let mut messages = MessageLog::new(config.messages.capacity);
        messages.push_text(format!(
            "[{}] Welcome to the dungeon.",
            initial_state.turn.clock
        ));

        let consumer = CliEventConsumer::new(messages);

        let event_loop = EventLoop::new(
            handle,
            subscriptions,
            tx_action,
            initial_state.entities.player.id,
            consumer,
        );

        let runtime_task = tokio::spawn(async move {
            if let Err(e) = runtime.run().await {
                tracing::error!("Runtime error: {}", e);
            }
        });

        let _consumer = event_loop
            .run(&mut terminal, oracles.map.as_ref(), initial_state)
            .await?;

        runtime_task.abort();
        let _ = runtime_task.await;

        terminal::restore()?;
        tracing::info!("CLI client exiting");

        Ok(())
    }
}

#[async_trait]
impl FrontendApp for CliApp {
    async fn run(self) -> Result<()> {
        self.execute().await
    }
}
