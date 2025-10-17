//! Cloneable façade for issuing commands to the runtime.
//!
//! [`RuntimeHandle`] hides channel plumbing and offers async helpers for
//! stepping the simulation or streaming events from specific topics.
use tokio::sync::{broadcast, mpsc, oneshot};

use game_core::{Action, EntityId, GameState};

use super::errors::{Result, RuntimeError};
use super::{ActionProvider, ProviderKind};
use crate::events::{Event, EventBus, Topic};
use crate::workers::Command;

/// Client-facing handle to interact with the runtime
///
/// # Concurrency Safety
///
/// Multiple clients can safely call methods concurrently. The underlying
/// [`SimulationWorker`] processes commands sequentially via a FIFO channel,
/// ensuring game state consistency without requiring explicit locks.
#[derive(Clone)]
pub struct RuntimeHandle {
    command_tx: mpsc::Sender<Command>,
    event_bus: EventBus,
}

impl RuntimeHandle {
    pub(crate) fn new(command_tx: mpsc::Sender<Command>, event_bus: EventBus) -> Self {
        Self {
            command_tx,
            event_bus,
        }
    }

    /// Prepare the next turn - determines which entity acts next and returns game state clone
    pub async fn prepare_next_turn(&self) -> Result<(EntityId, GameState)> {
        let (reply_tx, reply_rx) = oneshot::channel();

        self.command_tx
            .send(Command::PrepareNextTurn { reply: reply_tx })
            .await
            .map_err(|_| RuntimeError::CommandChannelClosed)?;

        reply_rx.await.map_err(RuntimeError::ReplyChannelClosed)?
    }

    /// Execute an action for the current turn entity
    pub async fn execute_action(&self, action: Action) -> Result<()> {
        let (reply_tx, reply_rx) = oneshot::channel();

        self.command_tx
            .send(Command::ExecuteAction {
                action,
                reply: reply_tx,
            })
            .await
            .map_err(|_| RuntimeError::CommandChannelClosed)?;

        reply_rx.await.map_err(RuntimeError::ReplyChannelClosed)?
    }

    /// Subscribe to events from a specific topic
    ///
    /// # Topics
    ///
    /// - `Topic::GameState` - Action execution and failures
    /// - `Topic::Proof` - ZK proof generation events
    pub fn subscribe(&self, topic: Topic) -> broadcast::Receiver<Event> {
        self.event_bus.subscribe(topic)
    }

    /// Subscribe to multiple topics at once
    ///
    /// Returns a map of topic to receiver for each requested topic.
    pub fn subscribe_multiple(
        &self,
        topics: &[Topic],
    ) -> std::collections::HashMap<Topic, broadcast::Receiver<Event>> {
        self.event_bus.subscribe_multiple(topics)
    }

    /// Query the current game state (read-only snapshot)
    pub async fn query_state(&self) -> Result<GameState> {
        let (reply_tx, reply_rx) = oneshot::channel();

        self.command_tx
            .send(Command::QueryState { reply: reply_tx })
            .await
            .map_err(|_| RuntimeError::CommandChannelClosed)?;

        reply_rx.await.map_err(RuntimeError::ReplyChannelClosed)
    }

    /// Get a reference to the event bus for advanced usage
    pub fn event_bus(&self) -> &EventBus {
        &self.event_bus
    }

    // Provider management methods

    /// Register a provider for a specific kind.
    ///
    /// The provider will be stored and can be used by entities bound to this kind.
    /// If a provider already exists for this kind, it will be replaced.
    ///
    /// # Example
    ///
    /// ```ignore
    /// use runtime::{ProviderKind, AiKind};
    ///
    /// handle.register_provider(
    ///     ProviderKind::Ai(AiKind::Aggressive),
    ///     AggressiveAI::new()
    /// ).await?;
    /// ```
    pub async fn register_provider(
        &self,
        kind: ProviderKind,
        provider: impl ActionProvider + 'static,
    ) -> Result<()> {
        let (reply_tx, reply_rx) = oneshot::channel();

        self.command_tx
            .send(Command::RegisterProvider {
                kind,
                provider: Box::new(provider),
                reply: reply_tx,
            })
            .await
            .map_err(|_| RuntimeError::CommandChannelClosed)?;

        reply_rx.await.map_err(RuntimeError::ReplyChannelClosed)?
    }

