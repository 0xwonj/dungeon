//! Cloneable façade for issuing commands to the runtime.
//!
//! [`RuntimeHandle`] hides channel plumbing and offers async helpers for
//! stepping the simulation or streaming events from specific topics.
use std::sync::{Arc, RwLock};

use tokio::sync::{broadcast, mpsc, oneshot};

use game_core::{Action, EntityId, GameState};

use super::errors::{Result, RuntimeError};
use super::{ActionProvider, ProviderKind, ProviderRegistry};
use crate::events::{Event, EventBus, Topic};
use crate::repository::ActionBatch;
use crate::workers::persistence::Command as PersistenceCommand;
use crate::workers::simulation::Command as SimulationCommand;

/// Client-facing handle to interact with the runtime
///
/// # Concurrency Safety
///
/// Multiple clients can safely call methods concurrently. The underlying
/// [`SimulationWorker`] processes commands sequentially via a FIFO channel,
/// ensuring game state consistency without requiring explicit locks.
///
/// Provider methods use Arc<RwLock> for thread-safe access from both
/// Runtime::step() and external clients.
#[derive(Clone)]
pub struct RuntimeHandle {
    simulation_tx: mpsc::Sender<SimulationCommand>,
    persistence_tx: Option<mpsc::Sender<PersistenceCommand>>,
    event_bus: EventBus,
    providers: Arc<RwLock<ProviderRegistry>>,
    session_id: String,
    #[allow(dead_code)] // Used in multiple methods but clippy misdetects it
    base_dir: std::path::PathBuf,
    #[cfg(feature = "sui")]
    blockchain_clients: Option<Arc<crate::blockchain::BlockchainClients>>,
}

impl RuntimeHandle {
    pub(crate) fn new(
        simulation_tx: mpsc::Sender<SimulationCommand>,
        persistence_tx: Option<mpsc::Sender<PersistenceCommand>>,
        event_bus: EventBus,
        providers: Arc<RwLock<ProviderRegistry>>,
        session_id: String,
        base_dir: std::path::PathBuf,
        #[cfg(feature = "sui")] blockchain_clients: Option<
            Arc<crate::blockchain::BlockchainClients>,
        >,
    ) -> Self {
        Self {
            simulation_tx,
            persistence_tx,
            event_bus,
            providers,
            session_id,
            base_dir,
            #[cfg(feature = "sui")]
            blockchain_clients,
        }
    }

    /// Prepare the next turn - determines which entity acts next and returns game state clone
    pub async fn prepare_next_turn(&self) -> Result<(EntityId, GameState)> {
        let (reply_tx, reply_rx) = oneshot::channel();

        self.simulation_tx
            .send(SimulationCommand::PrepareNextTurn { reply: reply_tx })
            .await
            .map_err(|_| RuntimeError::CommandChannelClosed)?;

        reply_rx.await.map_err(RuntimeError::ReplyChannelClosed)?
    }

    /// Execute an action for the current turn entity
    pub async fn execute_action(&self, action: Action) -> Result<()> {
        let (reply_tx, reply_rx) = oneshot::channel();

        self.simulation_tx
            .send(SimulationCommand::ExecuteAction {
                action,
                reply: reply_tx,
            })
            .await
            .map_err(|_| RuntimeError::CommandChannelClosed)?;

        reply_rx.await.map_err(RuntimeError::ReplyChannelClosed)?
    }

    /// Subscribe to events from a specific topic
    ///
    /// # Topics
    ///
    /// - `Topic::GameState` - Action execution and failures
    /// - `Topic::Proof` - ZK proof generation events
    pub fn subscribe(&self, topic: Topic) -> broadcast::Receiver<Event> {
        self.event_bus.subscribe(topic)
    }

    /// Subscribe to multiple topics at once
    ///
    /// Returns a map of topic to receiver for each requested topic.
    pub fn subscribe_multiple(
        &self,
        topics: &[Topic],
    ) -> std::collections::HashMap<Topic, broadcast::Receiver<Event>> {
        self.event_bus.subscribe_multiple(topics)
    }

