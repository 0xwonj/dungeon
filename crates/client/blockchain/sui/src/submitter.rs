//! Sui proof submission client.
//!
//! Handles transaction construction and submission to Sui blockchain.

use sui_sdk::SuiClient;
use sui_sdk::types::base_types::ObjectID;
use sui_sdk::types::transaction::TransactionDigest;

use zk::ProofData;

use crate::SuiProof;
use crate::converter::SuiProofConverter;

/// Errors that can occur during proof submission.
#[derive(Debug, thiserror::Error)]
pub enum SubmissionError {
    #[error("Proof conversion failed: {0}")]
    ConversionError(#[from] crate::converter::ConversionError),

    #[error("Sui transaction failed: {0}")]
    SuiError(String),

    #[error("Invalid configuration: {0}")]
    ConfigError(String),
}

/// Client for submitting proofs to Sui blockchain.
///
/// This handles:
/// - Proof format conversion (SP1 → Sui)
/// - Transaction construction
/// - On-chain submission
///
/// # Example
///
/// ```ignore
/// use client_sui::SuiProofSubmitter;
/// use sui_sdk::SuiClientBuilder;
///
/// // Connect to Sui
/// let sui_client = SuiClientBuilder::default().build_testnet().await?;
/// let submitter = SuiProofSubmitter::new(sui_client).await?;
///
/// // Submit proof
/// let proof_data = load_proof_from_disk()?;
/// let tx_digest = submitter
///     .submit_proof(vk_object_id, proof_data)
///     .await?;
///
/// println!("Proof verified on-chain: {}", tx_digest);
/// ```
pub struct SuiProofSubmitter {
    client: SuiClient,
    package_id: ObjectID,
}

impl SuiProofSubmitter {
    /// Create a new Sui proof submitter.
    ///
    /// # Arguments
    ///
    /// * `client` - Configured Sui client
    ///
    /// # Returns
    ///
    /// Submitter ready to send transactions.
    pub async fn new(client: SuiClient) -> Result<Self, SubmissionError> {
        // TODO: Load package_id from config or environment
        let package_id = ObjectID::ZERO; // Placeholder

        Ok(Self { client, package_id })
    }

    /// Convert proof without submitting.
    ///
    /// Useful for testing or inspecting converted proof data.
    pub fn convert_proof(&self, proof_data: ProofData) -> Result<SuiProof, SubmissionError> {
        SuiProofConverter::convert(proof_data).map_err(Into::into)
    }

    /// Submit proof to Sui for verification.
    ///
    /// This method:
    /// 1. Converts ProofData to Sui format
    /// 2. Constructs a transaction calling `verify_game_proof()`
    /// 3. Signs and submits the transaction
    /// 4. Returns the transaction digest
    ///
    /// # Arguments
    ///
    /// * `vk_object_id` - On-chain VerifyingKey object ID
    /// * `proof_data` - SP1 proof data to submit
    ///
    /// # Returns
    ///
    /// Transaction digest if successful.
    ///
    /// # Errors
    ///
    /// Returns `SubmissionError` if conversion or submission fails.
    pub async fn submit_proof(
        &self,
        _vk_object_id: ObjectID,
        proof_data: ProofData,
    ) -> Result<TransactionDigest, SubmissionError> {
        // Convert proof to Sui format
        let _sui_proof = self.convert_proof(proof_data)?;

        // TODO: Construct and submit transaction
        // let tx = self.build_verification_tx(vk_object_id, sui_proof)?;
        // let response = self.client.sign_and_execute_transaction(tx).await?;

        todo!("Implement Sui transaction construction and submission")
    }

    /// Deploy verifying key to Sui (one-time setup).
    ///
    /// This should be called once to register the circuit's verifying key on-chain.
    ///
    /// # Arguments
    ///
    /// * `vk_bytes` - Arkworks-serialized verifying key
    /// * `version` - Circuit version identifier
    ///
    /// # Returns
    ///
    /// Object ID of the deployed VerifyingKey.
    pub async fn deploy_verifying_key(
        &self,
        _vk_bytes: Vec<u8>,
        _version: u64,
    ) -> Result<ObjectID, SubmissionError> {
        // TODO: Construct transaction calling create_verifying_key()
        todo!("Implement VK deployment")
    }
}
