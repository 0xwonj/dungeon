//! game_session Move contract integration.
//!
//! This module provides direct interaction with the on-chain `game_session` contract.
//!
//! ## Move Contract Reference
//!
//! ```move
//! module dungeon::game_session {
//!     public struct GameSession has key, store {
//!         id: UID,
//!         player: address,
//!         oracle_root: vector<u8>,
//!         initial_state_root: vector<u8>,
//!         seed_commitment: vector<u8>,
//!         state_root: vector<u8>,
//!         nonce: u64,
//!         pending_action_logs: u64,
//!         finalized: bool,
//!     }
//!
//!     public fun create(...): GameSession;
//!     public fun update(session: &mut GameSession, ...);
//!     public fun finalize(session: &mut GameSession, ...);
//! }
//! ```

use anyhow::{Context, Result, anyhow};
use serde::{Deserialize, Serialize};
use shared_crypto::intent::{Intent, IntentMessage};
use sui_keys::keystore::{AccountKeystore, FileBasedKeystore};
use sui_sdk::SuiClient;
use sui_sdk::rpc_types::{SuiObjectDataOptions, SuiTransactionBlockEffectsAPI};
use sui_types::Identifier;
use sui_types::base_types::{ObjectID, SuiAddress};
use sui_types::object::Owner;
use sui_types::programmable_transaction_builder::ProgrammableTransactionBuilder;
use sui_types::transaction::{ObjectArg, TransactionData};

use crate::core::types::{ProofSubmission, SessionId, StateRoot, TxDigest};

// ============================================================================
// GameSession Object (1:1 mapping with Move struct)
// ============================================================================

/// On-chain GameSession object.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GameSession {
    /// Session Object ID
    pub id: String,

    /// Player address (session owner)
    pub player: Vec<u8>,

    /// Oracle data commitment (content hash)
    pub oracle_root: [u8; 32],

    /// Initial state root at game start
    pub initial_state_root: [u8; 32],

    /// Seed commitment for RNG fairness
    pub seed_commitment: [u8; 32],

    /// Current game state root
    pub state_root: [u8; 32],

    /// Action execution nonce
    pub nonce: u64,

    /// Number of pending action logs
    pub pending_action_logs: u64,

    /// Whether the session is finalized
    pub finalized: bool,
}

impl GameSession {
    pub fn new(
        id: String,
        player: Vec<u8>,
        oracle_root: [u8; 32],
        initial_state_root: [u8; 32],
        seed_commitment: [u8; 32],
    ) -> Self {
        Self {
            id,
            player,
            oracle_root,
            initial_state_root,
            seed_commitment,
            state_root: initial_state_root,
            nonce: 0,
            pending_action_logs: 0,
            finalized: false,
        }
    }

    pub fn is_finalized(&self) -> bool {
        self.finalized
    }
}

// ============================================================================
// GameSessionContract - Contract metadata and transaction builders
// ============================================================================

/// Game session contract metadata and transaction builders.
///
/// This struct contains only contract configuration (package ID, VK ID).
/// All blockchain interactions use Dependency Injection pattern - SDK resources
/// (SuiClient, keystore, address) are passed as method arguments.
///
/// ## Design Pattern: Dependency Injection
///
/// Benefits:
/// - Clear separation: Contract = domain logic, Client = infrastructure
/// - Testability: Can test contract methods independently with mocks
/// - Reusability: Contract can be used in different contexts
/// - Scalability: Adding new contracts doesn't bloat the client
pub struct GameSessionContract {
    /// Sui package ID (dungeon::game_session module)
    pub package_id: String,

    /// Verifying key object ID (for proof verification)
    pub vk_object_id: Option<String>,
}

impl GameSessionContract {
    /// Create new game session contract client.
    pub fn new(package_id: String, vk_object_id: Option<String>) -> Self {
        Self {
            package_id,
            vk_object_id,
        }
    }

    /// Set verifying key object ID.
    pub fn set_vk(&mut self, vk_id: String) {
        self.vk_object_id = Some(vk_id);
    }

    /// Get package ID as ObjectID.
    fn package_object_id(&self) -> Result<ObjectID> {
        self.package_id.parse().context("Invalid package ID format")
    }

