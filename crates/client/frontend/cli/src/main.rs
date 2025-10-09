//! Terminal client entry point.
mod app;
mod input;
mod presentation;

use anyhow::Result;
use app::CliApp;
use client_core::config::CliConfig;
use frontend_core::frontend::FrontendApp;

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::from_default_env()
                .add_directive(tracing::Level::INFO.into()),
        )
        .with_writer(std::io::stderr)
        .init();

    let config = CliConfig::from_env();

    CliApp::builder(config).build().await?.run().await
}
