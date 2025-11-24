//! Sui key generation command.

use std::path::PathBuf;

use anyhow::{Context, Result, anyhow};
use clap::Parser;
use sui_keys::keystore::{AccountKeystore, FileBasedKeystore, GenerateOptions, LocalGenerate};
use sui_types::crypto::{EncodeDecodeBase64, SignatureScheme};

/// Generate a new Sui address and private key
#[derive(Debug, Parser)]
pub struct Keygen {
    /// Alias for the generated key (optional, human-readable name)
    #[arg(short, long)]
    pub alias: Option<String>,

    /// Key scheme to use (ed25519, secp256k1, secp256r1)
    #[arg(short, long, default_value = "ed25519")]
    pub scheme: String,

    /// Show private key in output (WARNING: sensitive!)
    #[arg(long)]
    pub show_private_key: bool,
}

impl Keygen {
    /// Execute the keygen command.
    pub fn execute(&self) -> Result<()> {
        // Run async keygen in blocking context
        tokio::runtime::Runtime::new()?.block_on(self.execute_async())
    }

    /// Async implementation of keygen.
    async fn execute_async(&self) -> Result<()> {
        println!("🔑 Generating new Sui key...");
        println!();

        // Parse signature scheme
        let scheme = self.parse_scheme()?;

        // Get keystore path
        let keystore_path = self.keystore_path()?;

        // Load or create keystore
        let mut keystore = FileBasedKeystore::load_or_create(&keystore_path)
            .context("Failed to load or create keystore")?;

        if keystore.addresses().is_empty() {
            println!("📁 Creating new keystore at: {}", keystore_path.display());
        }

        // Generate new key
        let generate_opts = GenerateOptions::Local(LocalGenerate {
            key_scheme: scheme,
            derivation_path: None,
            word_length: None,
        });

        let generated = keystore
            .generate(self.alias.clone(), generate_opts)
            .await
            .context("Failed to generate new key")?;

        let address = generated.address;

        println!("✅ Key generated successfully!");
        println!();
        println!("Address: {}", address);

        if let Some(ref alias) = self.alias {
            println!("Alias:   {}", alias);
        }

        println!("Scheme:  {:?}", scheme);
        println!();

        // Show private key if requested (WARNING: sensitive!)
        if self.show_private_key {
            let keypair = keystore
                .export(&address)
                .context("Failed to export private key")?;

            println!("⚠️  WARNING: Do NOT share your private key!");
            println!();
            println!("Private key (Bech32):");
            println!("{}", keypair.encode_base64());
            println!();
        }

        println!("Keystore location: {}", keystore_path.display());
        println!();
        println!("To use this address:");
        if let Some(ref alias) = self.alias {
            println!("  export SUI_ACTIVE_ALIAS={}", alias);
        } else {
            println!("  # This is the default address (first in keystore)");
        }

        Ok(())
    }

    /// Parse signature scheme from string.
    fn parse_scheme(&self) -> Result<SignatureScheme> {
        match self.scheme.to_lowercase().as_str() {
            "ed25519" => Ok(SignatureScheme::ED25519),
            "secp256k1" => Ok(SignatureScheme::Secp256k1),
            "secp256r1" => Ok(SignatureScheme::Secp256r1),
            other => Err(anyhow!(
                "Invalid signature scheme: {}. Must be ed25519, secp256k1, or secp256r1",
                other
            )),
        }
    }

    /// Get Sui keystore path (~/.sui/sui_config/sui.keystore).
    fn keystore_path(&self) -> Result<PathBuf> {
        let home = directories::BaseDirs::new()
            .ok_or_else(|| anyhow!("Could not determine home directory"))?
            .home_dir()
            .to_path_buf();
        Ok(home.join(".sui").join("sui_config").join("sui.keystore"))
    }
}
