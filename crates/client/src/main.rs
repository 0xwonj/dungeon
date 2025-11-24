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
    use client_bootstrap::{RuntimeBuilder, RuntimeConfig, list_sessions, load_latest_state};
    use client_frontend_cli::{
        CliConfig, CliFrontend, FrontendConfig, StartChoice, logging, show_start_screen,
    };
    use dungeon_client::Client;

    // 1. Load configuration from environment
    let mut runtime_config = RuntimeConfig::from_env();
    let frontend_config = FrontendConfig::from_env();
    let cli_config = CliConfig::from_env();

    // 2. Show start screen and determine session
    let (session_id, initial_state) = {
        use client_frontend_cli::presentation::terminal;

        // Initialize terminal for start screen
        let mut terminal = terminal::init()?;

        // List existing sessions
        let save_dir = runtime_config
            .save_data_dir
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("Save data directory not configured"))?;
        let sessions = list_sessions(save_dir).unwrap_or_default();

        // Show start screen
        let choice = show_start_screen(&mut terminal, &sessions)?;

        // Restore terminal before continuing (must be done before scope ends)
        terminal::restore()?;

        match choice {
            StartChoice::NewGame => {
                // Generate new session ID (timestamp-based)
                let session_id = runtime_config.session_id.clone().unwrap_or_else(|| {
                    use std::time::{SystemTime, UNIX_EPOCH};
                    let timestamp = SystemTime::now()
                        .duration_since(UNIX_EPOCH)
                        .unwrap()
                        .as_secs();
                    format!("session_{}", timestamp)
                });
                (session_id, None)
            }
            StartChoice::Continue(session_idx) => {
                // Get selected session
                let selected_session = sessions
                    .get(session_idx)
                    .ok_or_else(|| anyhow::anyhow!("Invalid session index: {}", session_idx))?;

                // Load latest state from that session
                let (_nonce, state) = load_latest_state(save_dir, &selected_session.session_id)?
                    .ok_or_else(|| {
                        anyhow::anyhow!(
                            "Failed to load state from session {}",
                            selected_session.session_id
                        )
                    })?;

                (selected_session.session_id.clone(), Some(state))
            }
        }
    };

    // Update runtime config with chosen session ID
    runtime_config.session_id = Some(session_id);

    // 3. Setup logging (now with final session ID)
    logging::setup_logging(&runtime_config.session_id)?;

    tracing::info!("Starting Dungeon client");
    tracing::info!("Session ID: {:?}", runtime_config.session_id);
    tracing::info!("ZK proving: {}", runtime_config.enable_proving);
    tracing::info!("Persistence: {}", runtime_config.enable_persistence);

    // 3. Optional: Initialize blockchain clients for manual workflow (Sui feature only)
    #[cfg(feature = "sui")]
    let blockchain_clients = {
        use client_blockchain_sui::{SuiBlockchainClient, SuiConfig, WalrusClient, WalrusNetwork};

        tracing::debug!("Sui feature enabled, attempting to load Sui configuration...");

        match SuiConfig::from_env() {
            Ok(sui_config) => {
                tracing::info!(
                    "Sui configuration loaded: network={}",
                    sui_config.network_name()
                );

                match SuiBlockchainClient::new(sui_config).await {
                    Ok(sui_client) => {
                        tracing::info!("Sui blockchain client initialized successfully");

                        // Create Walrus client (testnet for now)
                        let walrus_client = WalrusClient::new(WalrusNetwork::Testnet);
                        tracing::info!("Walrus client initialized successfully");

                        Some(runtime::BlockchainClients::new(sui_client, walrus_client))
                    }
                    Err(e) => {
                        tracing::warn!(
                            "Failed to initialize Sui/Walrus clients: {}. Continuing without blockchain integration.",
                            e
                        );
                        None
                    }
                }
            }
            Err(e) => {
                tracing::warn!(
                    "Sui configuration not found: {}. Continuing without blockchain integration.",
                    e
                );
                None
            }
        }
    };

    #[cfg(not(feature = "sui"))]
    {
        tracing::debug!("Blockchain integration disabled (sui feature not enabled)");
    }

    // 4. Build Runtime (independent layer)
    tracing::debug!("Building runtime...");
    let mut runtime_builder = RuntimeBuilder::new().config(runtime_config);

    // Set initial state if resuming
    if let Some(state) = initial_state {
        tracing::info!("Using loaded state for session resumption");
        runtime_builder = runtime_builder.initial_state(state);
    }

    #[cfg(feature = "sui")]
    let runtime_builder = if let Some(clients) = blockchain_clients {
        runtime_builder.blockchain_clients(clients)
    } else {
        runtime_builder
    };

    let setup = runtime_builder.build().await?;

    tracing::info!("Runtime built successfully");

    // 5. Build Frontend (independent layer)
    tracing::debug!("Building CLI frontend...");
    let frontend = CliFrontend::new(frontend_config, cli_config, setup.oracles.clone());

    // 6. Build Client (composition layer)
    let builder = Client::builder().runtime(setup.runtime).frontend(frontend);

    // 7. Build and run
    let client = builder.build()?;

    tracing::info!("Client assembled, starting...");
    client.run().await?;

    tracing::info!("Client shutdown complete");
    Ok(())
}