    /// Get gas coin for transaction payment.
    ///
    /// Fetches the first available gas coin for the given address.
    async fn get_gas_coin(
        sui_client: &SuiClient,
        active_address: SuiAddress,
    ) -> Result<sui_types::base_types::ObjectRef> {
        let gas_coins = sui_client
            .coin_read_api()
            .get_coins(active_address, None, None, None)
            .await
            .context("Failed to get gas coins")?;

        let gas_coin = gas_coins
            .data
            .first()
            .ok_or_else(|| anyhow!("No gas coins available for address {}", active_address))?;

        tracing::debug!(
            "Using gas coin: {} with balance: {}",
            gas_coin.coin_object_id,
            gas_coin.balance
        );

        Ok(gas_coin.object_ref())
    }

    /// Create a new game session on-chain.
    ///
    /// Builds and executes a PTB calling `dungeon::game_session::create()`.
    ///
    /// # Arguments
    ///
    /// * `sui_client` - Sui RPC client (injected dependency)
    /// * `keystore` - Keystore for transaction signing (injected dependency)
    /// * `active_address` - Signer address (injected dependency)
    /// * `gas_budget` - Gas budget in MIST
    /// * `oracle_root` - Content hash of oracle data
    /// * `initial_state_root` - Initial game state root
    /// * `seed_commitment` - RNG seed commitment
    ///
    /// # Returns
    ///
    /// SessionId of the created on-chain object.
    ///
    /// # Errors
    ///
    /// Returns error if:
    /// - PTB construction fails
    /// - Transaction execution fails
    /// - Created object cannot be parsed from response
    #[allow(clippy::too_many_arguments)] // SDK dependency injection pattern
    pub async fn create(
        &self,
        sui_client: &SuiClient,
        keystore: &FileBasedKeystore,
        active_address: SuiAddress,
        gas_budget: u64,
        oracle_root: [u8; 32],
        initial_state_root: [u8; 32],
        seed_commitment: [u8; 32],
    ) -> Result<SessionId> {
        tracing::info!("Creating game session on-chain...");

        // Build Programmable Transaction Block
        let mut ptb = ProgrammableTransactionBuilder::new();

        // Prepare pure arguments (vectors of bytes)
        let oracle_root_arg = ptb.pure(oracle_root.to_vec())?;
        let initial_state_root_arg = ptb.pure(initial_state_root.to_vec())?;
        let seed_commitment_arg = ptb.pure(seed_commitment.to_vec())?;

        // Add Move call: package::game_session::create
        let package_id = self.package_object_id()?;
        ptb.programmable_move_call(
            package_id,
            Identifier::new("game_session")?,
            Identifier::new("create")?,
            vec![], // No type arguments
            vec![oracle_root_arg, initial_state_root_arg, seed_commitment_arg],
        );

        // Finalize PTB
        let pt = ptb.finish();

        // Get current gas price
        let gas_price = sui_client
            .read_api()
            .get_reference_gas_price()
            .await
            .context("Failed to get reference gas price")?;

        // Get gas coin for payment
        let gas_coin = Self::get_gas_coin(sui_client, active_address).await?;

        // Build transaction data
        let tx_data = TransactionData::new_programmable(
            active_address,
            vec![gas_coin],
            pt,
            gas_budget,
            gas_price,
        );

        // Sign transaction with intent
        let keypair = keystore
            .export(&active_address)
            .context("Failed to export keypair from keystore")?;

        let signature = sui_types::crypto::Signature::new_secure(
            &IntentMessage::new(Intent::sui_transaction(), &tx_data),
            keypair,
        );

        // Execute transaction
        tracing::debug!("Executing create session transaction...");
        let response = sui_client
            .quorum_driver_api()
            .execute_transaction_block(
                sui_types::transaction::Transaction::from_data(tx_data, vec![signature]),
                sui_sdk::rpc_types::SuiTransactionBlockResponseOptions::new()
                    .with_effects()
                    .with_object_changes(),
                None, // No execution options
            )
            .await
            .context("Failed to execute create session transaction")?;

        // Extract created object ID from response
        let object_changes = response
            .object_changes
            .ok_or_else(|| anyhow!("No object changes in transaction response"))?;

        for change in object_changes {
            if let sui_sdk::rpc_types::ObjectChange::Created {
                object_id,
                object_type,
                ..
            } = change
            {
                // Verify it's a GameSession object
                if object_type
                    .to_string()
                    .contains("game_session::GameSession")
                {
                    let session_id = SessionId::new(object_id.to_string());
                    tracing::info!("✓ Session created: {}", session_id.as_str());
                    return Ok(session_id);
                }
            }
        }

        Err(anyhow!(
            "Failed to find created GameSession object in transaction response"
        ))
    }

