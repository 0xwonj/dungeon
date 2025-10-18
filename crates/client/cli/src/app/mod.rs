//! Glue code tying the runtime, oracles, and terminal UI together.
use anyhow::Result;
use async_trait::async_trait;
use tokio::sync::mpsc;

use game_core::{Action, EntityId};
use runtime::{AiKind, InteractiveKind, ProviderKind, Runtime, Topic, WaitActionProvider};

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
    tx_action: mpsc::Sender<Action>,
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

        let (tx_action, _rx_action) = mpsc::channel::<Action>(config.channels.action_buffer);

        Self {
            config,
            oracles,
            runtime,
            tx_action,
        }
    }

    pub async fn execute(mut self) -> Result<()> {
        tracing::info!("CLI client starting...");

        // Setup providers before starting
        let (tx_action_new, rx_action) =
            mpsc::channel::<Action>(self.config.channels.action_buffer);
        self.tx_action = tx_action_new;

        let handle = self.runtime.handle();
        let cli_kind = ProviderKind::Interactive(InteractiveKind::CliInput);
        let wait_kind = ProviderKind::Ai(AiKind::Wait);

        // Register provider instances (now synchronous)
        handle.register_provider(cli_kind, CliActionProvider::new(rx_action))?;
        handle.register_provider(wait_kind, WaitActionProvider)?;

        // Bind player to CLI input (now synchronous)
        handle.bind_entity_provider(EntityId::PLAYER, cli_kind)?;

        // Set default to Wait for unmapped entities (NPCs, now synchronous)
        handle.set_default_provider(wait_kind)?;

        let mut terminal = terminal::init()?;
        let _guard = terminal::TerminalGuard;

        let CliApp {
            config,
            oracles,
            mut runtime,
            tx_action,
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
