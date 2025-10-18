//! Terminal client entry point.
mod app;
mod cursor;
mod input;
mod presentation;
mod state;

use anyhow::Result;
use app::CliApp;
use client_bootstrap::config::CliConfig;
use client_core::frontend::FrontendApp;
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::util::SubscriberInitExt;

#[tokio::main]
async fn main() -> Result<()> {
    // Load .env file if it exists (silently ignore if not found)
    let _ = dotenvy::dotenv();

    let config = CliConfig::from_env();

    // Setup logging: both to stderr and to file
    setup_logging(&config.session_id)?;

    CliApp::builder(config).build().await?.run().await
}

/// Setup logging to both stderr and file
fn setup_logging(session_id: &Option<String>) -> Result<()> {
    use std::time::{SystemTime, UNIX_EPOCH};

    // Determine log directory based on OS
    let log_dir = get_log_directory();

    // Create session ID if not provided
    let session_id = session_id.clone().unwrap_or_else(|| {
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();
        format!("session_{}", timestamp)
    });

    // Create session-specific log directory
    let session_log_dir = log_dir.join(&session_id);
    std::fs::create_dir_all(&session_log_dir)?;

    // Setup file appender
    let file_appender = tracing_appender::rolling::never(&session_log_dir, "client.log");
    let (non_blocking_file, _guard) = tracing_appender::non_blocking(file_appender);

    // Create env filter
    let env_filter = tracing_subscriber::EnvFilter::from_default_env()
        .add_directive(tracing::Level::INFO.into());

    // Setup file layer (always enabled)
    let file_layer = tracing_subscriber::fmt::layer()
        .with_writer(non_blocking_file)
        .with_ansi(true); // Enable ANSI codes in file for colorized tail-logs

    // Initialize subscriber with ONLY file layer (no stderr for TUI)
    tracing_subscriber::registry()
        .with(env_filter)
        .with(file_layer)
        .init();

    // Leak the guard to keep file writer alive
    std::mem::forget(_guard);

    tracing::info!("Logging initialized: session={}", session_id);
    tracing::info!("Log file: {}/client.log", session_log_dir.display());

    Ok(())
}

/// Get the platform-specific log directory
fn get_log_directory() -> std::path::PathBuf {
    #[cfg(target_os = "macos")]
    {
        if let Some(home) = std::env::var_os("HOME") {
            let mut path = std::path::PathBuf::from(home);
            path.push("Library");
            path.push("Caches");
            path.push("dungeon");
            path.push("logs");
            return path;
        }
    }

    #[cfg(target_os = "linux")]
    {
        if let Some(xdg_cache) = std::env::var_os("XDG_CACHE_HOME") {
            let mut path = std::path::PathBuf::from(xdg_cache);
            path.push("dungeon");
            path.push("logs");
            return path;
        } else if let Some(home) = std::env::var_os("HOME") {
            let mut path = std::path::PathBuf::from(home);
            path.push(".cache");
            path.push("dungeon");
            path.push("logs");
            return path;
        }
    }

    #[cfg(target_os = "windows")]
    {
        if let Some(local_appdata) = std::env::var_os("LOCALAPPDATA") {
            let mut path = std::path::PathBuf::from(local_appdata);
            path.push("dungeon");
            path.push("logs");
            return path;
        }
    }

    // Fallback
    std::path::PathBuf::from("/tmp/dungeon/logs")
}
