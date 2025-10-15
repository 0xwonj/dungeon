//! Cloneable façade for issuing commands to the runtime.
//!
//! [`RuntimeHandle`] hides channel plumbing and offers async helpers for
//! stepping the simulation or streaming events from specific topics.
use tokio::sync::{broadcast, mpsc, oneshot};

use game_core::{Action, EntityId, GameState};

use super::errors::{Result, RuntimeError};
use crate::events::{Event, EventBus, Topic};
use crate::workers::Command;

/// Client-facing handle to interact with the runtime
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
    /// - `Topic::Turn` - Turn management events
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// use runtime::Topic;
    ///
    /// // Only subscribe to game state events
    /// let mut game_rx = handle.subscribe(Topic::GameState);
    /// while let Ok(event) = game_rx.recv().await {
    ///     // Handle game state events
    /// }
    /// ```
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
}
