//! Event handling for CLI client.
//!
//! This module contains the event loop orchestrator and event consumer
//! that coordinate runtime events, user input, and UI updates.

mod consumer;
mod handlers;
mod r#loop;

pub use consumer::CliEventConsumer;
pub use r#loop::EventLoop;
