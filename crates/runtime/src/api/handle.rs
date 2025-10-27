//! Cloneable façade for issuing commands to the runtime.
//!
//! [`RuntimeHandle`] hides channel plumbing and offers async helpers for
//! stepping the simulation or streaming events from specific topics.
use std::sync::{Arc, RwLock};

use tokio::sync::{broadcast, mpsc, oneshot};

use game_core::{Action, EntityId, GameState};

use super::errors::{Result, RuntimeError};
use super::{ActionProvider, ProviderKind, ProviderRegistry};
use crate::events::{Event, EventBus, Topic};
use crate::workers::Command;

/// Client-facing handle to interact with the runtime
///
/// # Concurrency Safety
///
/// Multiple clients can safely call methods concurrently. The underlying
/// [`SimulationWorker`] processes commands sequentially via a FIFO channel,
/// ensuring game state consistency without requiring explicit locks.
///
/// Provider methods use Arc<RwLock> for thread-safe access from both
/// Runtime::step() and external clients.
#[derive(Clone)]
pub struct RuntimeHandle {
    command_tx: mpsc::Sender<Command>,
    event_bus: EventBus,
    providers: Arc<RwLock<ProviderRegistry>>,
}

impl RuntimeHandle {
    pub(crate) fn new(
        command_tx: mpsc::Sender<Command>,
        event_bus: EventBus,
        providers: Arc<RwLock<ProviderRegistry>>,
    ) -> Self {
        Self {
            command_tx,
            event_bus,
            providers,
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

    // Provider management methods (synchronous - use Arc<RwLock>)

    /// Register a provider for a specific kind.
    ///
    /// The provider will be stored and can be used by entities bound to this kind.
    /// If a provider already exists for this kind, it will be replaced.
    pub fn register_provider(
        &self,
        kind: ProviderKind,
        provider: impl ActionProvider + 'static,
    ) -> Result<()> {
        let mut registry = self
            .providers
            .write()
            .map_err(|_| RuntimeError::LockPoisoned)?;
        registry.register(kind, provider);
        Ok(())
    }

    /// Bind an entity to a specific provider kind.
    ///
    /// The entity will use the provider registered for this kind.
    /// This can be called at runtime to switch an entity's AI or control method.
    pub fn bind_entity_provider(&self, entity: EntityId, kind: ProviderKind) -> Result<()> {
        let mut registry = self
            .providers
            .write()
            .map_err(|_| RuntimeError::LockPoisoned)?;
        registry.bind_entity(entity, kind);
        Ok(())
    }

    /// Unbind an entity, reverting it to the default provider.
    ///
    /// Returns the previous provider kind if the entity was explicitly bound.
    pub fn unbind_entity_provider(&self, entity: EntityId) -> Result<Option<ProviderKind>> {
        let mut registry = self
            .providers
            .write()
            .map_err(|_| RuntimeError::LockPoisoned)?;
        Ok(registry.unbind_entity(entity))
    }

    /// Set the default provider kind for unmapped entities.
    ///
    /// This is used as a fallback when an entity has no explicit binding.
    pub fn set_default_provider(&self, kind: ProviderKind) -> Result<()> {
        let mut registry = self
            .providers
            .write()
            .map_err(|_| RuntimeError::LockPoisoned)?;
        registry.set_default(kind);
        Ok(())
    }

    /// Get the provider kind for an entity.
    ///
    /// Returns the explicitly bound kind, or the default if not bound.
    pub fn get_entity_provider_kind(&self, entity: EntityId) -> Result<ProviderKind> {
        let registry = self
            .providers
            .read()
            .map_err(|_| RuntimeError::LockPoisoned)?;
        Ok(registry.get_entity_kind(entity))
    }

    /// Check if a provider is registered for a specific kind.
    pub fn is_provider_registered(&self, kind: ProviderKind) -> Result<bool> {
        let registry = self
            .providers
            .read()
            .map_err(|_| RuntimeError::LockPoisoned)?;
        Ok(registry.has(kind))
    }
}