    /// Query the current game state (read-only snapshot)
    pub async fn query_state(&self) -> Result<GameState> {
        let (reply_tx, reply_rx) = oneshot::channel();

        self.simulation_tx
            .send(SimulationCommand::QueryState { reply: reply_tx })
            .await
            .map_err(|_| RuntimeError::CommandChannelClosed)?;

        reply_rx.await.map_err(RuntimeError::ReplyChannelClosed)
    }

    // Persistence and checkpoint methods

    /// Create a manual checkpoint (save point).
    ///
    /// Returns the nonce (end_nonce) of the created checkpoint.
    /// The checkpoint includes the current game state and all actions since the last checkpoint.
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - Persistence is not enabled
    /// - No active action batch exists
    /// - Failed to save state or batch metadata
    pub async fn create_checkpoint(&self) -> Result<u64> {
        let persistence_tx = self
            .persistence_tx
            .as_ref()
            .ok_or(RuntimeError::PersistenceNotEnabled)?;

        let (reply_tx, reply_rx) = oneshot::channel();

        persistence_tx
            .send(PersistenceCommand::CreateCheckpoint { reply: reply_tx })
            .await
            .map_err(|_| RuntimeError::CommandChannelClosed)?;

        reply_rx
            .await
            .map_err(RuntimeError::ReplyChannelClosed)?
            .map_err(|e| RuntimeError::PersistenceError(e.to_string()))
    }

    /// List all checkpoints (action batches).
    ///
    /// Returns all action batches for the current session, including InProgress.
    /// Caller should filter as needed (e.g., exclude InProgress for display).
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - Persistence is not enabled
    /// - Failed to read batch metadata from disk
    pub async fn list_all_checkpoints(&self) -> Result<Vec<ActionBatch>> {
        let persistence_tx = self
            .persistence_tx
            .as_ref()
            .ok_or(RuntimeError::PersistenceNotEnabled)?;

        let (reply_tx, reply_rx) = oneshot::channel();

        persistence_tx
            .send(PersistenceCommand::ListAllCheckpoints { reply: reply_tx })
            .await
            .map_err(|_| RuntimeError::CommandChannelClosed)?;

        reply_rx
            .await
            .map_err(RuntimeError::ReplyChannelClosed)?
            .map_err(|e| RuntimeError::PersistenceError(e.to_string()))
    }

    /// Get a specific checkpoint by its start nonce.
    ///
    /// Returns the action batch metadata if it exists.
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - Persistence is not enabled
    /// - Failed to read batch metadata from disk
    pub async fn get_checkpoint(&self, start_nonce: u64) -> Result<Option<ActionBatch>> {
        let persistence_tx = self
            .persistence_tx
            .as_ref()
            .ok_or(RuntimeError::PersistenceNotEnabled)?;

        let (reply_tx, reply_rx) = oneshot::channel();

        persistence_tx
            .send(PersistenceCommand::GetCheckpoint {
                start_nonce,
                reply: reply_tx,
            })
            .await
            .map_err(|_| RuntimeError::CommandChannelClosed)?;

        reply_rx
            .await
            .map_err(RuntimeError::ReplyChannelClosed)?
            .map_err(|e| RuntimeError::PersistenceError(e.to_string()))
    }

    /// Load a saved game state from a specific nonce.
    ///
    /// Returns the game state if it exists (was checkpointed).
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - Persistence is not enabled
    /// - Failed to read state from disk
    pub async fn load_state(&self, nonce: u64) -> Result<Option<GameState>> {
        let persistence_tx = self
            .persistence_tx
            .as_ref()
            .ok_or(RuntimeError::PersistenceNotEnabled)?;

        let (reply_tx, reply_rx) = oneshot::channel();

        persistence_tx
            .send(PersistenceCommand::LoadState {
                nonce,
                reply: reply_tx,
            })
            .await
            .map_err(|_| RuntimeError::CommandChannelClosed)?;

        reply_rx
            .await
            .map_err(RuntimeError::ReplyChannelClosed)?
            .map_err(|e| RuntimeError::PersistenceError(e.to_string()))
    }

