//! Sui blockchain configuration and deployment.

pub mod deployment;
pub mod network;

// Re-export commonly used items
pub use deployment::DeploymentInfo;
pub use network::{SuiConfig, SuiNetwork};