    /// Bind an entity to a specific provider kind.
    ///
    /// The entity will use the provider registered for this kind.
    /// This can be called at runtime to switch an entity's AI or control method.
    ///
    /// # Example
    ///
    /// ```ignore
    /// // Switch NPC to aggressive AI
    /// handle.bind_entity_provider(
    ///     npc_id,
    ///     ProviderKind::Ai(AiKind::Aggressive)
    /// ).await?;
    ///
    /// // Enable auto-pilot for player
    /// handle.bind_entity_provider(
    ///     EntityId::PLAYER,
    ///     ProviderKind::Ai(AiKind::Scripted)
    /// ).await?;
    /// ```
    pub async fn bind_entity_provider(
        &self,
        entity: EntityId,
        kind: ProviderKind,
    ) -> Result<()> {
        let (reply_tx, reply_rx) = oneshot::channel();

        self.command_tx
            .send(Command::BindEntityProvider {
                entity,
                kind,
                reply: reply_tx,
            })
            .await
            .map_err(|_| RuntimeError::CommandChannelClosed)?;

        reply_rx.await.map_err(RuntimeError::ReplyChannelClosed)?
    }

    /// Unbind an entity, reverting it to the default provider.
    ///
    /// Returns the previous provider kind if the entity was explicitly bound.
    pub async fn unbind_entity_provider(
        &self,
        entity: EntityId,
    ) -> Result<Option<ProviderKind>> {
        let (reply_tx, reply_rx) = oneshot::channel();

        self.command_tx
            .send(Command::UnbindEntityProvider {
                entity,
                reply: reply_tx,
            })
            .await
            .map_err(|_| RuntimeError::CommandChannelClosed)?;

        reply_rx.await.map_err(RuntimeError::ReplyChannelClosed)?
    }

    /// Set the default provider kind for unmapped entities.
    ///
    /// This is used as a fallback when an entity has no explicit binding.
    pub async fn set_default_provider(&self, kind: ProviderKind) -> Result<()> {
        let (reply_tx, reply_rx) = oneshot::channel();

        self.command_tx
            .send(Command::SetDefaultProvider {
                kind,
                reply: reply_tx,
            })
            .await
            .map_err(|_| RuntimeError::CommandChannelClosed)?;

        reply_rx.await.map_err(RuntimeError::ReplyChannelClosed)?
    }

    /// Get the provider kind for an entity.
    ///
    /// Returns the explicitly bound kind, or the default if not bound.
    pub async fn get_entity_provider_kind(&self, entity: EntityId) -> Result<ProviderKind> {
        let (reply_tx, reply_rx) = oneshot::channel();

        self.command_tx
            .send(Command::GetEntityProviderKind {
                entity,
                reply: reply_tx,
            })
            .await
            .map_err(|_| RuntimeError::CommandChannelClosed)?;

        reply_rx.await.map_err(RuntimeError::ReplyChannelClosed)?
    }

    /// Check if a provider is registered for a specific kind.
    pub async fn is_provider_registered(&self, kind: ProviderKind) -> Result<bool> {
        let (reply_tx, reply_rx) = oneshot::channel();

        self.command_tx
            .send(Command::IsProviderRegistered {
                kind,
                reply: reply_tx,
            })
            .await
            .map_err(|_| RuntimeError::CommandChannelClosed)?;

        reply_rx.await.map_err(RuntimeError::ReplyChannelClosed)?
    }

    /// Execute a complete turn step.
    ///
    /// This is a high-level helper that:
    /// 1. Prepares the next turn (determines which entity acts)
    /// 2. Queries the entity's provider for an action
    /// 3. Executes the action
    ///
    /// This encapsulates the entire game loop cycle in a single command.
    ///
    /// # Example
    ///
    /// ```ignore
    /// // Simple game loop
    /// loop {
    ///     handle.execute_turn_step().await?;
    /// }
    /// ```
    pub async fn execute_turn_step(&self) -> Result<()> {
        let (reply_tx, reply_rx) = oneshot::channel();

        self.command_tx
            .send(Command::ExecuteTurnStep { reply: reply_tx })
            .await
            .map_err(|_| RuntimeError::CommandChannelClosed)?;

        reply_rx.await.map_err(RuntimeError::ReplyChannelClosed)?
    }
}
