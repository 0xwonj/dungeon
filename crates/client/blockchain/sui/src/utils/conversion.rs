//! Type conversion utilities for Sui blockchain.
//!
//! This module provides conversions between domain types and Sui-specific types.
//!
//! ## Conversion Categories
//!
//! 1. **Identifiers**: SessionId ↔ Sui ObjectID, TxDigest ↔ Sui Digest
//! 2. **Addresses**: Player addresses, contract addresses
//! 3. **Encoding**: Bytes ↔ Sui BCS encoding

use crate::core::error::SuiError;
use crate::core::types::{SessionId, TxDigest};

// ============================================================================
// Session ID Conversions (Adapter Pattern)
// ============================================================================

/// Convert SessionId to Sui object ID string.
pub fn session_id_to_object_id(session_id: &SessionId) -> &str {
    session_id.as_str()
}

/// Convert Sui object ID string to SessionId.
pub fn object_id_to_session_id(object_id: String) -> SessionId {
    SessionId::new(object_id)
}

// ============================================================================
// Transaction ID Conversions
// ============================================================================

/// Convert TxDigest to string.
pub fn tx_digest_to_string(tx_digest: &TxDigest) -> &str {
    tx_digest.as_str()
}

/// Convert string to TxDigest.
pub fn string_to_tx_digest(digest: String) -> TxDigest {
    TxDigest::new(digest)
}

// ============================================================================
// Address Conversions
// ============================================================================

/// Convert Sui address bytes to hex string.
///
/// Sui addresses are 32-byte identifiers. This converts the raw bytes
/// to a hex string with "0x" prefix.
///
/// # Arguments
///
/// * `address_bytes` - 32-byte Sui address
///
/// # Returns
///
/// Hex-encoded address string.
pub fn address_to_string(address_bytes: &[u8]) -> String {
    format!("0x{}", hex::encode(address_bytes))
}

/// Convert hex string to Sui address bytes.
///
/// # Arguments
///
/// * `address_str` - Hex-encoded address string
///
/// # Returns
///
/// 32-byte address.
///
/// # Errors
///
/// Returns error if:
/// - Hex decoding fails
/// - Decoded bytes are not exactly 32 bytes
pub fn string_to_address(address_str: &str) -> Result<Vec<u8>, String> {
    let hex_str = address_str.strip_prefix("0x").unwrap_or(address_str);
    let bytes = hex::decode(hex_str).map_err(|e| format!("Invalid address hex: {}", e))?;

    if bytes.len() != 32 {
        return Err(format!("Sui address must be 32 bytes, got {}", bytes.len()));
    }

    Ok(bytes)
}

// ============================================================================
// Encoding Utilities
// ============================================================================

/// Encode bytes as BCS (Binary Canonical Serialization).
///
/// BCS is Sui's standard serialization format for Move types.
///
/// # Arguments
///
/// * `_data` - Data to encode (placeholder generic)
///
/// # Returns
///
/// BCS-encoded bytes.
///
/// # Errors
///
/// Returns error if serialization fails.
pub fn encode_bcs<T: serde::Serialize>(_data: &T) -> Result<Vec<u8>, String> {
    // TODO: Implement with bcs crate
    // bcs::to_bytes(data).map_err(|e| format!("BCS encoding failed: {}", e))
    Err("BCS encoding not implemented".to_string())
}

/// Decode BCS bytes.
///
/// # Arguments
///
/// * `_bytes` - BCS-encoded bytes
///
/// # Returns
///
/// Deserialized data.
///
/// # Errors
///
/// Returns error if deserialization fails.
pub fn decode_bcs<T: serde::de::DeserializeOwned>(_bytes: &[u8]) -> Result<T, String> {
    // TODO: Implement with bcs crate
    // bcs::from_bytes(bytes).map_err(|e| format!("BCS decoding failed: {}", e))
    Err("BCS decoding not implemented".to_string())
}

// ============================================================================
// Validation Utilities
// ============================================================================

/// Validate Sui object ID format.
///
/// Checks that a string is a valid hex-encoded 32-byte object ID.
///
/// # Arguments
///
/// * `object_id` - Object ID string to validate
///
/// # Returns
///
/// `true` if valid, `false` otherwise.
pub fn is_valid_object_id(object_id: &str) -> bool {
    let hex_str = object_id.strip_prefix("0x").unwrap_or(object_id);
    hex::decode(hex_str).map(|b| b.len() == 32).unwrap_or(false)
}

/// Validate Sui transaction digest format.
///
/// Checks that a string is a valid hex-encoded 32-byte digest.
///
/// # Arguments
///
/// * `digest` - Digest string to validate
///
/// # Returns
///
/// `true` if valid, `false` otherwise.
pub fn is_valid_digest(digest: &str) -> bool {
    let hex_str = digest.strip_prefix("0x").unwrap_or(digest);
    hex::decode(hex_str).map(|b| b.len() == 32).unwrap_or(false)
}

/// Validate Sui address format.
///
/// Checks that a string is a valid hex-encoded 32-byte address.
///
/// # Arguments
///
/// * `address` - Address string to validate
///
/// # Returns
///
/// `true` if valid, `false` otherwise.
pub fn is_valid_address(address: &str) -> bool {
    let hex_str = address.strip_prefix("0x").unwrap_or(address);
    hex::decode(hex_str).map(|b| b.len() == 32).unwrap_or(false)
}

// ============================================================================
// Error Conversions
// ============================================================================

/// Convert Sui RPC error to SuiError.
///
/// # Arguments
///
/// * `_error` - Sui RPC error (placeholder)
///
/// # Returns
///
/// Mapped error.
pub fn sui_rpc_error_to_sui_error(_error: &()) -> SuiError {
    // TODO: Implement with actual Sui error types
    SuiError::Network("Unknown Sui RPC error".to_string())
}
