//! Mock blockchain client for testing.

use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use async_trait::async_trait;
use zk::ProofData;

use crate::traits::{BlockchainClient, ProofSubmitter, Result, SessionManager};
use crate::types::{
    BlockchainConfig, ProofMetadata, SessionId, SessionState, SubmissionResult, TransactionId,
    TransactionStatus,
};

/// Mock blockchain client for testing without network.
///
/// Simulates blockchain operations in-memory.
#[derive(Clone)]
pub struct MockBlockchainClient {
    sessions: Arc<Mutex<HashMap<SessionId, SessionState>>>,
    pending_proofs: Arc<Mutex<Vec<ProofMetadata>>>,
    transaction_counter: Arc<Mutex<u64>>,
}

impl MockBlockchainClient {
    pub fn new() -> Self {
        Self {
            sessions: Arc::new(Mutex::new(HashMap::new())),
            pending_proofs: Arc::new(Mutex::new(Vec::new())),
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
impl ProofSubmitter for MockBlockchainClient {
    async fn submit_proof(
        &self,
        session_id: &SessionId,
        _proof_data: ProofData,
    ) -> Result<SubmissionResult> {
        // Verify session exists
        let sessions = self.sessions.lock().unwrap();
        if !sessions.contains_key(session_id) {
            return Err(crate::traits::BlockchainError::SessionNotFound(
                session_id.clone(),
            ));
        }
        drop(sessions);

        // Generate mock transaction
        let tx_id = self.next_tx_id();

        Ok(SubmissionResult {
            transaction_id: tx_id,
            gas_cost: 1000, // Mock gas cost
            status: TransactionStatus::Confirmed { block_height: 100 },
        })
    }

    async fn submit_batch(
        &self,
        session_id: &SessionId,
        proofs: Vec<ProofData>,
    ) -> Result<Vec<SubmissionResult>> {
        let mut results = Vec::new();
        for proof in proofs {
            results.push(self.submit_proof(session_id, proof).await?);
        }
        Ok(results)
    }

    async fn estimate_gas(&self, _session_id: &SessionId, _proof_data: &ProofData) -> Result<u64> {
        Ok(1000) // Mock gas estimate
    }

    async fn check_transaction(&self, _tx_id: &TransactionId) -> Result<TransactionStatus> {
        Ok(TransactionStatus::Confirmed { block_height: 100 })
    }
}

#[async_trait]
impl SessionManager for MockBlockchainClient {
    async fn create_session(&self, oracle_root: [u8; 32]) -> Result<SessionId> {
        let session_id = SessionId::from_bytes(oracle_root.to_vec());

        let session_state = SessionState {
            session_id: session_id.clone(),
            oracle_root,
            latest_state_root: [0u8; 32],
            latest_nonce: 0,
            finalized: false,
        };

        self.sessions
            .lock()
            .unwrap()
            .insert(session_id.clone(), session_state);

        Ok(session_id)
    }

    async fn get_session_state(&self, session_id: &SessionId) -> Result<SessionState> {
        self.sessions
            .lock()
            .unwrap()
            .get(session_id)
            .cloned()
            .ok_or_else(|| crate::traits::BlockchainError::SessionNotFound(session_id.clone()))
    }

    async fn finalize_session(&self, session_id: &SessionId) -> Result<TransactionId> {
        let mut sessions = self.sessions.lock().unwrap();
        let session = sessions
            .get_mut(session_id)
            .ok_or_else(|| crate::traits::BlockchainError::SessionNotFound(session_id.clone()))?;

        session.finalized = true;

        Ok(self.next_tx_id())
    }

    async fn is_session_active(&self, session_id: &SessionId) -> Result<bool> {
        let sessions = self.sessions.lock().unwrap();
        Ok(sessions
            .get(session_id)
            .map(|s| !s.finalized)
            .unwrap_or(false))
    }
}

#[async_trait]
impl BlockchainClient for MockBlockchainClient {
    async fn list_pending_proofs(&self) -> Result<Vec<ProofMetadata>> {
        Ok(self.pending_proofs.lock().unwrap().clone())
    }

    async fn submit_all_pending(&self, session_id: &SessionId) -> Result<Vec<SubmissionResult>> {
        let pending = self.list_pending_proofs().await?;
        let proofs: Vec<ProofData> = pending.into_iter().map(|p| p.proof_data).collect();

        if proofs.is_empty() {
            return Ok(Vec::new());
        }

        let results = self.submit_batch(session_id, proofs).await?;

        // Clear pending proofs after successful submission
        self.pending_proofs.lock().unwrap().clear();

        Ok(results)
    }

    fn config(&self) -> &dyn BlockchainConfig {
        &MockConfig
    }

    async fn health_check(&self) -> Result<()> {
        Ok(())
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
        let session_id = client.create_session(oracle_root).await.unwrap();

        // Verify session exists
        let state = client.get_session_state(&session_id).await.unwrap();
        assert_eq!(state.oracle_root, oracle_root);
        assert!(!state.finalized);

        // Check session is active
        assert!(client.is_session_active(&session_id).await.unwrap());

        // Submit proof (mock)
        let proof_data = zk::ProofData {
            bytes: vec![1, 2, 3],
            backend: zk::ProofBackend::Stub,
            journal: vec![0u8; 168],
            journal_digest: [0u8; 32],
        };

        let result = client.submit_proof(&session_id, proof_data).await.unwrap();
        assert_eq!(result.status, TransactionStatus::Confirmed { block_height: 100 });

        // Finalize session
        client.finalize_session(&session_id).await.unwrap();
        assert!(!client.is_session_active(&session_id).await.unwrap());
    }
}
