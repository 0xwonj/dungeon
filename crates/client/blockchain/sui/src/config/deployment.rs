//! Deployment information management.
//!
//! Deployment information is now stored in .env file using environment variables:
//! - SUI_NETWORK - Network name (testnet, mainnet, local)
//! - SUI_PACKAGE_ID - Deployed package ID
//! - SUI_VK_OBJECT_ID - Verifying key object ID
//! - SUI_SESSION_OBJECT_ID - Game session object ID

use std::env;
use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::PathBuf;

use anyhow::{Context, Result};

/// Sui deployment information.
///
/// Stores deployment artifacts and metadata.
/// This is now read from and written to .env file.
#[derive(Debug, Clone)]
pub struct DeploymentInfo {
    /// Network name (e.g., "testnet", "mainnet", "local")
    pub network: String,

    /// Deployed package ID (Move package)
    pub package_id: String,

    /// Verifying key object ID (on-chain VK for proof verification)
    pub vk_object_id: Option<String>,

    /// Game session object ID (for current active session)
    pub session_object_id: Option<String>,
}

impl DeploymentInfo {
    /// Create new deployment info.
    pub fn new(network: String, package_id: String) -> Self {
        Self {
            network,
            package_id,
            vk_object_id: None,
            session_object_id: None,
        }
    }

    /// Load deployment info from environment variables.
    ///
    /// Environment variables:
    /// - SUI_NETWORK - Network name (required)
    /// - SUI_PACKAGE_ID - Package ID (required)
    /// - SUI_VK_OBJECT_ID - VK object ID (optional)
    /// - SUI_SESSION_OBJECT_ID - Session object ID (optional)
    pub fn from_env() -> Result<Self> {
        let network =
            env::var("SUI_NETWORK").context("SUI_NETWORK environment variable not set")?;

        let package_id =
            env::var("SUI_PACKAGE_ID").context("SUI_PACKAGE_ID environment variable not set")?;

        let vk_object_id = env::var("SUI_VK_OBJECT_ID").ok();
        let session_object_id = env::var("SUI_SESSION_OBJECT_ID").ok();

        Ok(Self {
            network,
            package_id,
            vk_object_id,
            session_object_id,
        })
    }

    /// Update VK object ID.
    pub fn set_vk_object_id(&mut self, vk_object_id: String) {
        self.vk_object_id = Some(vk_object_id);
    }

    /// Update session object ID.
    pub fn set_session_object_id(&mut self, session_object_id: String) {
        self.session_object_id = Some(session_object_id);
    }

    /// Save deployment info to .env file.
    ///
    /// Appends or updates environment variables in .env file.
    pub fn save_to_env(&self) -> Result<()> {
        let env_path = PathBuf::from(".env");

        // Read existing .env content if it exists
        let existing_content = if env_path.exists() {
            fs::read_to_string(&env_path).context("Failed to read existing .env file")?
        } else {
            String::new()
        };

        // Parse existing variables
        let mut env_vars: Vec<(String, String)> = existing_content
            .lines()
            .filter(|line| !line.trim().is_empty() && !line.trim().starts_with('#'))
            .filter_map(|line| {
                let parts: Vec<&str> = line.splitn(2, '=').collect();
                if parts.len() == 2 {
                    Some((parts[0].trim().to_string(), parts[1].trim().to_string()))
                } else {
                    None
                }
            })
            .collect();

        // Update or add SUI_* variables
        Self::upsert_var(&mut env_vars, "SUI_NETWORK", &self.network);
        Self::upsert_var(&mut env_vars, "SUI_PACKAGE_ID", &self.package_id);

        if let Some(ref vk_id) = self.vk_object_id {
            Self::upsert_var(&mut env_vars, "SUI_VK_OBJECT_ID", vk_id);
        }

        if let Some(ref session_id) = self.session_object_id {
            Self::upsert_var(&mut env_vars, "SUI_SESSION_OBJECT_ID", session_id);
        }

        // Write back to .env file
        let mut file = OpenOptions::new()
            .write(true)
            .create(true)
            .truncate(true)
            .open(&env_path)
            .context("Failed to open .env file for writing")?;

        // Write comments if file is new
        if existing_content.is_empty() {
            writeln!(file, "# Sui blockchain deployment configuration")?;
            writeln!(file)?;
        }

        // Write all variables
        for (key, value) in env_vars {
            writeln!(file, "{}={}", key, value)?;
        }

        Ok(())
    }

    /// Helper to upsert a variable in the env_vars list.
    fn upsert_var(vars: &mut Vec<(String, String)>, key: &str, value: &str) {
        if let Some(pos) = vars.iter().position(|(k, _)| k == key) {
            vars[pos] = (key.to_string(), value.to_string());
        } else {
            vars.push((key.to_string(), value.to_string()));
        }
    }
}
