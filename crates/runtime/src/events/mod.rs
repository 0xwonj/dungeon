//! Topic-based event bus for runtime events.
//!
//! This module provides a flexible event system where events are published to
//! specific topics, and consumers can subscribe only to the topics they need.

mod bus;
mod extractor;
mod game_event;
mod types;

pub use bus::{Event, EventBus, Topic};
pub use extractor::extract_events;
pub use game_event::{GameEvent, HealthThreshold};
pub use types::{ActionRef, GameStateEvent, ProofEvent};

// Re-export for backwards compatibility
pub use types::{ProofBackend, ProofData};