    /// Update session with verified proof and pre-uploaded Walrus blob.
    ///
    /// Builds and executes a PTB calling `dungeon::game_session::update()`.
    ///
    /// **Prerequisites:**
    /// - ZK proof must be generated
    /// - Action log must be uploaded to Walrus (use WalrusClient separately)
    /// - Blob object must be owned by active_address
    /// - VK must be registered on-chain
    ///
    /// This method only handles transaction construction and execution.
    /// Walrus upload must be done separately before calling this method.
    ///
    /// # Arguments
    ///
    /// * `sui_client` - Sui RPC client (injected dependency)
    /// * `keystore` - Keystore for transaction signing (injected dependency)
    /// * `active_address` - Signer address (injected dependency)
    /// * `gas_budget` - Gas budget in MIST
    /// * `session_id` - Session object ID to update
    /// * `proof` - Proof submission containing ZK proof and journal
    /// * `blob_object_id` - Sui ObjectID of the Walrus Blob (NOT Walrus blob_id!)
    ///
    /// # Returns
    ///
    /// Transaction digest of the update transaction.
    ///
    /// # Errors
    ///
    /// Returns error if:
    /// - VK object ID is not configured
    /// - Session/VK/Blob objects cannot be fetched
    /// - Blob object is not owned by active_address
    /// - PTB construction fails
    /// - Transaction execution fails
    #[allow(clippy::too_many_arguments)] // SDK dependency injection pattern
    pub async fn update(
        &self,
        sui_client: &SuiClient,
        keystore: &FileBasedKeystore,
        active_address: SuiAddress,
        gas_budget: u64,
        session_id: &SessionId,
        proof: ProofSubmission,
        blob_object_id: &str,
    ) -> Result<TxDigest> {
        tracing::info!(
            "Updating session {} with proof (blob: {}...)...",
            session_id.as_str(),
            &blob_object_id[..blob_object_id.len().min(16)]
        );

        // Verify VK is configured
        let vk_id = self.vk_object_id.as_ref().ok_or_else(|| {
            anyhow!("Verifying key not configured. Run 'cargo xtask sui setup' first.")
        })?;

        // Parse journal to extract new values
        let (new_state_root, new_nonce) = proof.parse_journal()?;

        tracing::debug!(
            "Proof verification: new_state_root={}, new_nonce={}",
            hex::encode(new_state_root),
            new_nonce
        );

        // ========================================================================
        // Build Programmable Transaction Block
        // ========================================================================

        let mut ptb = ProgrammableTransactionBuilder::new();

        // Prepare session object argument (mutable reference)
        let session_obj_id: ObjectID = session_id
            .as_str()
            .parse()
            .context("Invalid session ID format")?;

        let session_obj = sui_client
            .read_api()
            .get_object_with_options(session_obj_id, SuiObjectDataOptions::default())
            .await
            .context("Failed to fetch session object")?
            .into_object()
            .context("Session object not found")?;

        let session_arg = ptb.obj(ObjectArg::ImmOrOwnedObject(session_obj.object_ref()))?;

        // Prepare VK object argument (immutable reference)
        let vk_obj_id: ObjectID = vk_id.parse().context("Invalid VK object ID format")?;

        let vk_obj = sui_client
            .read_api()
            .get_object_with_options(vk_obj_id, SuiObjectDataOptions::default())
            .await
            .context("Failed to fetch VK object")?
            .into_object()
            .context("VK object not found")?;

        let vk_arg = ptb.obj(ObjectArg::ImmOrOwnedObject(vk_obj.object_ref()))?;

        // Prepare Walrus Blob object argument
        let blob_obj_id: ObjectID = blob_object_id
            .parse()
            .context("Invalid Walrus blob object ID format")?;

        let blob_obj = sui_client
            .read_api()
            .get_object_with_options(blob_obj_id, SuiObjectDataOptions::default())
            .await
            .context("Failed to fetch Walrus blob object")?
            .into_object()
            .context("Walrus blob object not found")?;

        let blob_arg = ptb.obj(ObjectArg::ImmOrOwnedObject(blob_obj.object_ref()))?;

        // Prepare proof and state arguments
        let proof_arg = ptb.pure(proof.proof_points.clone())?;
        let new_state_root_arg = ptb.pure(new_state_root.to_vec())?;
        let new_nonce_arg = ptb.pure(new_nonce)?;

        // ========================================================================
        // Call game_session::update
        // ========================================================================

        let package_id = self.package_object_id()?;
        ptb.programmable_move_call(
            package_id,
            Identifier::new("game_session")?,
            Identifier::new("update")?,
            vec![], // No type arguments
            vec![
                session_arg,
                vk_arg,
                proof_arg,
                new_state_root_arg,
                new_nonce_arg,
                blob_arg, // Walrus Blob object
            ],
        );

        // Finalize PTB
        let pt = ptb.finish();

        // ========================================================================
        // Execute transaction
        // ========================================================================

        // Get current gas price
        let gas_price = sui_client
            .read_api()
            .get_reference_gas_price()
            .await
            .context("Failed to get reference gas price")?;

        // Get gas coin for payment
        let gas_coin = Self::get_gas_coin(sui_client, active_address).await?;

        // Build transaction data
        let tx_data = TransactionData::new_programmable(
            active_address,
            vec![gas_coin],
            pt,
            gas_budget,
            gas_price,
        );

        // Sign transaction with intent
        let keypair = keystore
            .export(&active_address)
            .context("Failed to export keypair from keystore")?;

        let signature = sui_types::crypto::Signature::new_secure(
            &IntentMessage::new(Intent::sui_transaction(), &tx_data),
            keypair,
        );

        // Execute transaction
        tracing::debug!("Executing update session transaction...");
        let response = sui_client
            .quorum_driver_api()
            .execute_transaction_block(
                sui_types::transaction::Transaction::from_data(tx_data, vec![signature]),
                sui_sdk::rpc_types::SuiTransactionBlockResponseOptions::new().with_effects(),
                None, // No execution options
            )
            .await
            .context("Failed to execute update session transaction")?;

        let digest = TxDigest::new(response.digest.to_string());
        tracing::info!("✓ Session updated. Transaction: {}", digest.as_str());

        Ok(digest)
    }

