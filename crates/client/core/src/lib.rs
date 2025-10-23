//! Cross-frontend primitives for presenting the game.
//!
//! Houses message logging, event handling, and view-model types that both CLI
//! and future graphical clients can reuse.
pub mod event;
pub mod frontend;
pub mod message;
pub mod targeting;
pub mod view_model;

pub use event::{EventConsumer, EventImpact};
