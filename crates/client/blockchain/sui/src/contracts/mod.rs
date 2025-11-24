//! Sui Move contract integrations.
//!
//! This module contains direct integrations with on-chain Move contracts.
//! Each contract is represented as a struct with methods corresponding to
//! on-chain function calls.

pub mod game_session;

// Re-export contract types
pub use game_session::{GameSession, GameSessionContract};
