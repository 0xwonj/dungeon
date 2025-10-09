//! Glue code tying the runtime, oracles, and terminal UI together.
use anyhow::Result;
use async_trait::async_trait;
use tokio::sync::mpsc;

use game_core::Action;
use runtime::Runtime;

use crate::input::CliActionProvider;
use crate::presentation::{CliEventConsumer, EventLoop, terminal};
use client_core::{
    bootstrap::{ClientBootstrap, RuntimeSetup},
    config::CliConfig,
    world::OracleBundle,
};
use frontend_core::{frontend::FrontendApp, message::MessageLog};

pub struct CliApp {
    config: CliConfig,
    oracles: OracleBundle,
    runtime: Runtime,
    tx_action: mpsc::Sender<Action>,
}

pub struct CliAppBuilder {
    bootstrap: ClientBootstrap,
}

impl CliAppBuilder {
    pub fn new(config: CliConfig) -> Self {
        Self {
            bootstrap: ClientBootstrap::new(config),
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
            mut runtime,
        } = setup;

        let (tx_action, rx_action) = mpsc::channel::<Action>(config.channels.action_buffer);
        runtime.set_player_provider(CliActionProvider::new(rx_action));

        Self {
            config,
            oracles,
            runtime,
            tx_action,
        }
    }

    pub async fn execute(self) -> Result<()> {
        tracing::info!("CLI client starting...");

        let mut terminal = terminal::init()?;
        let _guard = terminal::TerminalGuard;

        let CliApp {
            config,
            oracles,
            mut runtime,
            tx_action,
        } = self;

        let event_rx = runtime.subscribe_events();
        let handle = runtime.handle();
        let initial_state = handle.query_state().await?;

        let mut messages = MessageLog::new(config.messages.capacity);
        messages.push_text(format!(
            "[{}] Welcome to the dungeon.",
            initial_state.turn.clock.0
        ));

        let consumer = CliEventConsumer::new(messages);

        let event_loop = EventLoop::new(
            handle,
            event_rx,
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
