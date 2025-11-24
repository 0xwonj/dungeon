//! Walrus HTTP client implementation.

use anyhow::{Context, Result, anyhow};
use reqwest;

use super::types::{BlobObject, BlobResponse, Network};

/// Walrus storage client using HTTP API.
///
/// This client provides simple blob storage/retrieval operations using
/// Walrus's public HTTP endpoints.
pub struct WalrusClient {
    /// Publisher endpoint (for storing blobs)
    publisher_url: String,

    /// Aggregator endpoint (for retrieving blobs)
    aggregator_url: String,

    /// HTTP client
    http_client: reqwest::Client,

    /// Network
    network: Network,
}

impl WalrusClient {
    /// Create client for Walrus testnet.
    pub fn testnet() -> Self {
        Self::new(Network::Testnet)
    }

    /// Create client for specific network.
    pub fn new(network: Network) -> Self {
        Self {
            publisher_url: network.publisher_url().to_string(),
            aggregator_url: network.aggregator_url().to_string(),
            http_client: reqwest::Client::new(),
            network,
        }
    }

    /// Store blob in Walrus and return Blob object metadata.
    ///
    /// # Arguments
    ///
    /// * `data` - Blob data to store
    /// * `epochs` - Number of epochs to store blob
    /// * `recipient` - Sui address to receive the Blob object (optional)
    ///
    /// # Returns
    ///
    /// BlobObject with Sui object ID and Walrus blob ID
    ///
    /// # Errors
    ///
    /// Returns error if:
    /// - Network request fails
    /// - Walrus returns error response
    /// - Response parsing fails
    pub async fn store_blob(
        &self,
        data: &[u8],
        epochs: u64,
        recipient: Option<&str>,
    ) -> Result<BlobObject> {
        // Build URL with query parameters
        let mut url = format!("{}/v1/blobs?epochs={}", self.publisher_url, epochs);

        // Add recipient address if specified
        if let Some(addr) = recipient {
            url.push_str(&format!("&send_object_to={}", addr));
        }

        tracing::debug!(
            "Uploading blob to Walrus: {} bytes, {} epochs, recipient={:?}",
            data.len(),
            epochs,
            recipient
        );

        let response = self
            .http_client
            .put(&url)
            .header("Content-Type", "application/octet-stream")
            .body(data.to_vec())
            .send()
            .await
            .context("Failed to send blob upload request to Walrus")?;

        let status = response.status();
        if !status.is_success() {
            let error_text = response
                .text()
                .await
                .unwrap_or_else(|_| "Unknown error".to_string());
            return Err(anyhow!(
                "Walrus upload failed with status {}: {}",
                status,
                error_text
            ));
        }

        // Get response text first for better error reporting
        let response_text = response
            .text()
            .await
            .context("Failed to read Walrus response body")?;

        tracing::debug!("Walrus response: {}", response_text);

        // Parse response
        let blob_response: BlobResponse =
            serde_json::from_str(&response_text).with_context(|| {
                format!(
                    "Failed to parse Walrus upload response. Raw response: {}",
                    response_text
                )
            })?;

        match blob_response {
            BlobResponse::NewlyCreated(info) => {
                tracing::info!(
                    "✓ Blob uploaded to Walrus: {} (size: {} bytes, cost: {} MIST)",
                    info.blob_object.blob_id,
                    info.blob_object.size,
                    info.cost
                );
                Ok(info.blob_object)
            }
            BlobResponse::AlreadyCertified { blob_id, end_epoch } => {
                tracing::info!(
                    "✓ Blob already exists in Walrus: {} (expires epoch: {})",
                    blob_id,
                    end_epoch
                );
                // For cached blobs, we don't have object ID
                // This is a limitation - we'd need to query Sui to get object ID
                Err(anyhow!(
                    "Blob already exists but object ID not available. \
                     Blob ID: {}. This is a known limitation - consider using a unique \
                     blob per upload or querying Sui for object ID.",
                    blob_id
                ))
            }
        }
    }

    /// Retrieve blob data by Walrus blob ID.
    ///
    /// # Arguments
    ///
    /// * `blob_id` - Walrus blob ID (base64-encoded)
    ///
    /// # Returns
    ///
    /// Blob data as bytes
    ///
    /// # Errors
    ///
    /// Returns error if:
    /// - Network request fails
    /// - Blob not found
    /// - Download fails
    pub async fn get_blob(&self, blob_id: &str) -> Result<Vec<u8>> {
        let url = format!("{}/v1/blobs/{}", self.aggregator_url, blob_id);

        tracing::debug!("Downloading blob from Walrus: {}", blob_id);

        let response = self
            .http_client
            .get(&url)
            .send()
            .await
            .context("Failed to send blob download request to Walrus")?;

        let status = response.status();
        if !status.is_success() {
            let error_text = response
                .text()
                .await
                .unwrap_or_else(|_| "Unknown error".to_string());
            return Err(anyhow!(
                "Walrus download failed with status {}: {}",
                status,
                error_text
            ));
        }

        let data = response
            .bytes()
            .await
            .context("Failed to read blob data from Walrus")?
            .to_vec();

        tracing::debug!("✓ Blob downloaded from Walrus: {} bytes", data.len());

        Ok(data)
    }

    /// Retrieve blob data by Sui object ID.
    ///
    /// This is useful when you have the Blob object ID from a transaction
    /// but not the Walrus blob ID.
    ///
    /// # Arguments
    ///
    /// * `object_id` - Sui object ID (hex-encoded)
    ///
    /// # Returns
    ///
    /// Blob data as bytes
    pub async fn get_blob_by_object_id(&self, object_id: &str) -> Result<Vec<u8>> {
        let url = format!(
            "{}/v1/blobs/by-object-id/{}",
            self.aggregator_url, object_id
        );

        tracing::debug!("Downloading blob by object ID: {}", object_id);

        let response = self
            .http_client
            .get(&url)
            .send()
            .await
            .context("Failed to send blob download request to Walrus")?;

        let status = response.status();
        if !status.is_success() {
            let error_text = response
                .text()
                .await
                .unwrap_or_else(|_| "Unknown error".to_string());
            return Err(anyhow!(
                "Walrus download failed with status {}: {}",
                status,
                error_text
            ));
        }

        let data = response
            .bytes()
            .await
            .context("Failed to read blob data from Walrus")?
            .to_vec();

        tracing::debug!("✓ Blob downloaded from Walrus: {} bytes", data.len());

        Ok(data)
    }

    /// Get network configuration.
    pub fn network(&self) -> Network {
        self.network
    }

    /// Get publisher URL.
    pub fn publisher_url(&self) -> &str {
        &self.publisher_url
    }

    /// Get aggregator URL.
    pub fn aggregator_url(&self) -> &str {
        &self.aggregator_url
    }
}

impl Default for WalrusClient {
    fn default() -> Self {
        Self::testnet()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_client_creation() {
        let client = WalrusClient::testnet();
        assert_eq!(client.network(), Network::Testnet);
        assert_eq!(
            client.publisher_url(),
            "https://publisher.walrus-testnet.walrus.space"
        );
        assert_eq!(
            client.aggregator_url(),
            "https://aggregator.walrus-testnet.walrus.space"
        );
    }
}
