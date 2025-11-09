//! Event handler implementations for EventLoop.
//!
//! This module contains handler methods organized by responsibility:
//! - `input`: Keyboard input and directional handling
//! - `action`: Action execution (slots, abilities, targeting)
//! - `targeting`: Auto-targeting and entity cycling
//! - `rendering`: Terminal rendering
//!
//! All handlers are implemented as `impl EventLoop` blocks in separate files,
//! and are automatically available to the EventLoop through Rust's module system.

mod action;
mod input;
mod rendering;
mod targeting;
