//! Sui blockchain configuration.

use client_blockchain_core::BlockchainConfig;
use std::env;

/// Sui network types.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SuiNetwork {
    /// Sui mainnet
    Mainnet,
    /// Sui testnet
    Testnet,
    /// Local Sui network
    Local,
}

impl SuiNetwork {
    pub fn default_rpc_url(&self) -> &str {
        match self {
            SuiNetwork::Mainnet => "https://fullnode.mainnet.sui.io:443",
            SuiNetwork::Testnet => "https://fullnode.testnet.sui.io:443",
            SuiNetwork::Local => "http://127.0.0.1:9000",
        }
    }
}

/// Sui-specific configuration.
pub struct SuiConfig {
    /// Sui network to connect to
    pub network: SuiNetwork,

    /// Custom RPC endpoint URL (overrides network default)
    pub rpc_url: Option<String>,

    /// Package ID of the deployed game contract
    pub package_id: Option<String>,

    /// Gas budget for transactions (in MIST)
    pub gas_budget: u64,
}

impl SuiConfig {
    /// Create a new Sui configuration.
    pub fn new(network: SuiNetwork) -> Self {
        Self {
            network,
            rpc_url: None,
            package_id: None,
            gas_budget: 100_000_000, // 0.1 SUI
        }
    }

    /// Load configuration from environment variables.
    ///
    /// Environment variables:
    /// - `SUI_NETWORK` - Network name (mainnet, testnet, local) (default: testnet)
    /// - `SUI_RPC_URL` - Custom RPC endpoint URL
    /// - `SUI_PACKAGE_ID` - Deployed game package ID
    /// - `SUI_GAS_BUDGET` - Gas budget in MIST (default: 100000000)
    pub fn from_env() -> Result<Self, String> {
        let network = match env::var("SUI_NETWORK")
            .unwrap_or_else(|_| "testnet".to_string())
            .to_lowercase()
            .as_str()
        {
            "mainnet" => SuiNetwork::Mainnet,
            "testnet" => SuiNetwork::Testnet,
            "local" => SuiNetwork::Local,
            other => {
                return Err(format!(
                    "Invalid SUI_NETWORK: {}. Must be mainnet, testnet, or local",
                    other
                ));
            }
        };

        let rpc_url = env::var("SUI_RPC_URL").ok();
        let package_id = env::var("SUI_PACKAGE_ID").ok();

        let gas_budget = env::var("SUI_GAS_BUDGET")
            .ok()
            .and_then(|s| s.parse::<u64>().ok())
            .unwrap_or(100_000_000);

        Ok(Self {
            network,
            rpc_url,
            package_id,
            gas_budget,
        })
    }

    /// Set custom RPC URL.
    pub fn with_rpc_url(mut self, url: String) -> Self {
        self.rpc_url = Some(url);
        self
    }

    /// Set package ID.
    pub fn with_package_id(mut self, package_id: String) -> Self {
        self.package_id = Some(package_id);
        self
    }

    /// Set gas budget.
    pub fn with_gas_budget(mut self, budget: u64) -> Self {
        self.gas_budget = budget;
        self
    }

    /// Get the RPC URL (custom or default for network).
    pub fn get_rpc_url(&self) -> &str {
        self.rpc_url
            .as_deref()
            .unwrap_or_else(|| self.network.default_rpc_url())
    }
}

impl BlockchainConfig for SuiConfig {
    fn network_name(&self) -> &str {
        match self.network {
            SuiNetwork::Mainnet => "sui-mainnet",
            SuiNetwork::Testnet => "sui-testnet",
            SuiNetwork::Local => "sui-local",
        }
    }

    fn rpc_url(&self) -> &str {
        self.get_rpc_url()
    }

    fn validate(&self) -> Result<(), String> {
        // Validate RPC URL format
        let url = self.get_rpc_url();
        if !url.starts_with("http://") && !url.starts_with("https://") {
            return Err(format!("Invalid RPC URL format: {}", url));
        }

        // Validate gas budget
        if self.gas_budget == 0 {
            return Err("Gas budget must be greater than 0".to_string());
        }

        // Package ID is optional (may not be deployed yet)
        if let Some(ref pkg_id) = self.package_id {
            if pkg_id.is_empty() {
                return Err("Package ID cannot be empty".to_string());
            }
        }

        Ok(())
    }
}

impl Default for SuiConfig {
    fn default() -> Self {
        Self::new(SuiNetwork::Testnet)
    }
}
