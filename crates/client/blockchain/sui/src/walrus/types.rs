//! Walrus type definitions.
//!
//! # Resilience to API Changes
//!
//! These types are designed to be resilient to Walrus API changes:
//! - Most fields are `Option<T>` with `#[serde(default)]` to handle missing fields
//! - `#[serde(flatten)]` catches unknown fields in `extra: HashMap<String, Value>`
//! - Only critical fields (`id`, `blob_id`) are required
//!
//! This approach prevents deserialization failures when:
//! - Walrus adds new fields (captured in `extra`)
//! - Walrus makes fields nullable
//! - Walrus changes field types (gracefully defaults to 0 or None)

use serde::{Deserialize, Serialize};

/// Walrus network configuration.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Network {
    /// Testnet (public)
    #[default]
    Testnet,
    /// Mainnet (future)
    Mainnet,
}

impl Network {
    /// Get publisher URL for this network.
    pub fn publisher_url(&self) -> &'static str {
        match self {
            Network::Testnet => "https://publisher.walrus-testnet.walrus.space",
            Network::Mainnet => "https://publisher.walrus-mainnet.walrus.space", // Future
        }
    }

    /// Get aggregator URL for this network.
    pub fn aggregator_url(&self) -> &'static str {
        match self {
            Network::Testnet => "https://aggregator.walrus-testnet.walrus.space",
            Network::Mainnet => "https://aggregator.walrus-mainnet.walrus.space", // Future
        }
    }
}

/// Response from storing a blob in Walrus.
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum BlobResponse {
    /// Blob was newly created
    NewlyCreated(Box<BlobInfo>),
    /// Blob already exists (cached)
    AlreadyCertified { blob_id: String, end_epoch: u64 },
}

impl BlobResponse {
    /// Get blob object (only available for newly created blobs).
    pub fn blob_object(&self) -> Option<&BlobObject> {
        match self {
            BlobResponse::NewlyCreated(info) => Some(&info.blob_object),
            BlobResponse::AlreadyCertified { .. } => None,
        }
    }

    /// Get blob ID.
    pub fn blob_id(&self) -> &str {
        match self {
            BlobResponse::NewlyCreated(info) => &info.blob_object.blob_id,
            BlobResponse::AlreadyCertified { blob_id, .. } => blob_id,
        }
    }
}

/// Information about a newly created blob.
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BlobInfo {
    /// Blob object metadata
    pub blob_object: BlobObject,

    /// Storage cost in MIST
    #[serde(default)]
    pub cost: u64,

    /// Catch-all for unknown fields (resource_operation, etc.)
    #[serde(flatten)]
    pub extra: std::collections::HashMap<String, serde_json::Value>,
}

/// Walrus blob object (stored on Sui).
///
/// This represents the on-chain Sui object that references the off-chain blob data.
///
/// **Design**: Most fields are optional to handle API changes gracefully.
/// Only `id` and `blob_id` are required for core functionality.
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BlobObject {
    /// Sui object ID (used to reference blob in Move contracts) - REQUIRED
    pub id: String,

    /// Walrus blob ID (base64-encoded, used for HTTP retrieval) - REQUIRED
    pub blob_id: String,

    /// Blob size in bytes
    #[serde(default)]
    pub size: u64,

    /// Encoding type (e.g., "RS2" for Reed-Solomon erasure coding)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub encoding_type: Option<String>,

    /// Epoch when blob was registered
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub registered_epoch: Option<u64>,

    /// Epoch when blob was certified (optional, null for newly uploaded blobs)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub certified_epoch: Option<u64>,

    /// Storage information (contains start_epoch and end_epoch)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub storage: Option<StorageInfo>,

    /// Whether the blob is deletable
    #[serde(default)]
    pub deletable: bool,

    /// Catch-all for unknown fields (prevents future API changes from breaking)
    #[serde(flatten)]
    pub extra: std::collections::HashMap<String, serde_json::Value>,
}

/// Storage information for a Walrus blob.
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct StorageInfo {
    /// Storage object ID
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,

    /// Epoch when storage starts
    #[serde(default)]
    pub start_epoch: u64,

    /// Epoch when storage expires
    #[serde(default)]
    pub end_epoch: u64,

    /// Total storage size (in some unit)
    #[serde(default)]
    pub storage_size: u64,

    /// Catch-all for unknown fields
    #[serde(flatten)]
    pub extra: std::collections::HashMap<String, serde_json::Value>,
}

impl BlobObject {
    /// Get Sui object ID.
    pub fn object_id(&self) -> &str {
        &self.id
    }

    /// Get Walrus blob ID (for HTTP retrieval).
    pub fn blob_id(&self) -> &str {
        &self.blob_id
    }
}