    /// Restore game state from a checkpoint (fully load and replace current state).
    ///
    /// This combines loading state from disk and restoring it in the simulation worker.
    /// OnChain checkpoints cannot be restored (they are finalized on-chain).
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - Persistence is not enabled
    /// - State file does not exist for the given nonce
    /// - Checkpoint is in OnChain status (cannot roll back finalized state)
    /// - Failed to restore state in simulation worker
    pub async fn restore_state(&self, nonce: u64) -> Result<()> {
        use crate::repository::ActionBatchStatus;

        // First, check if this checkpoint is OnChain (cannot restore)
        let checkpoints = self.list_all_checkpoints().await?;
        if let Some(batch) = checkpoints.iter().find(|b| b.end_nonce == nonce)
            && matches!(
                batch.status,
                ActionBatchStatus::OnChain { .. } | ActionBatchStatus::SubmittingOnchain { .. }
            )
        {
            return Err(RuntimeError::PersistenceError(format!(
                "Cannot restore checkpoint at nonce {} (on-chain or being submitted)",
                nonce
            )));
        }

        // Load the state from disk
        let state = self.load_state(nonce).await?.ok_or_else(|| {
            RuntimeError::PersistenceError(format!("No state found at nonce {}", nonce))
        })?;

        // Then, restore it in the simulation worker
        let (reply_tx, reply_rx) = oneshot::channel();

        self.simulation_tx
            .send(SimulationCommand::RestoreState {
                state,
                reply: reply_tx,
            })
            .await
            .map_err(|_| RuntimeError::CommandChannelClosed)?;

        reply_rx.await.map_err(RuntimeError::ReplyChannelClosed)?
    }

    /// Update the status of an action batch (for manual workflow).
    ///
    /// This is used for manual blockchain submission workflows where the CLI
    /// controls each step (proving, Walrus upload, blockchain submission).
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - Persistence is not enabled
    /// - Batch not found at the given start_nonce
    /// - Failed to save updated batch
    pub async fn update_batch_status(
        &self,
        start_nonce: u64,
        status: crate::repository::ActionBatchStatus,
    ) -> Result<()> {
        let persistence_tx = self
            .persistence_tx
            .as_ref()
            .ok_or(RuntimeError::PersistenceNotEnabled)?;

        let (reply_tx, reply_rx) = oneshot::channel();

        persistence_tx
            .send(PersistenceCommand::UpdateBatchStatus {
                start_nonce,
                status,
                reply: reply_tx,
            })
            .await
            .map_err(|_| RuntimeError::CommandChannelClosed)?;

        reply_rx
            .await
            .map_err(RuntimeError::ReplyChannelClosed)?
            .map_err(|e| RuntimeError::PersistenceError(e.to_string()))
    }

    /// Read action log for a specific batch.
    ///
    /// Returns the raw action log bytes for the batch starting at start_nonce.
    /// This is used for Walrus upload in the manual workflow.
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - Persistence is not enabled
    /// - Action log file not found
    /// - Failed to read action log
    pub async fn get_action_log(&self, start_nonce: u64) -> Result<Vec<u8>> {
        let persistence_tx = self
            .persistence_tx
            .as_ref()
            .ok_or(RuntimeError::PersistenceNotEnabled)?;

        let (reply_tx, reply_rx) = oneshot::channel();

        persistence_tx
            .send(PersistenceCommand::GetActionLog {
                start_nonce,
                reply: reply_tx,
            })
            .await
            .map_err(|_| RuntimeError::CommandChannelClosed)?;

        reply_rx
            .await
            .map_err(RuntimeError::ReplyChannelClosed)?
            .map_err(|e| RuntimeError::PersistenceError(e.to_string()))
    }

    /// Get the session ID for this runtime instance.
    pub fn session_id(&self) -> &str {
        &self.session_id
    }

    /// Get a reference to the event bus for advanced usage
    pub fn event_bus(&self) -> &EventBus {
        &self.event_bus
    }

    // Provider management methods (synchronous - use Arc<RwLock>)

