//! Sui blockchain client implementation.

use async_trait::async_trait;
use client_blockchain_core::{
    BlockchainClient, BlockchainConfig, BlockchainError, ProofMetadata, ProofSubmitter, Result,
    SessionId, SessionManager, SessionState, SubmissionResult, TransactionId, TransactionStatus,
};
use zk::ProofData;

use crate::config::SuiConfig;
use crate::converter::SuiProofConverter;

/// Sui blockchain client.
///
/// Implements the `BlockchainClient` trait for Sui blockchain.
pub struct SuiBlockchainClient {
    config: SuiConfig,
    // TODO: Add sui_sdk::SuiClient when SDK is available
    // sui_client: sui_sdk::SuiClient,
}

impl SuiBlockchainClient {
    /// Create a new Sui blockchain client.
    ///
    /// # Arguments
    ///
    /// * `config` - Sui-specific configuration
    ///
    /// # Returns
    ///
    /// Configured Sui client ready to submit proofs.
    pub async fn new(config: SuiConfig) -> Result<Self> {
        // Validate configuration
        config
            .validate()
            .map_err(|e| BlockchainError::ConfigError(e))?;

        // TODO: Initialize sui_sdk::SuiClient
        // let sui_client = sui_sdk::SuiClientBuilder::default()
        //     .build(config.rpc_url())
        //     .await
        //     .map_err(|e| BlockchainError::NetworkError(e.to_string()))?;

        Ok(Self {
            config,
            // sui_client,
        })
    }
}

#[async_trait]
impl ProofSubmitter for SuiBlockchainClient {
    async fn submit_proof(
        &self,
        _session_id: &SessionId,
        proof_data: ProofData,
    ) -> Result<SubmissionResult> {
        // Convert proof to Sui format
        let _sui_proof = SuiProofConverter::convert(proof_data)
            .map_err(|e| BlockchainError::InvalidProof(e.to_string()))?;

        // TODO: Build and submit transaction
        // let tx = self.build_verification_tx(session_id, sui_proof)?;
        // let response = self.sui_client.sign_and_execute_transaction(tx).await
        //     .map_err(|e| BlockchainError::TransactionFailed(e.to_string()))?;

        // Placeholder implementation
        tracing::warn!("SuiBlockchainClient::submit_proof not fully implemented yet");

        Ok(SubmissionResult {
            transaction_id: TransactionId::from_bytes(vec![0u8; 32]),
            gas_cost: 0,
            status: TransactionStatus::Pending,
        })
    }

    async fn submit_batch(
        &self,
        session_id: &SessionId,
        proofs: Vec<ProofData>,
    ) -> Result<Vec<SubmissionResult>> {
        // Sui doesn't support batch proof submission natively
        // Fall back to sequential submission
        let mut results = Vec::new();
        for proof in proofs {
            results.push(self.submit_proof(session_id, proof).await?);
        }
        Ok(results)
    }

    async fn estimate_gas(&self, _session_id: &SessionId, _proof_data: &ProofData) -> Result<u64> {
        // TODO: Implement gas estimation
        // This should call sui_client.dry_run_transaction()
        tracing::warn!("SuiBlockchainClient::estimate_gas not implemented yet");
        Ok(100_000) // Placeholder
    }

    async fn check_transaction(&self, _tx_id: &TransactionId) -> Result<TransactionStatus> {
        // TODO: Query transaction status from Sui
        tracing::warn!("SuiBlockchainClient::check_transaction not implemented yet");
        Ok(TransactionStatus::Pending)
    }
}

#[async_trait]
impl SessionManager for SuiBlockchainClient {
    async fn create_session(&self, _oracle_root: [u8; 32]) -> Result<SessionId> {
        // TODO: Call Sui smart contract to create session
        tracing::warn!("SuiBlockchainClient::create_session not implemented yet");

        // Placeholder: generate random session ID
        let session_bytes = vec![0u8; 32];
        Ok(SessionId::from_bytes(session_bytes))
    }

    async fn get_session_state(&self, session_id: &SessionId) -> Result<SessionState> {
        // TODO: Query session state from Sui
        tracing::warn!("SuiBlockchainClient::get_session_state not implemented yet");

        // Placeholder
        Ok(SessionState {
            session_id: session_id.clone(),
            oracle_root: [0u8; 32],
            latest_state_root: [0u8; 32],
            latest_nonce: 0,
            finalized: false,
        })
    }

    async fn finalize_session(&self, _session_id: &SessionId) -> Result<TransactionId> {
        // TODO: Call Sui smart contract to finalize session
        tracing::warn!("SuiBlockchainClient::finalize_session not implemented yet");

        Ok(TransactionId::from_bytes(vec![0u8; 32]))
    }

    async fn is_session_active(&self, session_id: &SessionId) -> Result<bool> {
        let state = self.get_session_state(session_id).await?;
        Ok(!state.finalized)
    }
}

#[async_trait]
impl BlockchainClient for SuiBlockchainClient {
    async fn list_pending_proofs(&self) -> Result<Vec<ProofMetadata>> {
        // TODO: Load pending proofs from local storage
        // This should query the ProofRepository
        tracing::warn!("SuiBlockchainClient::list_pending_proofs not implemented yet");
        Ok(Vec::new())
    }

    async fn submit_all_pending(&self, session_id: &SessionId) -> Result<Vec<SubmissionResult>> {
        let pending = self.list_pending_proofs().await?;

        if pending.is_empty() {
            return Ok(Vec::new());
        }

        let proofs: Vec<ProofData> = pending.into_iter().map(|p| p.proof_data).collect();

        self.submit_batch(session_id, proofs).await
    }

    fn config(&self) -> &dyn BlockchainConfig {
        &self.config
    }

    async fn health_check(&self) -> Result<()> {
        // TODO: Check connection to Sui network
        // self.sui_client.health_check().await
        //     .map_err(|e| BlockchainError::NetworkError(e.to_string()))?;

        tracing::info!("Health check: connected to {}", self.config.network_name());
        Ok(())
    }
}
