//! Worker tasks that back the runtime orchestration.
//!
//! The simulation worker executes gameplay commands, while additional workers
//! (e.g., prover) can be added to offload specialized duties.

mod prover;
mod simulation;

pub use prover::ProverWorker;
pub use simulation::{Command, SimulationWorker};