    /// Register a provider for a specific kind.
    ///
    /// The provider will be stored and can be used by entities bound to this kind.
    /// If a provider already exists for this kind, it will be replaced.
    pub fn register_provider(
        &self,
        kind: ProviderKind,
        provider: impl ActionProvider + 'static,
    ) -> Result<()> {
        let mut registry = self
            .providers
            .write()
            .map_err(|_| RuntimeError::LockPoisoned)?;
        registry.register(kind, provider);
        Ok(())
    }

    /// Bind an entity to a specific provider kind.
    ///
    /// The entity will use the provider registered for this kind.
    /// This can be called at runtime to switch an entity's AI or control method.
    pub fn bind_entity_provider(&self, entity: EntityId, kind: ProviderKind) -> Result<()> {
        let mut registry = self
            .providers
            .write()
            .map_err(|_| RuntimeError::LockPoisoned)?;
        registry.bind_entity(entity, kind);
        Ok(())
    }

    /// Unbind an entity, reverting it to the default provider.
    ///
    /// Returns the previous provider kind if the entity was explicitly bound.
    pub fn unbind_entity_provider(&self, entity: EntityId) -> Result<Option<ProviderKind>> {
        let mut registry = self
            .providers
            .write()
            .map_err(|_| RuntimeError::LockPoisoned)?;
        Ok(registry.unbind_entity(entity))
    }

    /// Set the default provider kind for unmapped entities.
    ///
    /// This is used as a fallback when an entity has no explicit binding.
    pub fn set_default_provider(&self, kind: ProviderKind) -> Result<()> {
        let mut registry = self
            .providers
            .write()
            .map_err(|_| RuntimeError::LockPoisoned)?;
        registry.set_default(kind);
        Ok(())
    }

    /// Get the provider kind for an entity.
    ///
    /// Returns the explicitly bound kind, or the default if not bound.
    pub fn get_entity_provider_kind(&self, entity: EntityId) -> Result<ProviderKind> {
        let registry = self
            .providers
            .read()
            .map_err(|_| RuntimeError::LockPoisoned)?;
        Ok(registry.get_entity_kind(entity))
    }

    /// Check if a provider is registered for a specific kind.
    pub fn is_provider_registered(&self, kind: ProviderKind) -> Result<bool> {
        let registry = self
            .providers
            .read()
            .map_err(|_| RuntimeError::LockPoisoned)?;
        Ok(registry.has(kind))
    }

    // Blockchain methods (feature-gated for sui)

    /// Upload action log to Walrus blob storage.
    ///
    /// This uploads the action log for a specific batch to Walrus and updates the batch status.
    ///
    /// # Returns
    ///
    /// Returns (blob_object_id, walrus_blob_id) on success.
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - Blockchain integration is not enabled
    /// - Batch not found at the given start_nonce
    /// - Batch is not in Proven status (must prove before uploading)
    /// - Walrus upload fails
    #[cfg(feature = "sui")]
    pub async fn upload_to_walrus(&self, start_nonce: u64) -> Result<(String, String)> {
        let clients = self
            .blockchain_clients
            .as_ref()
            .ok_or(RuntimeError::BlockchainNotEnabled)?;

        // 1. Get the batch to verify it's proven
        let batch = self.get_checkpoint(start_nonce).await?.ok_or_else(|| {
            RuntimeError::PersistenceError(format!("Batch not found at nonce {}", start_nonce))
        })?;

        // 2. Verify batch is proven
        if !matches!(
            batch.status,
            crate::repository::ActionBatchStatus::Proven { .. }
        ) {
            return Err(RuntimeError::InvalidConfig(format!(
                "Batch at nonce {} must be proven before uploading to Walrus (current status: {:?})",
                start_nonce, batch.status
            )));
        }

        // 3. Update status to UploadingToWalrus
        self.update_batch_status(
            start_nonce,
            crate::repository::ActionBatchStatus::UploadingToWalrus,
        )
        .await?;

        // 4. Get action log
        let action_log = self.get_action_log(start_nonce).await?;

        // 5. Upload to Walrus
        // Use 5 epochs for storage duration, send_object_to current address
        let active_address = clients.sui.active_address();
        let blob = match clients
            .walrus
            .store_blob(&action_log, 5, Some(&active_address.to_string()))
            .await
        {
            Ok(blob) => blob,
            Err(e) => {
                // Revert status back to Proven on failure
                if let crate::repository::ActionBatchStatus::Proven {
                    proof_file,
                    generation_time_ms,
                } = batch.status
                {
                    let _ = self
                        .update_batch_status(
                            start_nonce,
                            crate::repository::ActionBatchStatus::Proven {
                                proof_file,
                                generation_time_ms,
                            },
                        )
                        .await;
                }
                return Err(RuntimeError::PersistenceError(format!(
                    "Failed to upload to Walrus: {}",
                    e
                )));
            }
        };

        // 6. Update status to BlobUploaded
        self.update_batch_status(
            start_nonce,
            crate::repository::ActionBatchStatus::BlobUploaded {
                blob_object_id: blob.id.clone(),
                walrus_blob_id: blob.blob_id.clone(),
            },
        )
        .await?;

        Ok((blob.id, blob.blob_id))
    }

