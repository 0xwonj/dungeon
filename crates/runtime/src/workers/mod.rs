//! Worker tasks that back the runtime orchestration.
//!
//! The simulation worker executes gameplay commands, while additional workers
//! (e.g., prover) can be added to offload specialized duties.

mod metrics;
pub mod persistence;
mod prover;
pub mod simulation;

pub use metrics::ProofMetrics;
pub use persistence::{CheckpointStrategy, PersistenceConfig, PersistenceWorker};
pub use prover::{ProverConfig, ProverWorker};
pub use simulation::{Command, SimulationWorker};
