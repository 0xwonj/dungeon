//! Sui blockchain client implementation.

use std::sync::Arc;

use anyhow::{Context, Result, anyhow};
use sui_keys::keystore::{AccountKeystore, FileBasedKeystore};
use sui_sdk::{SuiClient, SuiClientBuilder};
use sui_types::base_types::SuiAddress;
use sui_types::crypto::SuiKeyPair;

use crate::config::deployment::DeploymentInfo;
use crate::config::network::SuiConfig;
use crate::contracts::GameSessionContract;

/// Sui blockchain client.
///
/// Provides unified access to all Sui blockchain operations for the game.
pub struct SuiBlockchainClient {
    /// Configuration
    pub config: SuiConfig,

    /// Sui RPC client
    sui_client: Arc<SuiClient>,

    /// Keystore for transaction signing
    keystore: FileBasedKeystore,

    /// Active address (signer)
    active_address: SuiAddress,

    /// Game session contract client
    pub game_session: GameSessionContract,
}

impl SuiBlockchainClient {
    /// Create a new Sui blockchain client.
    ///
    /// # Arguments
    ///
    /// * `config` - Sui configuration (network, package ID, etc.)
    ///
    /// # Returns
    ///
    /// Configured client ready to interact with Sui network.
    ///
    /// # Errors
    ///
    /// Returns error if configuration is invalid or Sui SDK initialization fails.
    pub async fn new(config: SuiConfig) -> Result<Self> {
        // Validate configuration
        config
            .validate()
            .map_err(|e| anyhow!("Invalid configuration: {}", e))?;

        tracing::info!(
            "Initializing Sui client for network: {}",
            config.network_name()
        );

        // Initialize Sui SDK client
        let sui_client = Arc::new(
            SuiClientBuilder::default()
                .build(config.get_rpc_url())
                .await
                .context("Failed to connect to Sui RPC")?,
        );

        tracing::debug!("Connected to Sui RPC: {}", config.get_rpc_url());

        // Load keystore from default Sui CLI location (~/.sui/sui_config/sui.keystore)
        let keystore_path = sui_config_dir()?.join(SUI_KEYSTORE_FILENAME);

        tracing::debug!("Keystore path: {}", keystore_path.display());

        // Load keystore (or create empty if missing)
        let keystore = FileBasedKeystore::load_or_create(&keystore_path).context(format!(
            "Failed to access keystore at {}",
            keystore_path.display()
        ))?;

        // Fail if keystore is empty - we don't auto-generate keys
        if keystore.addresses().is_empty() {
            return Err(anyhow!(
                "No addresses found in keystore at {}. \
                 Generate a key first with 'sui client new-address ed25519' or 'cargo xtask sui keygen'.",
                keystore_path.display()
            ));
        }

        // Get active address (from alias or first address)
        let active_address = if let Ok(alias) = std::env::var("SUI_ACTIVE_ALIAS") {
            // Use address by alias
            *keystore
                .addresses_with_alias()
                .iter()
                .find(|(_, a)| a.alias == alias)
                .ok_or_else(|| anyhow!("Address with alias '{}' not found in keystore", alias))?
                .0
        } else {
            // Use first address as default
            keystore
                .addresses()
                .first()
                .copied()
                .ok_or_else(|| anyhow!(
                    "No addresses found in keystore. Generate a key first with 'cargo xtask sui keygen'."
                ))?
        };

        tracing::info!("Using address: {}", active_address);

        // Get package ID from config
        let package_id = config.package_id.clone().ok_or_else(|| {
            anyhow!("Package ID not configured. Run 'cargo xtask sui deploy' first.")
        })?;

        // Load VK from deployment info if available
        let _network = match config.network {
            crate::config::SuiNetwork::Mainnet => "mainnet",
            crate::config::SuiNetwork::Testnet => "testnet",
            crate::config::SuiNetwork::Local => "local",
        };

        let vk_object_id = DeploymentInfo::from_env().ok().and_then(|d| d.vk_object_id);

        if let Some(ref vk_id) = vk_object_id {
            tracing::info!("Loaded VK object ID from deployment: {}", vk_id);
        } else {
            tracing::warn!(
                "VK object ID not found in deployment info. \
                 Proof verification will fail until VK is configured. \
                 Run 'cargo xtask sui setup' to register VK."
            );
        }

        // Create game session contract client
        let game_session = GameSessionContract::new(package_id, vk_object_id);

        Ok(Self {
            config,
            sui_client,
            keystore,
            active_address,
            game_session,
        })
    }

    /// Create client with default configuration (testnet).
    pub async fn new_with_defaults() -> Result<Self> {
        Self::new(SuiConfig::default()).await
    }

    /// Set verifying key object ID.
    ///
    /// This should be called after deploying the VK to the network.
    pub fn set_verifying_key(&mut self, vk_id: String) {
        self.game_session.set_vk(vk_id);
    }

    /// Get network name.
    pub fn network(&self) -> &str {
        match self.config.network {
            crate::config::SuiNetwork::Mainnet => "mainnet",
            crate::config::SuiNetwork::Testnet => "testnet",
            crate::config::SuiNetwork::Local => "local",
        }
    }

    /// Get active Sui address.
    pub fn active_address(&self) -> SuiAddress {
        self.active_address
    }