    /// Submit proof to blockchain (Sui).
    ///
    /// This submits a proven batch to the blockchain, referencing the Walrus blob.
    ///
    /// # Returns
    ///
    /// Returns the transaction digest on success.
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - Blockchain integration is not enabled
    /// - Batch not found at the given start_nonce
    /// - Batch is not in BlobUploaded status (must upload to Walrus first)
    /// - Blockchain submission fails
    #[cfg(feature = "sui")]
    pub async fn submit_to_blockchain(&self, start_nonce: u64) -> Result<String> {
        use client_blockchain_sui::core::{ProofSubmission, SessionId};

        let clients = self
            .blockchain_clients
            .as_ref()
            .ok_or(RuntimeError::BlockchainNotEnabled)?;

        // 1. Get the batch to retrieve blob info and proof
        let batch = self.get_checkpoint(start_nonce).await?.ok_or_else(|| {
            RuntimeError::PersistenceError(format!("Batch not found at nonce {}", start_nonce))
        })?;

        // 2. Verify batch is BlobUploaded
        let (blob_object_id, walrus_blob_id) = match &batch.status {
            crate::repository::ActionBatchStatus::BlobUploaded {
                blob_object_id,
                walrus_blob_id,
            } => (blob_object_id.clone(), walrus_blob_id.clone()),
            _ => {
                return Err(RuntimeError::InvalidConfig(format!(
                    "Batch at nonce {} must be uploaded to Walrus before blockchain submission (current status: {:?})",
                    start_nonce, batch.status
                )));
            }
        };

        // 3. Load proof file from disk using batch's proof filename
        let session_dir = self.base_dir.join(&self.session_id);
        let proof_file = batch.proof_filename();
        let proof_path = session_dir.join("proofs").join(&proof_file);

        let proof_bytes = std::fs::read(&proof_path).map_err(|e| {
            RuntimeError::PersistenceError(format!(
                "Failed to read proof file {}: {}",
                proof_path.display(),
                e
            ))
        })?;

        // Deserialize proof data
        let proof_data: zk::ProofData = bincode::deserialize(&proof_bytes).map_err(|e| {
            RuntimeError::PersistenceError(format!("Failed to deserialize proof: {}", e))
        })?;

        // 4. Load action log from disk
        let action_log_path = session_dir
            .join("actions")
            .join(batch.action_log_filename());

        let action_log_bytes = std::fs::read(&action_log_path).map_err(|e| {
            RuntimeError::PersistenceError(format!(
                "Failed to read action log {}: {}",
                action_log_path.display(),
                e
            ))
        })?;

        // 5. Update status to SubmittingOnchain
        self.update_batch_status(
            start_nonce,
            crate::repository::ActionBatchStatus::SubmittingOnchain {
                blob_object_id: blob_object_id.clone(),
                walrus_blob_id: walrus_blob_id.clone(),
            },
        )
        .await?;

        // 6. Create proof submission
        let proof_submission = ProofSubmission::from_proof_data(&proof_data, action_log_bytes)
            .map_err(|e| {
                RuntimeError::PersistenceError(format!("Failed to create proof submission: {}", e))
            })?;

        // 7. Submit to blockchain
        let session_id = SessionId::new(self.session_id.clone());
        let tx_digest = match clients
            .sui
            .update_session(&session_id, proof_submission, &blob_object_id)
            .await
        {
            Ok(digest) => digest,
            Err(e) => {
                // Revert status back to BlobUploaded on failure
                let _ = self
                    .update_batch_status(
                        start_nonce,
                        crate::repository::ActionBatchStatus::BlobUploaded {
                            blob_object_id: blob_object_id.clone(),
                            walrus_blob_id: walrus_blob_id.clone(),
                        },
                    )
                    .await;
                return Err(RuntimeError::PersistenceError(format!(
                    "Failed to submit to blockchain: {}",
                    e
                )));
            }
        };

        // 8. Update status to OnChain
        self.update_batch_status(
            start_nonce,
            crate::repository::ActionBatchStatus::OnChain {
                blob_object_id,
                walrus_blob_id,
                tx_digest: tx_digest.as_str().to_string(),
            },
        )
        .await?;

        Ok(tx_digest.as_str().to_string())
    }

