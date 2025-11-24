//! Blockchain client integration for runtime.
//!
//! Provides a unified container for blockchain-related clients (Sui, Walrus)
//! that are optionally available when the `sui` feature is enabled.

#[cfg(feature = "sui")]
use client_blockchain_sui::{SuiBlockchainClient, WalrusClient};

/// Container for blockchain clients.
///
/// This struct holds all blockchain-related clients (Sui for on-chain submission,
/// Walrus for blob storage) in a single Arc-sharable container.
///
/// Note: This struct does not implement Clone because the underlying clients
/// (SuiBlockchainClient, WalrusClient) do not implement Clone. Instead, this
/// struct is wrapped in Arc for shared ownership.
#[cfg(feature = "sui")]
pub struct BlockchainClients {
    pub sui: SuiBlockchainClient,
    pub walrus: WalrusClient,
}

#[cfg(feature = "sui")]
impl BlockchainClients {
    /// Create a new BlockchainClients container.
    pub fn new(sui: SuiBlockchainClient, walrus: WalrusClient) -> Self {
        Self { sui, walrus }
    }
}