    /// Get Sui client reference.
    pub fn sui_client(&self) -> &SuiClient {
        &self.sui_client
    }

    /// Get keypair for signing transactions.
    #[allow(dead_code)]
    fn get_key_pair(&self) -> Result<&SuiKeyPair> {
        self.keystore
            .export(&self.active_address)
            .context("Failed to get keypair for active address")
    }

    // ========================================================================
    // Convenience Wrappers (Delegate to GameSessionContract with DI)
    // ========================================================================

    /// Create a new game session on-chain.
    ///
    /// Convenience wrapper that injects SDK dependencies into GameSessionContract.
    ///
    /// # Arguments
    ///
    /// * `oracle_root` - Content hash of oracle data
    /// * `initial_state_root` - Initial game state root
    /// * `seed_commitment` - RNG seed commitment
    ///
    /// # Returns
    ///
    /// SessionId of the created on-chain object.
    pub async fn create_session(
        &self,
        oracle_root: [u8; 32],
        initial_state_root: [u8; 32],
        seed_commitment: [u8; 32],
    ) -> Result<crate::core::SessionId> {
        self.game_session
            .create(
                &self.sui_client,
                &self.keystore,
                self.active_address,
                self.config.gas_budget,
                oracle_root,
                initial_state_root,
                seed_commitment,
            )
            .await
    }

    /// Update session with ZK proof.
    ///
    /// Convenience wrapper that injects SDK dependencies into GameSessionContract.
    ///
    /// # Arguments
    ///
    /// * `session_id` - Session object ID to update
    /// * `proof` - Proof submission containing ZK proof and journal
    ///
    /// # Returns
    ///
    /// Transaction digest of the update transaction.
    pub async fn update_session(
        &self,
        session_id: &crate::core::SessionId,
        proof: crate::core::ProofSubmission,
        blob_object_id: &str,
    ) -> Result<crate::core::TxDigest> {
        self.game_session
            .update(
                &self.sui_client,
                &self.keystore,
                self.active_address,
                self.config.gas_budget,
                session_id,
                proof,
                blob_object_id,
            )
            .await
    }

    /// Update session state with ZK proof without Walrus blob (testing only).
    ///
    /// Convenience wrapper that injects SDK dependencies into GameSessionContract.
    /// This bypasses Walrus blob upload and directly calls `update_without_blob`.
    ///
    /// # Arguments
    ///
    /// * `session_id` - Session object ID to update
    /// * `proof` - Proof submission data (proof, journal_digest, actions_root, state_root, nonce)
    ///
    /// # Returns
    ///
    /// Transaction digest of the update transaction.
    pub async fn update_session_without_blob(
        &self,
        session_id: &crate::core::SessionId,
        proof: crate::core::ProofSubmission,
    ) -> Result<crate::core::TxDigest> {
        self.game_session
            .update_without_blob(
                &self.sui_client,
                &self.keystore,
                self.active_address,
                self.config.gas_budget,
                session_id,
                proof,
            )
            .await
    }

    /// Finalize a game session.
    ///
    /// Convenience wrapper that injects SDK dependencies into GameSessionContract.
    ///
    /// # Arguments
    ///
    /// * `session_id` - Session object ID to finalize
    ///
    /// # Returns
    ///
    /// Transaction digest of the finalize transaction.
    pub async fn finalize_session(
        &self,
        session_id: &crate::core::SessionId,
    ) -> Result<crate::core::TxDigest> {
        self.game_session
            .finalize(
                &self.sui_client,
                &self.keystore,
                self.active_address,
                self.config.gas_budget,
                session_id,
            )
            .await
    }

    /// Get session state from blockchain.
    ///
    /// Convenience wrapper that injects SDK dependencies into GameSessionContract.
    ///
    /// # Arguments
    ///
    /// * `session_id` - Session object ID to query
    ///
    /// # Returns
    ///
    /// GameSession struct with all fields from on-chain data.
    pub async fn get_session(
        &self,
        session_id: &crate::core::SessionId,
    ) -> Result<crate::contracts::GameSession> {
        self.game_session.get(&self.sui_client, session_id).await
    }

    /// Get current state root for a session.
    pub async fn get_state_root(
        &self,
        session_id: &crate::core::SessionId,
    ) -> Result<crate::core::StateRoot> {
        self.game_session
            .get_state_root(&self.sui_client, session_id)
            .await
    }

    /// Get current nonce for a session.
    pub async fn get_nonce(&self, session_id: &crate::core::SessionId) -> Result<u64> {
        self.game_session
            .get_nonce(&self.sui_client, session_id)
            .await
    }

    /// Check if session is active (not finalized).
    pub async fn is_session_active(&self, session_id: &crate::core::SessionId) -> Result<bool> {
        self.game_session
            .is_active(&self.sui_client, session_id)
            .await
    }
}

// ============================================================================
// Helper Functions
// ============================================================================

/// Default Sui keystore filename
const SUI_KEYSTORE_FILENAME: &str = "sui.keystore";

/// Get Sui config directory (~/.sui/sui_config/)
fn sui_config_dir() -> Result<std::path::PathBuf> {
    let home = dirs::home_dir().ok_or_else(|| anyhow!("Could not determine home directory"))?;
    Ok(home.join(".sui").join("sui_config"))
}
