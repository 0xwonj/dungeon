//! Common types for blockchain interactions.

use serde::{Deserialize, Serialize};
use zk::ProofData;

/// Generic session identifier (blockchain-specific object ID or address).
///
/// Each blockchain uses its own format:
/// - Sui: ObjectID (32 bytes)
/// - Ethereum: Contract address (20 bytes)
/// - StarkNet: Felt (32 bytes)
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
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

/// Result of proof submission.
#[derive(Debug, Clone)]
pub struct SubmissionResult {
    /// Transaction ID on the blockchain
    pub transaction_id: TransactionId,

    /// Gas cost in native currency
    pub gas_cost: u64,

    /// Transaction status
    pub status: TransactionStatus,
}

/// Metadata about a proof pending submission.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProofMetadata {
    /// Action nonce from game state
    pub nonce: u64,

    /// Proof data from zkVM
    pub proof_data: ProofData,

    /// Estimated gas cost for submission
    pub estimated_gas: Option<u64>,

    /// Whether this proof has been submitted
    pub submitted: bool,

    /// Transaction ID if submitted
    pub transaction_id: Option<TransactionId>,
}

/// On-chain game session state.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionState {
    /// Session identifier
    pub session_id: SessionId,

    /// Oracle root commitment
    pub oracle_root: [u8; 32],

    /// Latest state root on-chain
    pub latest_state_root: [u8; 32],

    /// Latest action nonce on-chain
    pub latest_nonce: u64,

    /// Whether the session is finalized
    pub finalized: bool,
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