    /// Update session state without Walrus blob (testing only).
    ///
    /// Similar to `update()` but calls `update_without_blob()` which bypasses Walrus blob
    /// requirement. The actions_root is provided directly instead of being derived from blob_id.
    ///
    /// # Arguments
    ///
    /// * `sui_client` - Sui RPC client (injected dependency)
    /// * `keystore` - Keystore for transaction signing (injected dependency)
    /// * `active_address` - Signer address (injected dependency)
    /// * `gas_budget` - Gas budget in MIST
    /// * `session_id` - Session object ID to update
    /// * `proof` - Proof submission containing ZK proof and journal
    ///
    /// # Returns
    ///
    /// Transaction digest of the update transaction.
    pub async fn update_without_blob(
        &self,
        sui_client: &SuiClient,
        keystore: &FileBasedKeystore,
        active_address: SuiAddress,
        gas_budget: u64,
        session_id: &SessionId,
        proof: ProofSubmission,
    ) -> Result<TxDigest> {
        tracing::info!(
            "[TEST] Updating session {} with proof (without blob)...",
            session_id.as_str()
        );

        // Verify VK is configured
        let vk_id = self.vk_object_id.as_ref().ok_or_else(|| {
            anyhow!("Verifying key not configured. Run 'cargo xtask sui setup' first.")
        })?;

        // Parse journal to extract values
        let journal_fields = zk::parse_journal(&proof.journal)
            .map_err(|e| anyhow!("Failed to parse journal: {}", e))?;

        let new_state_root = journal_fields.new_state_root;
        let new_nonce = journal_fields.new_nonce;
        let actions_root = journal_fields.actions_root;

        tracing::debug!(
            "[TEST] Proof verification: new_state_root={}, new_nonce={}, actions_root={}",
            hex::encode(new_state_root),
            new_nonce,
            hex::encode(actions_root)
        );

        // ========================================================================
        // Build Programmable Transaction Block
        // ========================================================================

        let mut ptb = ProgrammableTransactionBuilder::new();

        // Prepare session object argument (mutable reference)
        let session_obj_id: ObjectID = session_id
            .as_str()
            .parse()
            .context("Invalid session ID format")?;

        let session_obj = sui_client
            .read_api()
            .get_object_with_options(
                session_obj_id,
                SuiObjectDataOptions::new()
                    .with_type()
                    .with_owner()
                    .with_previous_transaction(),
            )
            .await
            .context("Failed to fetch session object")?
            .into_object()
            .context("Session object not found")?;

        tracing::debug!(
            "[TEST] Session object: id={}, version={}, digest={:?}, owner={:?}",
            session_obj.object_id,
            session_obj.version,
            session_obj.digest,
            session_obj.owner
        );

        let session_arg = ptb.obj(ObjectArg::ImmOrOwnedObject(session_obj.object_ref()))?;

        // Prepare VK object argument (immutable reference)
        let vk_obj_id: ObjectID = vk_id.parse().context("Invalid VK object ID format")?;

        let vk_obj = sui_client
            .read_api()
            .get_object_with_options(
                vk_obj_id,
                SuiObjectDataOptions::new()
                    .with_type()
                    .with_owner()
                    .with_previous_transaction(),
            )
            .await
            .context("Failed to fetch VK object")?
            .into_object()
            .context("VK object not found")?;

        tracing::debug!(
            "[TEST] VK object: id={}, version={}, digest={:?}, owner={:?}",
            vk_obj.object_id,
            vk_obj.version,
            vk_obj.digest,
            vk_obj.owner
        );

        // VK is a shared object, not owned
        let vk_arg = if let Some(Owner::Shared {
            initial_shared_version,
        }) = &vk_obj.owner
        {
            tracing::debug!(
                "[TEST] VK is shared object with initial version: {}",
                initial_shared_version.value()
            );
            // For shared objects, we need to use CallArg directly
            ptb.input(sui_types::transaction::CallArg::Object(
                ObjectArg::SharedObject {
                    id: vk_obj.object_id,
                    initial_shared_version: *initial_shared_version,
                    mutability: sui_types::transaction::SharedObjectMutability::Immutable,
                },
            ))?
        } else {
            // Fallback to ImmOrOwnedObject if not shared (this shouldn't happen for VK)
            tracing::warn!("[TEST] VK is not a shared object, using as owned/immutable");
            ptb.input(sui_types::transaction::CallArg::Object(
                ObjectArg::ImmOrOwnedObject(vk_obj.object_ref()),
            ))?
        };

        // Prepare proof and state arguments
        let proof_arg = ptb.pure(proof.proof_points.clone())?;
        let journal_digest_arg = ptb.pure(proof.journal_digest.to_vec())?;
        let actions_root_arg = ptb.pure(actions_root.to_vec())?;
        let new_state_root_arg = ptb.pure(new_state_root.to_vec())?;
        let new_nonce_arg = ptb.pure(new_nonce)?;

        tracing::debug!(
            "[TEST] Proof data: proof_points_len={}, journal_digest={}, actions_root={}, new_state_root={}, new_nonce={}",
            proof.proof_points.len(),
            hex::encode(proof.journal_digest),
            hex::encode(actions_root),
            hex::encode(new_state_root),
            new_nonce
        );

        // ========================================================================
        // Call game_session::update_without_blob
        // ========================================================================

        let package_id = self.package_object_id()?;
        ptb.programmable_move_call(
            package_id,
            Identifier::new("game_session")?,
            Identifier::new("update_without_blob")?,
            vec![], // No type arguments
            vec![
                session_arg,
                vk_arg,
                proof_arg,
                journal_digest_arg,
                actions_root_arg,
                new_state_root_arg,
                new_nonce_arg,
            ],
        );

        // Finalize PTB
        let pt = ptb.finish();

        // ========================================================================
        // Execute transaction
        // ========================================================================

        // Get current gas price
        let gas_price = sui_client
            .read_api()
            .get_reference_gas_price()
            .await
            .context("Failed to get reference gas price")?;

        // Get gas coin for payment
        let gas_coin = Self::get_gas_coin(sui_client, active_address).await?;

        // Build transaction data
        let tx_data = TransactionData::new_programmable(
            active_address,
            vec![gas_coin],
            pt,
            gas_budget,
            gas_price,
        );

        // Sign transaction with intent
        let keypair = keystore
            .export(&active_address)
            .context("Failed to export keypair from keystore")?;

        let signature = sui_types::crypto::Signature::new_secure(
            &IntentMessage::new(Intent::sui_transaction(), &tx_data),
            keypair,
        );

        // Dry run first to check for errors
        tracing::debug!("[TEST] Performing dry-run of update session transaction...");
        let dry_run_result = sui_client
            .read_api()
            .dry_run_transaction_block(tx_data.clone())
            .await;

        match dry_run_result {
            Ok(dry_run) => {
                tracing::debug!("[TEST] Dry-run status: {:?}", dry_run.effects.status());
                if let sui_sdk::rpc_types::SuiExecutionStatus::Failure { error } =
                    dry_run.effects.status()
                {
                    tracing::error!("[TEST] Dry-run FAILED: {}", error);
                    return Err(anyhow!("Dry-run failed: {}", error));
                }
                tracing::info!("[TEST] Dry-run successful, proceeding with execution...");
            }
            Err(e) => {
                tracing::error!("[TEST] Dry-run error: {:?}", e);
                return Err(anyhow!("Dry-run error: {}", e));
            }
        }

        // Execute transaction
        tracing::debug!("[TEST] Executing update session transaction (without blob)...");
        let response = sui_client
            .quorum_driver_api()
            .execute_transaction_block(
                sui_types::transaction::Transaction::from_data(tx_data, vec![signature]),
                sui_sdk::rpc_types::SuiTransactionBlockResponseOptions::new()
                    .with_effects()
                    .with_events()
                    .with_object_changes(),
                None, // No execution options
            )
            .await
            .context("Failed to execute update session transaction")?;

        // Check transaction effects for errors
        let digest = TxDigest::new(response.digest.to_string());

        if let Some(effects) = &response.effects {
            // Check execution status
            let status = &effects.status();
            tracing::debug!("[TEST] Transaction status: {:?}", status);

            match status {
                sui_sdk::rpc_types::SuiExecutionStatus::Success => {
                    tracing::info!(
                        "✓ [TEST] Session updated (without blob). Transaction: {}",
                        digest.as_str()
                    );
                }
                sui_sdk::rpc_types::SuiExecutionStatus::Failure { error } => {
                    tracing::error!(
                        "[TEST] Transaction FAILED on-chain. Digest: {}, Error: {}",
                        digest.as_str(),
                        error
                    );
                    return Err(anyhow!(
                        "Transaction failed on-chain: {} (tx: {})",
                        error,
                        digest.as_str()
                    ));
                }
            }

            // Log additional debugging info
            if let Some(events) = &response.events {
                tracing::debug!(
                    "[TEST] Transaction events: {} events emitted",
                    events.data.len()
                );
                for event in &events.data {
                    tracing::debug!("[TEST] Event: {:?}", event.type_);
                }
            }

            if let Some(obj_changes) = &response.object_changes {
                tracing::debug!(
                    "[TEST] Object changes: {} objects affected",
                    obj_changes.len()
                );
            }
        } else {
            tracing::warn!("[TEST] No effects returned in transaction response");
        }

        Ok(digest)
    }

