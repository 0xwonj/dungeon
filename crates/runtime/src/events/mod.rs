//! Topic-based event bus for runtime events.
//!
//! This module provides a flexible event system where events are published to
//! specific topics, and consumers can subscribe only to the topics they need.

mod bus;
mod types;

pub use bus::{Event, EventBus, Topic};
pub use types::{ActionRef, GameStateEvent, ProofEvent};

// Re-export for backwards compatibility
pub use types::{ProofBackend, ProofData};
