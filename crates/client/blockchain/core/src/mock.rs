//! Mock blockchain client for testing.

use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use async_trait::async_trait;
use zk::ProofData;

use crate::traits::{GameBlockchain, ProofError, ProofSubmitter, SessionError, SessionManager, StateError, StateVerifier};
use crate::types::{
    BlockchainConfig, GasEstimate, OnChainSession, ProofReceipt, SessionId, SessionStatus,
    StateRoot, TransactionId,
};

/// Mock blockchain client for testing without network.
///
/// Simulates blockchain operations in-memory.
#[derive(Clone)]
pub struct MockBlockchainClient {
    sessions: Arc<Mutex<HashMap<SessionId, OnChainSession>>>,
    transaction_counter: Arc<Mutex<u64>>,
}

impl MockBlockchainClient {
    pub fn new() -> Self {
        Self {
            sessions: Arc::new(Mutex::new(HashMap::new())),
            transaction_counter: Arc::new(Mutex::new(0)),
        }
    }

    fn next_tx_id(&self) -> TransactionId {
        let mut counter = self.transaction_counter.lock().unwrap();
        *counter += 1;
        TransactionId::from_bytes(counter.to_le_bytes().to_vec())
    }
}

impl Default for MockBlockchainClient {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl SessionManager for MockBlockchainClient {
    async fn create_session(
        &self,
        oracle_root: [u8; 32],
        initial_state_root: [u8; 32],
    ) -> Result<SessionId, SessionError> {
        let session_id = SessionId::from_bytes(oracle_root.to_vec());

        let session = OnChainSession {
            session_id: session_id.clone(),
            oracle_root,
            current_state_root: initial_state_root,
            nonce: 0,
            status: SessionStatus::Active,
            created_at: 0, // Mock timestamp
            finalized_at: None,
        };

        self.sessions
            .lock()
            .unwrap()
            .insert(session_id.clone(), session);

        Ok(session_id)
    }

    async fn get_session(&self, session_id: &SessionId) -> Result<OnChainSession, SessionError> {
        self.sessions
            .lock()
            .unwrap()
            .get(session_id)
            .cloned()
            .ok_or_else(|| SessionError::SessionNotFound(session_id.clone()))
    }

    async fn finalize_session(&self, session_id: &SessionId) -> Result<TransactionId, SessionError> {
        let mut sessions = self.sessions.lock().unwrap();
        let session = sessions
            .get_mut(session_id)
            .ok_or_else(|| SessionError::SessionNotFound(session_id.clone()))?;

        if session.status == SessionStatus::Finalized {
            return Err(SessionError::AlreadyFinalized);
        }

        session.status = SessionStatus::Finalized;
        session.finalized_at = Some(100); // Mock timestamp

        Ok(self.next_tx_id())
    }

    async fn is_session_active(&self, session_id: &SessionId) -> Result<bool, SessionError> {
        let sessions = self.sessions.lock().unwrap();
        Ok(sessions
            .get(session_id)
            .map(|s| s.status == SessionStatus::Active)
            .unwrap_or(false))
    }
}

#[async_trait]
impl ProofSubmitter for MockBlockchainClient {
    async fn submit_proof(
        &self,
        session_id: &SessionId,
        _proof: ProofData,
    ) -> Result<ProofReceipt, ProofError> {
        // Verify session exists and is active
        let mut sessions = self.sessions.lock().unwrap();
        let session = sessions
            .get_mut(session_id)
            .ok_or_else(|| ProofError::SessionNotFound(session_id.clone()))?;

        if session.status != SessionStatus::Active {
            return Err(ProofError::SessionInactive);
        }

        // Update session state
        session.nonce += 1;
        session.current_state_root = [1u8; 32]; // Mock new state root

        // Generate mock transaction
        let tx_id = self.next_tx_id();

        Ok(ProofReceipt {
            transaction_id: tx_id,
            gas_used: 1000, // Mock gas cost
            new_state_root: session.current_state_root,
            new_nonce: session.nonce,
        })
    }

    async fn estimate_proof_gas(
        &self,
        _session_id: &SessionId,
        _proof: &ProofData,
    ) -> Result<GasEstimate, ProofError> {
        Ok(GasEstimate {
            amount: 1000,
            unit: "MIST".to_string(),
            estimated_cost_usd: Some(0.001),
        })
    }
}

#[async_trait]
impl StateVerifier for MockBlockchainClient {
    async fn get_verified_state_root(&self, session_id: &SessionId) -> Result<StateRoot, StateError> {
        let sessions = self.sessions.lock().unwrap();
        let session = sessions
            .get(session_id)
            .ok_or_else(|| StateError::SessionNotFound(session_id.clone()))?;

        Ok(session.current_state_root)
    }

    async fn get_session_nonce(&self, session_id: &SessionId) -> Result<u64, StateError> {
        let sessions = self.sessions.lock().unwrap();
        let session = sessions
            .get(session_id)
            .ok_or_else(|| StateError::SessionNotFound(session_id.clone()))?;

        Ok(session.nonce)
    }
}

impl GameBlockchain for MockBlockchainClient {
    fn name(&self) -> &str {
        "MockBlockchain"
    }

    fn network(&self) -> &str {
        "mock-network"
    }
}

struct MockConfig;

impl BlockchainConfig for MockConfig {
    fn network_name(&self) -> &str {
        "mock-network"
    }

    fn rpc_url(&self) -> &str {
        "http://localhost:8545"
    }

    fn validate(&self) -> std::result::Result<(), String> {
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_mock_blockchain_client() {
        let client = MockBlockchainClient::new();

        // Create session
        let oracle_root = [1u8; 32];
        let initial_state = [2u8; 32];
        let session_id = client
            .create_session(oracle_root, initial_state)
            .await
            .unwrap();

        // Verify session exists
        let session = client.get_session(&session_id).await.unwrap();
        assert_eq!(session.oracle_root, oracle_root);
        assert_eq!(session.current_state_root, initial_state);
        assert_eq!(session.status, SessionStatus::Active);

        // Check session is active
        assert!(client.is_session_active(&session_id).await.unwrap());

        // Submit proof (mock)
        let proof_data = zk::ProofData {
            bytes: vec![1, 2, 3],
            backend: zk::ProofBackend::Stub,
            journal: vec![0u8; 168],
            journal_digest: [0u8; 32],
        };

        let receipt = client.submit_proof(&session_id, proof_data).await.unwrap();
        assert_eq!(receipt.gas_used, 1000);
        assert_eq!(receipt.new_nonce, 1);

        // Verify state was updated
        let state_root = client.get_verified_state_root(&session_id).await.unwrap();
        assert_eq!(state_root, receipt.new_state_root);

        let nonce = client.get_session_nonce(&session_id).await.unwrap();
        assert_eq!(nonce, 1);

        // Finalize session
        client.finalize_session(&session_id).await.unwrap();
        assert!(!client.is_session_active(&session_id).await.unwrap());

        // Verify GameBlockchain trait
        assert_eq!(client.name(), "MockBlockchain");
        assert_eq!(client.network(), "mock-network");
    }
}
