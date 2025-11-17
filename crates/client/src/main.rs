//! Dungeon game client binary.
//!
//! This is a composable binary that can be built with different features:
//! - Frontend: CLI (more to come)
//! - Blockchain: Sui (optional)
//! - ZK backend: RISC0, SP1, Stub, Arkworks
//!
//! # Examples
//!
//! ```bash
//! # CLI only, stub prover (fast development)
//! cargo run -p dungeon-client --features "frontend-cli,zkvm-stub"
//!
//! # CLI + Sui blockchain, SP1 prover
//! cargo run -p dungeon-client --features "frontend-cli,blockchain-sui,zkvm-sp1"
//! ```

use anyhow::Result;

#[tokio::main]
async fn main() -> Result<()> {
    // Load .env file if it exists (silently ignore if not found)
    let _ = dotenvy::dotenv();

    // Run the selected frontend
    #[cfg(feature = "frontend-cli")]
    {
        run_cli().await?;
    }

    #[cfg(feature = "frontend-gui")]
    {
        run_gui().await?;
    }

    #[cfg(not(any(feature = "frontend-cli", feature = "frontend-gui")))]
    {
        compile_error!("At least one frontend feature must be enabled (frontend-cli, frontend-gui)");
    }

    Ok(())
}

#[cfg(feature = "frontend-cli")]
async fn run_cli() -> Result<()> {
    use client_frontend_cli::{logging, CliApp, CliConfig, FrontendConfig, RuntimeConfig};
    use client_frontend_core::frontend::FrontendApp;

    // Load configuration from environment
    let runtime_config = RuntimeConfig::from_env();
    let frontend_config = FrontendConfig::from_env();
    let cli_config = CliConfig::from_env();

    // Setup logging
    logging::setup_logging(&runtime_config.session_id)?;

    // Build CLI app
    let builder = CliApp::builder(runtime_config, frontend_config, cli_config);

    // Optionally attach blockchain client
    #[cfg(feature = "blockchain-sui")]
    {
        use client_blockchain_core::BlockchainClient;
        use client_blockchain_sui::{SuiBlockchainClient, SuiConfig};

        match SuiConfig::from_env() {
            Ok(sui_config) => {
                match SuiBlockchainClient::new(sui_config).await {
                    Ok(sui_client) => {
                        tracing::info!("Sui blockchain client initialized");
                        // TODO: Attach to builder when CliApp supports blockchain integration
                        // builder = builder.blockchain(sui_client);
                        let _ = sui_client; // Silence unused warning
                    }
                    Err(e) => {
                        tracing::warn!("Failed to initialize Sui client: {}", e);
                        tracing::warn!("Continuing without blockchain integration");
                    }
                }
            }
            Err(e) => {
                tracing::warn!("Failed to load Sui config: {}", e);
                tracing::warn!("Continuing without blockchain integration");
            }
        }
    }

    // Build and run
    let app = builder.build().await?;
    app.run().await
}

#[cfg(feature = "frontend-gui")]
async fn run_gui() -> Result<()> {
    // TODO: Implement GUI frontend
    anyhow::bail!("GUI frontend not implemented yet");
}
