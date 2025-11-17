//! Sui blockchain client implementation.
//!
//! NOTE: This is a work-in-progress implementation. The Sui SDK API is still evolving
//! and some methods are stubbed with todo!() placeholders.

use std::collections::HashMap;
use std::sync::Arc;

use anyhow::{Result, anyhow};
use async_trait::async_trait;
use tokio::sync::RwLock;
use zk::ProofData;

use client_blockchain_core::{
    BlockchainClient, GasEstimate, OnChainSession, ProofError, ProofReceipt, ProofSubmitter,
    SessionError, SessionId, SessionManager, SessionStatus, StateError, StateRoot, StateVerifier,
    TransactionId,
};

use crate::config::SuiConfig;
use crate::converter::SuiProofConverter;

/// Sui blockchain client.
///
/// Provides Sui network integration for game session management,
/// ZK proof submission, and state verification.
///
/// NOTE: This implementation is currently stubbed. Full Sui SDK integration
/// requires matching the latest Sui API changes.
pub struct SuiBlockchainClient {
    /// Sui configuration
    config: SuiConfig,

    /// In-memory session cache (SessionId → internal data)
    sessions: Arc<RwLock<HashMap<SessionId, OnChainSession>>>,
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
    /// Configured Sui client ready to interact with network.
    ///
    /// # Errors
    ///
    /// Returns error if configuration is invalid.
    pub async fn new(config: SuiConfig) -> Result<Self> {
        // Validate configuration
        config
            .validate()
            .map_err(|e| anyhow!("Invalid configuration: {}", e))?;

        Ok(Self {
            config,
            sessions: Arc::new(RwLock::new(HashMap::new())),
        })
    }
}

#[async_trait]
impl SessionManager for SuiBlockchainClient {
    async fn create_session(
        &self,
        oracle_root: [u8; 32],
        initial_state_root: [u8; 32],
    ) -> Result<SessionId, SessionError> {
        // TODO: Implement real Sui transaction calling game_session::create
        // This requires:
        // 1. Building Programmable Transaction Block
        // 2. Calling package_id::game_session::create(oracle_root, initial_state_root, seed_commitment)
        // 3. Signing and executing transaction
        // 4. Parsing created GameSession object ID from response

        tracing::warn!(
            "SuiBlockchainClient::create_session is stubbed - implement with real Sui SDK"
        );

        // Placeholder: create mock session
        let session_id = SessionId::from_bytes(vec![0u8; 32]);
        let session = OnChainSession {
            session_id: session_id.clone(),
            oracle_root,
            current_state_root: initial_state_root,
            nonce: 0,
            status: SessionStatus::Active,
            created_at: 0,
            finalized_at: None,
        };

        {
            let mut sessions = self.sessions.write().await;
            sessions.insert(session_id.clone(), session);
        }

        Ok(session_id)
    }

    async fn get_session(&self, session_id: &SessionId) -> Result<OnChainSession, SessionError> {
        // TODO: Implement real Sui object query
        // This requires:
        // 1. Converting SessionId to ObjectID
        // 2. Querying object with get_object_with_options
        // 3. Parsing Move struct fields
        // 4. Extracting oracle_root, state_root, nonce, finalized

        tracing::warn!("SuiBlockchainClient::get_session is stubbed - using in-memory cache");

        let sessions = self.sessions.read().await;
        sessions
            .get(session_id)
            .cloned()
            .ok_or_else(|| SessionError::SessionNotFound(session_id.clone()))
    }

    async fn finalize_session(
        &self,
        session_id: &SessionId,
    ) -> Result<TransactionId, SessionError> {
        // TODO: Implement real Sui transaction calling game_session::finalize
        // This requires:
        // 1. Building Programmable Transaction Block
        // 2. Calling package_id::game_session::finalize(session)
        // 3. Signing and executing transaction
        // 4. Returning transaction digest

        tracing::warn!("SuiBlockchainClient::finalize_session is stubbed");

        {
            let mut sessions = self.sessions.write().await;
            if let Some(session) = sessions.get_mut(session_id) {
                session.status = SessionStatus::Finalized;
                session.finalized_at = Some(0);
            }
        }

        Ok(TransactionId::from_bytes(vec![0u8; 32]))
    }

    async fn is_session_active(&self, session_id: &SessionId) -> Result<bool, SessionError> {
        let session = self.get_session(session_id).await?;
        Ok(session.status == SessionStatus::Active)
    }
}

#[async_trait]
impl ProofSubmitter for SuiBlockchainClient {
    async fn submit_proof(
        &self,
        session_id: &SessionId,
        proof: ProofData,
    ) -> Result<ProofReceipt, ProofError> {
        // Convert proof to Sui format
        let sui_proof = SuiProofConverter::convert(proof)
            .map_err(|e| ProofError::InvalidProof(e.to_string()))?;

        // TODO: Implement real Sui transaction calling game_session::update
        // This requires:
        // 1. Extracting new_state_root and new_nonce from journal
        // 2. Building Programmable Transaction Block
        // 3. Calling package_id::game_session::update(session, vk, proof, new_state_root, new_nonce, actions_blob)
        // 4. Signing and executing transaction
        // 5. Parsing gas cost from effects

        tracing::warn!("SuiBlockchainClient::submit_proof is stubbed");

        // Extract values from journal for mock receipt
        let fields = zk::parse_journal(&sui_proof.journal)
            .map_err(|e| ProofError::InvalidProof(e.to_string()))?;

        let new_state_root = fields.new_state_root;
        let new_nonce = fields.new_nonce;

        // Update session in cache
        {
            let mut sessions = self.sessions.write().await;
            if let Some(session) = sessions.get_mut(session_id) {
                session.current_state_root = new_state_root;
                session.nonce = new_nonce;
            }
        }

        Ok(ProofReceipt {
            transaction_id: TransactionId::from_bytes(vec![0u8; 32]),
            gas_used: 100_000, // Placeholder
            new_state_root,
            new_nonce,
        })
    }

    async fn estimate_proof_gas(
        &self,
        _session_id: &SessionId,
        _proof: &ProofData,
    ) -> Result<GasEstimate, ProofError> {
        // TODO: Implement real gas estimation with dry run
        tracing::warn!("SuiBlockchainClient::estimate_proof_gas is stubbed");

        Ok(GasEstimate {
            amount: self.config.gas_budget,
            unit: "MIST".to_string(),
            estimated_cost_usd: None,
        })
    }
}

#[async_trait]
impl StateVerifier for SuiBlockchainClient {
    async fn get_verified_state_root(
        &self,
        session_id: &SessionId,
    ) -> Result<StateRoot, StateError> {
        let session = self.get_session(session_id).await.map_err(|e| match e {
            SessionError::SessionNotFound(id) => StateError::SessionNotFound(id),
            _ => StateError::InvalidData(e.to_string()),
        })?;

        Ok(session.current_state_root)
    }

    async fn get_session_nonce(&self, session_id: &SessionId) -> Result<u64, StateError> {
        let session = self.get_session(session_id).await.map_err(|e| match e {
            SessionError::SessionNotFound(id) => StateError::SessionNotFound(id),
            _ => StateError::InvalidData(e.to_string()),
        })?;

        Ok(session.nonce)
    }
}

impl BlockchainClient for SuiBlockchainClient {
    fn name(&self) -> &str {
        "Sui"
    }

    fn network(&self) -> &str {
        match self.config.network {
            crate::config::SuiNetwork::Mainnet => "mainnet",
            crate::config::SuiNetwork::Testnet => "testnet",
            crate::config::SuiNetwork::Local => "local",
        }
    }
}
