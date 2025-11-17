//! Glue code tying the runtime, oracles, and terminal UI together.
use anyhow::Result;
use async_trait::async_trait;
use tokio::sync::mpsc;

use game_core::{Action, EntityId};
use runtime::{InteractiveKind, ProviderKind, Runtime, Topic};

use crate::event::{CliEventConsumer, EventLoop};
use crate::input::CliActionProvider;
use crate::presentation::terminal;
use client_bootstrap::{
    RuntimeConfig,
    builder::{RuntimeBuilder, RuntimeSetup},
    oracles::OracleBundle,
};
use client_frontend_core::{FrontendConfig, frontend::FrontendApp, message::MessageLog};

pub struct CliApp {
    runtime_config: RuntimeConfig,
    frontend_config: FrontendConfig,
    cli_config: crate::config::CliConfig,
    oracles: OracleBundle,
    runtime: Runtime,
}

pub struct CliAppBuilder {
    runtime_config: RuntimeConfig,
    frontend_config: FrontendConfig,
    cli_config: crate::config::CliConfig,
}

impl CliAppBuilder {
    pub fn new(
        runtime_config: RuntimeConfig,
        frontend_config: FrontendConfig,
        cli_config: crate::config::CliConfig,
    ) -> Self {
        Self {
            runtime_config,
            frontend_config,
            cli_config,
        }
    }

    pub async fn build(self) -> Result<CliApp> {
        let setup = RuntimeBuilder::new()
            .config(self.runtime_config.clone())
            .build()
            .await?;

        Ok(CliApp::from_runtime_setup(
            setup,
            self.frontend_config,
            self.cli_config,
        ))
    }
}

impl CliApp {
    pub fn builder(
        runtime_config: RuntimeConfig,
        frontend_config: FrontendConfig,
        cli_config: crate::config::CliConfig,
    ) -> CliAppBuilder {
        CliAppBuilder::new(runtime_config, frontend_config, cli_config)
    }

    fn from_runtime_setup(
        setup: RuntimeSetup,
        frontend_config: FrontendConfig,
        cli_config: crate::config::CliConfig,
    ) -> Self {
        let RuntimeSetup {
            config: runtime_config,
            oracles,
            runtime,
        } = setup;

        Self {
            runtime_config,
            frontend_config,
            cli_config,
            oracles,
            runtime,
        }
    }

    pub async fn execute(self) -> Result<()> {
        tracing::info!("CLI client starting...");

        // Setup CLI-specific provider (interactive input)
        let (tx_action, rx_action) =
            mpsc::channel::<Action>(self.frontend_config.channels.action_buffer);

        let handle = self.runtime.handle();
        let cli_kind = ProviderKind::Interactive(InteractiveKind::CliInput);

        // Register CLI input provider for player
        handle.register_provider(cli_kind, CliActionProvider::new(rx_action))?;

        // Bind player to CLI input
        handle.bind_entity_provider(EntityId::PLAYER, cli_kind)?;

        // Note: AI providers and default are already set up in RuntimeBuilder
        // NPCs will use the AggressiveAiProvider configured in bootstrap

        let CliApp {
            runtime_config: _runtime_config,
            frontend_config,
            cli_config,
            oracles,
            mut runtime,
        } = self;

        // Subscribe to topics that CLI needs (GameState and Proof)
        let handle = runtime.handle();
        let subscriptions = handle.subscribe_multiple(&[Topic::GameState, Topic::Proof]);
        let initial_state = handle.query_state().await?;

        let mut messages = MessageLog::new(frontend_config.messages.capacity);
        messages.push_text(format!(
            "[{}] Welcome to the dungeon.",
            initial_state.turn.clock
        ));

        let consumer =
            CliEventConsumer::new(messages, frontend_config.messages.effect_visibility.clone());

        let event_loop = EventLoop::new(
            subscriptions,
            tx_action,
            initial_state.entities.player().id,
            consumer,
            &initial_state,
            oracles.clone(),
            None, // Use default targeting strategy (ThreatBased)
            cli_config,
        );

        // Start runtime.run() BEFORE terminal init to avoid deadlock
        // EventLoop is already prepared to receive events, but terminal isn't initialized yet
        let runtime_task = tokio::spawn(async move {
            if let Err(e) = runtime.run().await {
                tracing::error!("Runtime error: {}", e);
            }
        });

        // Initialize terminal AFTER runtime starts so EventLoop can consume events immediately
        let mut terminal = terminal::init()?;
        let _guard = terminal::TerminalGuard;

        let _consumer = event_loop.run(&mut terminal).await?;

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