    /// Create a new session on blockchain.
    ///
    /// Creates a new GameSession on-chain using state 0 (initial state) and saves
    /// the object ID to session_metadata.json.
    ///
    /// # Returns
    ///
    /// Session object ID
    ///
    /// # Errors
    ///
    /// Returns error if:
    /// - Session metadata already exists
    /// - Blockchain client is not configured
    /// - State 0 checkpoint not found
    /// - Session creation fails
    #[cfg(feature = "sui")]
    pub async fn create_session_on_blockchain(&self) -> Result<String> {
        use crate::runtime::{BlockchainSessionData, SessionInit};

        // Load session_init.json to get cryptographic commitments
        let session_dir = self.base_dir.join(&self.session_id);
        let session_init_path = session_dir.join("session_init.json");

        if !session_init_path.exists() {
            return Err(RuntimeError::PersistenceError(
                "session_init.json not found. This should be created during game initialization."
                    .to_string(),
            ));
        }

        // Read session_init.json
        let session_init_bytes = std::fs::read(&session_init_path).map_err(|e| {
            RuntimeError::PersistenceError(format!("Failed to read session_init.json: {}", e))
        })?;

        let mut session_init: SessionInit =
            serde_json::from_slice(&session_init_bytes).map_err(|e| {
                RuntimeError::PersistenceError(format!("Failed to parse session_init.json: {}", e))
            })?;

        // Check if blockchain session already exists
        if session_init.blockchain.is_some() {
            let blockchain_data = session_init.blockchain.as_ref().unwrap();
            return Err(RuntimeError::InvalidConfig(format!(
                "Blockchain session already exists: {}\nDelete session_init.json to recreate.",
                blockchain_data.session_object_id
            )));
        }

        // Get blockchain client
        let blockchain_clients = self.blockchain_clients.as_ref().ok_or_else(|| {
            RuntimeError::InvalidConfig("Blockchain client not configured".to_string())
        })?;

        tracing::info!(
            "Creating session on blockchain using session_init.json:\n\
             - oracle_root: {}\n\
             - initial_state_root: {}\n\
             - seed_commitment: {}",
            hex::encode(session_init.oracle_root),
            hex::encode(session_init.initial_state_root),
            hex::encode(session_init.seed_commitment)
        );

        // Create session on blockchain
        let session_id = blockchain_clients
            .sui
            .create_session(
                session_init.oracle_root,
                session_init.initial_state_root,
                session_init.seed_commitment,
            )
            .await
            .map_err(|e| {
                RuntimeError::PersistenceError(format!(
                    "Failed to create session on blockchain: {:?}\n\
                     This may be due to:\n\
                     - Network connection issues\n\
                     - Invalid Sui configuration (check SUI_PACKAGE_ID)\n\
                     - Insufficient gas (check SUI_GAS_BUDGET)\n\
                     - Keystore issues (check ~/.sui/sui_config/sui.keystore)",
                    e
                ))
            })?;

        let session_object_id = session_id.as_str().to_string();

        // Get network name from Sui config
        let network = blockchain_clients.sui.config.network_name().to_string();

        // Update session_init.json with blockchain data
        session_init.blockchain = Some(BlockchainSessionData {
            session_object_id: session_object_id.clone(),
            network,
        });

        // Write updated session_init.json
        let json = serde_json::to_string_pretty(&session_init).map_err(|e| {
            RuntimeError::PersistenceError(format!("Failed to serialize session_init.json: {}", e))
        })?;

        std::fs::write(&session_init_path, json).map_err(|e| {
            RuntimeError::PersistenceError(format!("Failed to write session_init.json: {}", e))
        })?;

        tracing::info!(
            "Session created successfully. session_object_id: {}",
            session_object_id
        );

        Ok(session_object_id)
    }

