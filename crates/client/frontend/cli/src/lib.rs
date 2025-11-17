//! Terminal UI frontend for Dungeon game.
//!
//! This crate provides a terminal-based user interface for the game.
//! It implements the `dungeon_client::Frontend` trait for pure UI rendering.
//!
//! # Architecture
//!
//! CliFrontend is a pure UI layer that:
//! - Receives a RuntimeHandle for communication
//! - Does NOT own the Runtime
//! - Subscribes to events and submits actions via the handle

mod app;
mod config;
mod cursor;
mod event;
mod input;
pub mod logging;
mod presentation;
mod state;

pub use app::CliFrontend;
pub use config::CliConfig;

// Re-export for convenience (used in main.rs)
pub use client_frontend_core::FrontendConfig;
