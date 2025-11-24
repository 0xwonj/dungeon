//! Top-level client orchestrating Runtime, Frontend, and Blockchain layers.
//!
//! # Architecture
//!
//! ```text
//! Client (Top-level container)
//!   ├─→ Runtime (Game logic and state management)
//!   ├─→ Frontend (UI layer - CLI, GUI, etc.)
//!   └─→ Blockchain (Optional - Proof submission)
//! ```
//!
//! # Separation of Concerns
//!
//! - **Client**: Composition root, lifecycle management, layer coordination
//! - **Runtime**: Pure game logic, deterministic state transitions, event emission
//! - **Frontend**: User interaction, event consumption, rendering (via RuntimeHandle only)
//! - **Blockchain**: Proof submission, transaction management (via RuntimeHandle only)
//!
//! # Design Principles
//!
//! - **Dependency Injection**: All layers injected into Client via builder
//! - **Trait-based Abstraction**: Frontend and Blockchain are traits for extensibility
//! - **Single Responsibility**: Each layer has one clear purpose
//! - **Testability**: Mock implementations can be injected for testing

mod builder;

pub use builder::ClientBuilder;

// Re-export Frontend trait from client-frontend-core
pub use client_frontend_core::Frontend;

use anyhow::Result;
use runtime::RuntimeHandle;

/// Top-level client container.
///
/// Orchestrates three independent layers:
/// - **Runtime**: Game state machine and event bus
/// - **Frontend**: UI rendering and user input (receives RuntimeHandle)
/// - **Blockchain**: Optional proof submission worker (receives RuntimeHandle)
///
/// # Lifecycle
///
/// 1. Client::builder() constructs layers independently
/// 2. Client::run() starts runtime worker in background
/// 3. Client::run() optionally starts blockchain worker
/// 4. Client::run() transfers control to frontend (blocking)
/// 5. On frontend exit, runtime and blockchain workers are cleaned up
pub struct Client {
    runtime: runtime::Runtime,
    frontend: Box<dyn Frontend>,
    blockchain: Option<Box<dyn BlockchainClient>>,
}

impl Client {
    /// Create a new ClientBuilder.
    pub fn builder() -> ClientBuilder {
        ClientBuilder::new()
    }

    /// Run the client.
    ///
    /// This method:
    /// 1. Starts the runtime worker in the background
    /// 2. Optionally starts the blockchain proof submission worker
    /// 3. Transfers control to the frontend (blocking until user quits)
    /// 4. Cleans up workers on exit
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - Runtime initialization fails
    /// - Frontend execution fails
    /// - Blockchain worker fails critically
    pub async fn run(self) -> Result<()> {
        let handle = self.runtime.handle();

        // Optional: Start blockchain proof submission worker
        let blockchain_task = if let Some(blockchain) = self.blockchain {
            let bc_handle = handle.clone();
            Some(tokio::spawn(async move {
                if let Err(e) = run_blockchain_worker(bc_handle, blockchain).await {
                    tracing::error!("Blockchain worker error: {}", e);
                }
            }))
        } else {
            None
        };

        // Start runtime in background
        let mut runtime = self.runtime;
        let frontend_handle = handle.clone();
        let runtime_task = tokio::spawn(async move {
            if let Err(e) = runtime.run().await {
                tracing::error!("Runtime error: {}", e);
            }
        });

        // Run frontend (blocks until user quits)
        let mut frontend = self.frontend;
        let frontend_result = frontend.run(frontend_handle).await;

        // Cleanup workers
        runtime_task.abort();
        let _ = runtime_task.await;

        if let Some(task) = blockchain_task {
            task.abort();
            let _ = task.await;
        }

        frontend_result
    }
}

/// Blockchain client trait for proof submission.
///
/// Each blockchain implementation (Sui, Ethereum, etc.) implements this trait.
pub trait BlockchainClient: Send + Sync {
    // TODO: Define common blockchain operations when needed
}

// Implement BlockchainClient for Sui
#[cfg(feature = "sui")]
impl BlockchainClient for client_blockchain_sui::SuiBlockchainClient {}

/// Background worker for blockchain proof submission.
///
/// Subscribes to Proof events from the runtime and submits them to the blockchain.
///
/// # Error Handling
///
/// Non-critical errors are logged. The worker only fails on critical errors
/// (e.g., complete loss of blockchain connectivity).
#[cfg(feature = "sui")]
async fn run_blockchain_worker(
    handle: RuntimeHandle,
    mut client: Box<dyn BlockchainClient>,
) -> Result<()> {
    use runtime::Topic;

    tracing::info!("Blockchain worker started");

    let mut proof_events = handle.subscribe(Topic::Proof);

    while let Ok(event) = proof_events.recv().await {
        // Extract proof data from event
        if let runtime::Event::Proof(runtime::ProofEvent::ProofGenerated { proof_data, .. }) = event
        {
            tracing::debug!("Submitting proof to blockchain");

            // Submit proof (non-blocking)
            if let Err(e) = submit_proof(&mut *client, proof_data).await {
                tracing::warn!("Failed to submit proof: {}", e);
                // Continue processing - proof submission failures are non-critical
            }
        }
    }

    tracing::info!("Blockchain worker stopped");
    Ok(())
}

/// Stub implementation when blockchain features are disabled.
#[cfg(not(feature = "sui"))]
async fn run_blockchain_worker(
    _handle: RuntimeHandle,
    _client: Box<dyn BlockchainClient>,
) -> Result<()> {
    tracing::warn!("Blockchain worker started but no blockchain features enabled");
    Ok(())
}

/// Submit a proof to the blockchain.
#[cfg(feature = "sui")]
async fn submit_proof(
    _client: &mut dyn BlockchainClient,
    _proof_data: zk::ProofData,
) -> Result<()> {
    // TODO: Implement proof submission
    // 1. Extract session ID from proof metadata
    // 2. Call blockchain client's submit method
    // 3. Handle transaction result

    tracing::warn!("Proof submission not yet implemented");
    Ok(())
}