    /// Finalize a game session.
    ///
    /// Builds and executes a PTB calling `dungeon::game_session::finalize()`.
    ///
    /// # Arguments
    ///
    /// * `sui_client` - Sui RPC client (injected dependency)
    /// * `keystore` - Keystore for transaction signing (injected dependency)
    /// * `active_address` - Signer address (injected dependency)
    /// * `gas_budget` - Gas budget in MIST
    /// * `session_id` - Session object ID to finalize
    ///
    /// # Returns
    ///
    /// Transaction digest of the finalize transaction.
    ///
    /// # Errors
    ///
    /// Returns error if:
    /// - Session object cannot be fetched
    /// - Session has pending action logs (must be cleaned up first)
    /// - PTB construction fails
    /// - Transaction execution fails
    pub async fn finalize(
        &self,
        sui_client: &SuiClient,
        keystore: &FileBasedKeystore,
        active_address: SuiAddress,
        gas_budget: u64,
        session_id: &SessionId,
    ) -> Result<TxDigest> {
        tracing::info!("Finalizing session {}...", session_id.as_str());

        // Build Programmable Transaction Block
        let mut ptb = ProgrammableTransactionBuilder::new();

        // Prepare session object argument (mutable reference)
        let session_obj_id: ObjectID = session_id
            .as_str()
            .parse()
            .context("Invalid session ID format")?;

        let session_obj = sui_client
            .read_api()
            .get_object_with_options(session_obj_id, SuiObjectDataOptions::default())
            .await
            .context("Failed to fetch session object")?
            .into_object()
            .context("Session object not found")?;

        let session_arg = ptb.obj(ObjectArg::ImmOrOwnedObject(session_obj.object_ref()))?;

        // Add Move call: package::game_session::finalize
        let package_id = self.package_object_id()?;
        ptb.programmable_move_call(
            package_id,
            Identifier::new("game_session")?,
            Identifier::new("finalize")?,
            vec![], // No type arguments
            vec![session_arg],
        );

        // Finalize PTB
        let pt = ptb.finish();

        // Get current gas price
        let gas_price = sui_client
            .read_api()
            .get_reference_gas_price()
            .await
            .context("Failed to get reference gas price")?;

        // Get gas coin for payment
        let gas_coin = Self::get_gas_coin(sui_client, active_address).await?;

        // Build transaction data
        let tx_data = TransactionData::new_programmable(
            active_address,
            vec![gas_coin],
            pt,
            gas_budget,
            gas_price,
        );

        // Sign transaction with intent
        let keypair = keystore
            .export(&active_address)
            .context("Failed to export keypair from keystore")?;

        let signature = sui_types::crypto::Signature::new_secure(
            &IntentMessage::new(Intent::sui_transaction(), &tx_data),
            keypair,
        );

        // Execute transaction
        tracing::debug!("Executing finalize session transaction...");
        let response = sui_client
            .quorum_driver_api()
            .execute_transaction_block(
                sui_types::transaction::Transaction::from_data(tx_data, vec![signature]),
                sui_sdk::rpc_types::SuiTransactionBlockResponseOptions::new().with_effects(),
                None, // No execution options
            )
            .await
            .context("Failed to execute finalize session transaction")?;

        let digest = TxDigest::new(response.digest.to_string());
        tracing::info!("✓ Session finalized. Transaction: {}", digest.as_str());

        Ok(digest)
    }

