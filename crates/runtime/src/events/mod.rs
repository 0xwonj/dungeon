//! Topic-based event bus for runtime events.
//!
//! This module provides a flexible event system where events are published to
//! specific topics, and consumers can subscribe only to the topics they need.
//!
//! # Design
//!
//! - **Topics**: Event categories (GameState, Proof, Turn, etc.)
//! - **Type-safe**: Each topic has its own event type
//! - **Efficient**: Consumers only receive events they subscribed to
//! - **Extensible**: New topics can be added without breaking existing code
//!
//! # Example
//!
//! ```rust,ignore
//! // Subscribe to specific topics
//! let mut game_rx = event_bus.subscribe(Topic::GameState);
//! let mut proof_rx = event_bus.subscribe(Topic::Proof);
//!
//! // Only receive events you care about
//! while let Ok(event) = game_rx.recv().await {
//!     handle_game_event(event);
//! }
//! ```

mod bus;
mod types;

pub use bus::{Event, EventBus, Topic};
pub use types::{GameStateEvent, ProofEvent, TurnEvent};

// Re-export for backwards compatibility
pub use types::{ProofBackend, ProofData};