    /// Save session object ID to file.
    ///
    /// Stores the Sui GameSession object ID in the session directory for later use.
    ///
    /// # Arguments
    ///
    /// * `session_object_id` - Sui object ID of the created GameSession
    #[cfg(feature = "sui")]
    pub async fn save_session_object_id(&self, session_object_id: String) -> Result<()> {
        let session_dir = self.base_dir.join(&self.session_id);
        let metadata_path = session_dir.join("session_metadata.json");

        let metadata = serde_json::json!({
            "session_object_id": session_object_id,
            "created_at": std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_secs()
                .to_string(),
        });

        std::fs::write(
            &metadata_path,
            serde_json::to_string_pretty(&metadata).unwrap(),
        )
        .map_err(|e| {
            RuntimeError::PersistenceError(format!("Failed to save session metadata: {}", e))
        })?;

        Ok(())
    }

    /// Load session object ID from file.
    ///
    /// Reads the Sui GameSession object ID from the session directory.
    ///
    /// # Returns
    ///
    /// Session object ID string
    ///
    /// # Errors
    ///
    /// Returns error if metadata file doesn't exist or is invalid.
    #[cfg(feature = "sui")]
    fn load_session_object_id(&self) -> Result<String> {
        use crate::runtime::SessionInit;

        let session_dir = self.base_dir.join(&self.session_id);
        let session_init_path = session_dir.join("session_init.json");

        if !session_init_path.exists() {
            return Err(RuntimeError::InvalidConfig(format!(
                "Session initialization file not found at {}. Create a session on-chain first.",
                session_init_path.display()
            )));
        }

        let session_init_bytes = std::fs::read(&session_init_path).map_err(|e| {
            RuntimeError::PersistenceError(format!("Failed to read session_init.json: {}", e))
        })?;

        let session_init: SessionInit =
            serde_json::from_slice(&session_init_bytes).map_err(|e| {
                RuntimeError::PersistenceError(format!("Failed to parse session_init.json: {}", e))
            })?;

        session_init
            .blockchain
            .as_ref()
            .map(|b| b.session_object_id.clone())
            .ok_or_else(|| {
                RuntimeError::InvalidConfig(
                    "Blockchain session not created yet. Use [C] key to create session on-chain first.".to_string()
                )
            })
    }

    /// Get blockchain session info (if available).
    ///
    /// This queries the blockchain for the current state of the session.
    /// Returns None if blockchain integration is disabled or session is not created.
    #[cfg(feature = "sui")]
    pub async fn get_blockchain_session_info(
        &self,
    ) -> Result<Option<client_blockchain_sui::contracts::GameSession>> {
        use client_blockchain_sui::core::SessionId;

        // 1. Check if blockchain is enabled
        let clients = match &self.blockchain_clients {
            Some(c) => c,
            None => return Ok(None),
        };

        // 2. Load session object ID
        let session_object_id = match self.load_session_object_id() {
            Ok(id) => id,
            Err(_) => return Ok(None), // Session not created yet
        };

        // 3. Query blockchain
        let session_id = SessionId::new(session_object_id);
        match clients.sui.get_session(&session_id).await {
            Ok(session) => Ok(Some(session)),
            Err(e) => {
                tracing::warn!("Failed to fetch session info: {}", e);
                Ok(None)
            }
        }
    }
}