    /// Get session object from blockchain.
    ///
    /// Queries the session object via Sui RPC and parses Move struct fields.
    ///
    /// # Arguments
    ///
    /// * `sui_client` - Sui RPC client (injected dependency)
    /// * `session_id` - Session object ID to query
    ///
    /// # Returns
    ///
    /// GameSession struct with all fields populated from on-chain data.
    ///
    /// # Errors
    ///
    /// Returns error if:
    /// - Session object ID is invalid
    /// - Object cannot be fetched from RPC
    /// - Object content cannot be parsed
    /// - Move struct fields are missing or malformed
    pub async fn get(&self, sui_client: &SuiClient, session_id: &SessionId) -> Result<GameSession> {
        tracing::debug!("Querying session {}...", session_id.as_str());

        // Parse session object ID
        let obj_id: ObjectID = session_id
            .as_str()
            .parse()
            .context("Invalid session ID format")?;

        // Fetch object with content
        let response = sui_client
            .read_api()
            .get_object_with_options(
                obj_id,
                SuiObjectDataOptions::new().with_content().with_bcs(),
            )
            .await
            .context("Failed to fetch session object from RPC")?;

        let obj_data = response
            .into_object()
            .context("Session object not found on-chain")?;

        // Parse Move struct content
        let content = obj_data
            .content
            .ok_or_else(|| anyhow!("Session object has no content"))?;

        // Use BCS deserialization if available, otherwise parse JSON
        if let Some(bcs_bytes) = obj_data.bcs {
            match bcs_bytes {
                sui_sdk::rpc_types::SuiRawData::MoveObject(_move_obj) => {
                    // BCS deserialize the Move struct
                    // Note: This requires the GameSession struct to implement BCS Deserialize
                    // and match the on-chain Move struct layout exactly

                    tracing::warn!(
                        "BCS deserialization not yet implemented for GameSession.\n\
                         Using fallback JSON parsing instead."
                    );
                    // Fall through to JSON parsing below
                }
                _ => {
                    return Err(anyhow!("Unexpected BCS data type for session object"));
                }
            }
        }

        // Fallback: Parse JSON representation
        if let sui_sdk::rpc_types::SuiParsedData::MoveObject(move_obj) = content {
            let fields = move_obj.fields.to_json_value();

            // Extract fields from JSON
            let player = parse_address_field(&fields, "player")?;
            let oracle_root = parse_bytes_field(&fields, "oracle_root")?;
            let initial_state_root = parse_bytes_field(&fields, "initial_state_root")?;
            let seed_commitment = parse_bytes_field(&fields, "seed_commitment")?;
            let state_root = parse_bytes_field(&fields, "state_root")?;
            let nonce = parse_u64_field(&fields, "nonce")?;
            let pending_action_logs = parse_u64_field(&fields, "pending_action_logs")?;
            let finalized = parse_bool_field(&fields, "finalized")?;

            Ok(GameSession {
                id: obj_id.to_string(),
                player,
                oracle_root,
                initial_state_root,
                seed_commitment,
                state_root,
                nonce,
                pending_action_logs,
                finalized,
            })
        } else {
            Err(anyhow!("Session object content is not a Move object"))
        }
    }

