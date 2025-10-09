//! Placeholder for the future prover worker.
//!
//! This worker will eventually offload zero-knowledge proof generation from the
//! main simulation loop. For now we stub the interface so other components can
//! depend on it without pulling in real proving logic yet.

/// Handles proof-generation tasks (to be implemented).
pub struct ProverWorker;

impl ProverWorker {
    /// Run the prover worker event loop.
    pub async fn run(self) {
        // TODO: wire prover task scheduling once the proving pipeline lands.
        tokio::task::yield_now().await;
    }
}
