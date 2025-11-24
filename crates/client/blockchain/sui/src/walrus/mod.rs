//! Walrus decentralized storage integration.
//!
//! This module provides integration with Walrus, a decentralized blob store
//! built on Sui for coordination and governance.
//!
//! ## Overview
//!
//! Walrus stores large binary files (blobs) in a decentralized network while
//! using Sui smart contracts for:
//! - Proof of availability
//! - Storage duration management
//! - Access control via Blob objects
//!
//! ## Integration Pattern
//!
//! We use Walrus HTTP API for simplicity and stability:
//! 1. Upload blob via Publisher endpoint → get Blob object ID
//! 2. Pass Blob object to Move contract (game_session::update)
//! 3. Contract verifies blob availability on-chain
//!
//! ## Modules
//!
//! - [`client`]: HTTP client for Walrus storage operations
//! - [`types`]: Walrus-specific types (BlobObject, BlobInfo, etc.)

pub mod client;
pub mod types;

// Re-export primary types
pub use client::WalrusClient;
pub use types::{BlobInfo, BlobObject, BlobResponse, Network};
