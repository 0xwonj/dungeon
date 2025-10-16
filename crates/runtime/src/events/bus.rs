//! Topic-based event bus implementation.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::{RwLock, broadcast};

use super::types::{ActionRef, GameStateEvent, ProofEvent, TurnEvent};

/// Topics for event routing
#[derive(Debug, Clone, Copy, Hash, Eq, PartialEq, Serialize, Deserialize)]
pub enum Topic {
    /// Game state changes (actions, failures)
    GameState,
    /// ZK proof events
    Proof,
    /// Turn management events
    Turn,
}

/// Event wrapper that carries the topic and typed event
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Event {
    GameState(GameStateEvent),
    Proof(ProofEvent),
    Turn(TurnEvent),

    /// Reference to an action in the actions.log file.
    ///
    /// This variant is used in events.log to maintain the complete event timeline
    /// without duplicating the full action data. The full `ActionLogEntry` can be
    /// retrieved from actions.log using the `action_offset`.
    ActionRef(ActionRef),
}

impl Event {
    pub fn topic(&self) -> Topic {
        match self {
            Event::GameState(_) => Topic::GameState,
            Event::Proof(_) => Topic::Proof,
            Event::Turn(_) => Topic::Turn,
            Event::ActionRef(_) => Topic::GameState, // ActionRef belongs to GameState topic
        }
    }
}

/// Topic-based event bus
///
/// Allows consumers to subscribe to specific topics and only receive
/// events they care about.
pub struct EventBus {
    channels: Arc<RwLock<HashMap<Topic, broadcast::Sender<Event>>>>,
}

impl EventBus {
    /// Creates a new event bus with default capacity for each topic
    pub fn new() -> Self {
        Self::with_capacity(100)
    }

    /// Creates a new event bus with specified capacity per topic
    pub fn with_capacity(capacity: usize) -> Self {
        let mut channels = HashMap::new();

        // Pre-create channels for each topic
        channels.insert(Topic::GameState, broadcast::channel(capacity).0);
        channels.insert(Topic::Proof, broadcast::channel(capacity).0);
        channels.insert(Topic::Turn, broadcast::channel(capacity).0);

        Self {
            channels: Arc::new(RwLock::new(channels)),
        }
    }

    /// Publish an event to its corresponding topic
    pub fn publish(&self, event: Event) {
        let topic = event.topic();

        // Use try_read to avoid blocking in async context
        // If we can't get the lock, just skip (events are best-effort)
        match self.channels.try_read() {
            Ok(channels) => {
                if let Some(tx) = channels.get(&topic)
                    && tx.send(event).is_err()
                {
                    // No subscribers for this topic - this is normal, not an error
                    tracing::trace!("No subscribers for topic {:?}", topic);
                }
            }
            Err(_) => {
                // Failed to acquire lock - event bus is likely under heavy contention
                // This is best-effort, so we skip the event
                tracing::debug!("Failed to acquire event bus lock for topic {:?}", topic);
            }
        }
    }

    /// Subscribe to a specific topic
    ///
    /// Returns a receiver that will only receive events for that topic.
    pub fn subscribe(&self, topic: Topic) -> broadcast::Receiver<Event> {
        let channels = self
            .channels
            .try_read()
            .expect("Failed to acquire read lock on event channels");
        channels
            .get(&topic)
            .expect("Topic channel not initialized")
            .subscribe()
    }

    /// Subscribe to multiple topics
    ///
    /// Returns receivers for each requested topic.
    pub fn subscribe_multiple(
        &self,
        topics: &[Topic],
    ) -> HashMap<Topic, broadcast::Receiver<Event>> {
        let channels = self
            .channels
            .try_read()
            .expect("Failed to acquire read lock on event channels");
        topics
            .iter()
            .map(|&topic| {
                let rx = channels
                    .get(&topic)
                    .expect("Topic channel not initialized")
                    .subscribe();
                (topic, rx)
            })
            .collect()
    }
}

impl Clone for EventBus {
    fn clone(&self) -> Self {
        Self {
            channels: Arc::clone(&self.channels),
        }
    }
}

impl Default for EventBus {
    fn default() -> Self {
        Self::new()
    }
}
