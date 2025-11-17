//! Cross-frontend primitives for presenting the game.
//!
//! Houses message logging, event handling, and view-model types that both CLI
//! and future graphical clients can reuse.
pub mod event;
pub mod format;
pub mod frontend;
pub mod message;
pub mod services;
pub mod view_model;

pub use event::{EventConsumer, EventImpact};
pub use services::{UpdateScope, ViewModelUpdater, targeting};
