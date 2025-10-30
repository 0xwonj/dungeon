//! View-model layer for presentation.
//!
//! This module provides a stateful ViewModel that is incrementally updated
//! as events arrive, avoiding full state regeneration on every change.
//!
//! The ViewModel directly reuses `game-core` types (e.g., `StatsSnapshot`)
//! to ensure consistency between ZK proofs and UI rendering.

pub mod core;
pub mod entities;
pub mod map;
pub mod presentation;
pub mod turn;
pub mod world;

// Re-export core types
pub use self::core::ViewModel;
pub use entities::{ActorView, ItemView, PropView};
pub use map::{MapView, TileView};
pub use presentation::PresentationMapper;
pub use turn::TurnView;
pub use world::WorldSummary;
