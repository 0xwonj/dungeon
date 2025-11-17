//! Dungeon game client binary.
//!
//! Main entry point for the Dungeon game client.
//!
//! # Features
//!
//! - `cli`: Terminal-based UI (default)
//! - `risc0`, `sp1`, `stub`, `arkworks`: ZK backend selection
//!
//! # Examples
//!
//! ```bash
//! cargo run -p dungeon-client --features "cli,sp1"
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
        compile_error!("CLI frontend feature must be enabled");
    }

    Ok(())
}

#[cfg(feature = "cli")]
async fn run_cli() -> Result<()> {
    use client_frontend_cli::{CliApp, CliConfig, FrontendConfig, RuntimeConfig, logging};
    use client_frontend_core::frontend::FrontendApp;

    // Load configuration from environment
    let runtime_config = RuntimeConfig::from_env();
    let frontend_config = FrontendConfig::from_env();
    let cli_config = CliConfig::from_env();

    // Setup logging
    logging::setup_logging(&runtime_config.session_id)?;

    // Build and run the app
    let app = CliApp::builder(runtime_config, frontend_config, cli_config)
        .build()
        .await?;

    app.run().await
}
