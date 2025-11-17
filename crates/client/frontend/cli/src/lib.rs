//! Terminal UI frontend for Dungeon game.
//!
//! This crate provides a terminal-based user interface for the game.
//! It can be used standalone or composed with other features (like blockchain integration).

mod app;
mod config;
mod cursor;
mod event;
mod input;
pub mod logging;
mod presentation;
mod state;

pub use app::{CliApp, CliAppBuilder};
pub use config::CliConfig;

// Re-export for convenience
pub use client_bootstrap::RuntimeConfig;
pub use client_frontend_core::FrontendConfig;
