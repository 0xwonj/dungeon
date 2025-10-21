//! Lightweight behavior tree library optimized for turn-based games.
//!
//! This library provides a minimal, deterministic behavior tree implementation
//! designed specifically for turn-based games and ZK-proof generation.
//!
//! - **No delta time**: Every tick completes immediately (turn-based semantics)
//! - **No Running state**: Actions either succeed or fail instantly
//! - **Minimal state**: Optimized for ZK circuit efficiency
//! - **Zero dependencies**: Pure Rust with no external crates
//!
//! # Architecture
//!
//! - [`Behavior`]: Core trait for all nodes
//! - [`Status`]: Success or Failure (no Running state)
//! - Composite nodes: [`Sequence`], [`Selector`]
//! - Decorator nodes: [`Inverter`], [`AlwaysSucceed`]

pub mod behavior;
pub mod builder;
pub mod composite;
pub mod decorator;
pub mod status;

// Re-export core types for ergonomic API
pub use behavior::Behavior;
pub use composite::{Selector, Sequence};
pub use decorator::{AlwaysSucceed, Inverter};
pub use status::Status;
