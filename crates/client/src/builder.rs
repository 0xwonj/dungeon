//! Client builder with dependency injection pattern.

use crate::{BlockchainClient, Client, Frontend};
use anyhow::{Context, Result};

/// Builder for constructing a Client with proper validation.
///
/// # Design Principles
///
/// - **Required fields**: Runtime and Frontend must be provided
/// - **Optional fields**: Blockchain client is optional
/// - **Fail-fast validation**: Missing required fields cause build() to fail
/// - **Fluent API**: Chainable methods for ergonomic construction
#[derive(Default)]
pub struct ClientBuilder {
    runtime: Option<runtime::Runtime>,
    frontend: Option<Box<dyn Frontend>>,
    blockchain: Option<Box<dyn BlockchainClient>>,
}

impl ClientBuilder {
    /// Create a new ClientBuilder.
    pub fn new() -> Self {
        Self::default()
    }

    /// Set the runtime (required).
    ///
    /// The runtime handles game logic, state management, and event emission.
    /// It should be constructed via `RuntimeBuilder` from the `client-bootstrap` crate.
    pub fn runtime(mut self, runtime: runtime::Runtime) -> Self {
        self.runtime = Some(runtime);
        self
    }

    /// Set the frontend (required).
    ///
    /// The frontend handles UI rendering and user input. It receives a RuntimeHandle
    /// for communication with the game.
    pub fn frontend(mut self, frontend: impl Frontend + 'static) -> Self {
        self.frontend = Some(Box::new(frontend));
        self
    }

    /// Set the blockchain client (optional).
    ///
    /// If provided, the client will start a background worker to submit proofs
    /// to the blockchain. If not provided, proofs will only be stored locally.
    #[cfg(any(feature = "sui", feature = "ethereum"))]
    pub fn blockchain(mut self, client: impl BlockchainClient + 'static) -> Self {
        self.blockchain = Some(Box::new(client));
        self
    }

    /// Build the Client.
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - Runtime is not set (required)
    /// - Frontend is not set (required)
    pub fn build(self) -> Result<Client> {
        let runtime = self
            .runtime
            .context("Runtime is required. Use .runtime() to set it.")?;

        let frontend = self
            .frontend
            .context("Frontend is required. Use .frontend() to set it.")?;

        Ok(Client {
            runtime,
            frontend,
            blockchain: self.blockchain,
        })
    }
}
