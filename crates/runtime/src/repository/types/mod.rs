//! Shared types for repository layer.

mod action_log;
mod checkpoint;

pub use action_log::ActionLogEntry;
pub use checkpoint::{Checkpoint, EventReference, ProofReference, StateReference};
