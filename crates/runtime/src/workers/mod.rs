//! Worker tasks that back the runtime orchestration.
//!
//! The simulation worker executes gameplay commands, while additional workers
//! (e.g., prover) can be added to offload specialized duties.

#[allow(dead_code)]
mod prover;
mod simulation;

pub use simulation::{Command, SimulationWorker};
