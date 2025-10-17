//! Shared types for repository layer.

mod action_log;
mod checkpoint;
mod proof_index;

pub use action_log::ActionLogEntry;
pub use checkpoint::{Checkpoint, EventReference, ProofReference, StateReference};
pub use proof_index::{ProofEntry, ProofIndex};
