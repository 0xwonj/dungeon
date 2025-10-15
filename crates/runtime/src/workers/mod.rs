//! Worker tasks that back the runtime orchestration.
//!
//! The simulation worker executes gameplay commands, while additional workers
//! (e.g., prover) can be added to offload specialized duties.

mod metrics;
mod prover;
mod simulation;

pub use metrics::ProofMetrics;
pub use prover::ProverWorker;
pub use simulation::{Command, SimulationWorker};
