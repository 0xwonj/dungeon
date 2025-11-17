//! Dungeon game client binary.
//!
//! Main entry point for the Dungeon game client.
//!
//! # Architecture
//!
//! This binary is the composition root that assembles:
//! 1. Runtime (game logic) via RuntimeBuilder
//! 2. Frontend (UI) - CLI, GUI, etc.
//! 3. Blockchain (optional) - Sui, Ethereum, etc.
//!
//! All components are built independently and injected into the Client container.
//!
//! # Features
//!
//! - `cli`: Terminal-based UI (default)
//! - `sui`: Sui blockchain integration (optional)
//! - `risc0`, `sp1`, `stub`, `arkworks`: ZK backend selection
//!
//! # Examples
//!
//! ```bash
//! # CLI only with SP1 backend
//! cargo run -p dungeon-client --features "cli,sp1"
//!
//! # CLI + Sui blockchain with RISC0 backend
//! cargo run -p dungeon-client --features "cli,sui,risc0"
//! ```

use anyhow::Result;

#[tokio::main]
async fn main() -> Result<()> {
    dotenvy::dotenv().ok();

    #[cfg(feature = "cli")]
    {
        run_cli().await?;
    }

    #[cfg(not(feature = "cli"))]
    {
        compile_error!("At least one frontend feature must be enabled (cli, gui, etc.)");
    }

    Ok(())
}

/// Run the CLI frontend.
#[cfg(feature = "cli")]
async fn run_cli() -> Result<()> {
    use client_bootstrap::{RuntimeBuilder, RuntimeConfig};
    use client_frontend_cli::{CliConfig, CliFrontend, FrontendConfig, logging};
    use dungeon_client::Client;

    // 1. Load configuration from environment
    let runtime_config = RuntimeConfig::from_env();
    let frontend_config = FrontendConfig::from_env();
    let cli_config = CliConfig::from_env();

    // 2. Setup logging
    logging::setup_logging(&runtime_config.session_id)?;

    tracing::info!("Starting Dungeon client");
    tracing::info!("Session ID: {:?}", runtime_config.session_id);
    tracing::info!("ZK proving: {}", runtime_config.enable_proving);
    tracing::info!("Persistence: {}", runtime_config.enable_persistence);

    // 3. Build Runtime (independent layer)
    tracing::debug!("Building runtime...");
    let setup = RuntimeBuilder::new().config(runtime_config).build().await?;

    tracing::info!("Runtime built successfully");

    // 4. Build Frontend (independent layer)
    tracing::debug!("Building CLI frontend...");
    let frontend = CliFrontend::new(frontend_config, cli_config, setup.oracles.clone());

    // 5. Build Client (composition layer)
    #[cfg_attr(not(feature = "sui"), allow(unused_mut))]
    let mut builder = Client::builder().runtime(setup.runtime).frontend(frontend);

    // 6. Optional: Add Blockchain client
    #[cfg(feature = "sui")]
    {
        use client_blockchain_sui::{SuiBlockchainClient, SuiConfig};

        tracing::debug!("Sui feature enabled, attempting to load Sui configuration...");

        match SuiConfig::from_env() {
            Ok(sui_config) => {
                tracing::info!("Sui configuration loaded: network={}", sui_config.network());

                match SuiBlockchainClient::new(sui_config).await {
                    Ok(sui_client) => {
                        tracing::info!("Sui blockchain client initialized successfully");
                        builder = builder.blockchain(sui_client);
                    }
                    Err(e) => {
                        tracing::warn!(
                            "Failed to initialize Sui client: {}. Continuing without blockchain integration.",
                            e
                        );
                    }
                }
            }
            Err(e) => {
                tracing::warn!(
                    "Sui configuration not found: {}. Continuing without blockchain integration.",
                    e
                );
            }
        }
    }

    #[cfg(not(feature = "sui"))]
    {
        tracing::debug!("Blockchain integration disabled (sui feature not enabled)");
    }

    // 7. Build and run
    let client = builder.build()?;

    tracing::info!("Client assembled, starting...");
    client.run().await?;

    tracing::info!("Client shutdown complete");
    Ok(())
}