    /// Get current state root for a session.
    pub async fn get_state_root(
        &self,
        sui_client: &SuiClient,
        session_id: &SessionId,
    ) -> Result<StateRoot> {
        let session = self.get(sui_client, session_id).await?;
        Ok(session.state_root)
    }

    /// Get current nonce for a session.
    pub async fn get_nonce(&self, sui_client: &SuiClient, session_id: &SessionId) -> Result<u64> {
        let session = self.get(sui_client, session_id).await?;
        Ok(session.nonce)
    }

    /// Check if session is active (not finalized).
    pub async fn is_active(&self, sui_client: &SuiClient, session_id: &SessionId) -> Result<bool> {
        let session = self.get(sui_client, session_id).await?;
        Ok(!session.finalized)
    }
}

// ============================================================================
// Helper Functions for JSON Parsing
// ============================================================================

/// Parse address field from Move object JSON.
fn parse_address_field(fields: &serde_json::Value, field_name: &str) -> Result<Vec<u8>> {
    let addr_str = fields
        .get(field_name)
        .and_then(|v| v.as_str())
        .ok_or_else(|| anyhow!("Missing or invalid '{}' field", field_name))?;

    // Sui addresses are hex strings like "0x..."
    let addr_hex = addr_str.strip_prefix("0x").unwrap_or(addr_str);
    hex::decode(addr_hex)
        .with_context(|| format!("Failed to decode address field '{}'", field_name))
}

