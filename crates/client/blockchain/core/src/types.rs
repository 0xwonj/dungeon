//! Common types for blockchain interactions.
//!
//! This module defines blockchain-agnostic types used across all layers of the blockchain abstraction.

use serde::{Deserialize, Serialize};

/// Generic session identifier (blockchain-specific object ID or address).
///
/// Each blockchain uses its own format:
/// - Sui: ObjectID (32 bytes)
/// - Ethereum: Contract address (20 bytes)
/// - StarkNet: Felt (32 bytes)
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct SessionId(pub Vec<u8>);

impl SessionId {
    pub fn as_bytes(&self) -> &[u8] {
        &self.0
    }

    pub fn from_bytes(bytes: Vec<u8>) -> Self {
        Self(bytes)
    }
}

/// Generic transaction identifier.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TransactionId(pub Vec<u8>);

impl TransactionId {
    pub fn as_bytes(&self) -> &[u8] {
        &self.0
    }

    pub fn from_bytes(bytes: Vec<u8>) -> Self {
        Self(bytes)
    }
}

// ============================================================================
// Layer 0: Pure Infrastructure Types (BlockchainTransport)
// ============================================================================

/// Generic object identifier on the blockchain.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ObjectId(pub Vec<u8>);

impl ObjectId {
    pub fn as_bytes(&self) -> &[u8] {
        &self.0
    }

    pub fn from_bytes(bytes: Vec<u8>) -> Self {
        Self(bytes)
    }
}

/// Transaction data (opaque blockchain-specific bytes).
#[derive(Debug, Clone)]
pub struct TransactionData {
    pub payload: Vec<u8>,
}

/// Object data from blockchain (opaque bytes).
#[derive(Debug, Clone)]
pub struct ObjectData {
    pub data: Vec<u8>,
}

/// Transaction status on the blockchain.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum TransactionStatus {
    /// Transaction is pending in mempool
    Pending,

    /// Transaction is confirmed on-chain
    Confirmed { block_height: u64 },

    /// Transaction failed on-chain
    Failed { error: String },
}

// ============================================================================
// Layer 1: Game Domain Types (SessionManager, ProofSubmitter, StateVerifier)
// ============================================================================

/// State root (SHA-256 hash of game state).
pub type StateRoot = [u8; 32];

/// On-chain game session state.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OnChainSession {
    /// Session identifier
    pub session_id: SessionId,

    /// Oracle root commitment
    pub oracle_root: [u8; 32],

    /// Current verified state root
    pub current_state_root: StateRoot,

    /// Current action nonce
    pub nonce: u64,

    /// Session status
    pub status: SessionStatus,

    /// Creation timestamp
    pub created_at: u64,

    /// Finalization timestamp (if finalized)
    pub finalized_at: Option<u64>,
}

/// Session status.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SessionStatus {
    /// Session is active and accepting proofs
    Active,

    /// Session is finalized, no more proofs accepted
    Finalized,
}

/// Proof submission receipt.
#[derive(Debug, Clone)]
pub struct ProofReceipt {
    /// Transaction ID on the blockchain
    pub transaction_id: TransactionId,

    /// Gas used for this submission
    pub gas_used: u64,

    /// New state root after this proof
    pub new_state_root: StateRoot,

    /// New nonce after this proof
    pub new_nonce: u64,
}

/// Gas estimation result.
#[derive(Debug, Clone)]
pub struct GasEstimate {
    /// Estimated gas amount
    pub amount: u64,

    /// Gas unit (e.g., "MIST", "wei", "lamports")
    pub unit: String,

    /// Estimated cost in USD (if available)
    pub estimated_cost_usd: Option<f64>,
}

/// Blockchain-specific configuration.
///
/// This is a trait to allow different blockchains to provide their own config types.
pub trait BlockchainConfig: Send + Sync {
    /// Human-readable network name (e.g., "sui-testnet", "ethereum-mainnet")
    fn network_name(&self) -> &str;

    /// RPC endpoint URL
    fn rpc_url(&self) -> &str;

    /// Validate configuration (e.g., check credentials, network connectivity)
    fn validate(&self) -> Result<(), String>;
}