/// Parse bytes field (vector<u8>) from Move object JSON.
fn parse_bytes_field(fields: &serde_json::Value, field_name: &str) -> Result<[u8; 32]> {
    let bytes_array = fields
        .get(field_name)
        .and_then(|v| v.as_array())
        .ok_or_else(|| anyhow!("Missing or invalid '{}' field", field_name))?;

    if bytes_array.len() != 32 {
        return Err(anyhow!(
            "Field '{}' expected 32 bytes, got {}",
            field_name,
            bytes_array.len()
        ));
    }

    let mut result = [0u8; 32];
    for (i, byte_val) in bytes_array.iter().enumerate() {
        result[i] = byte_val
            .as_u64()
            .ok_or_else(|| anyhow!("Invalid byte value in '{}'", field_name))?
            as u8;
    }

    Ok(result)
}

/// Parse u64 field from Move object JSON.
fn parse_u64_field(fields: &serde_json::Value, field_name: &str) -> Result<u64> {
    fields
        .get(field_name)
        .and_then(|v| {
            // Sui JSON can represent u64 as either number or string
            v.as_u64().or_else(|| v.as_str()?.parse().ok())
        })
        .ok_or_else(|| anyhow!("Missing or invalid '{}' field", field_name))
}

/// Parse bool field from Move object JSON.
fn parse_bool_field(fields: &serde_json::Value, field_name: &str) -> Result<bool> {
    fields
        .get(field_name)
        .and_then(|v| v.as_bool())
        .ok_or_else(|| anyhow!("Missing or invalid '{}' field", field_name))
}
