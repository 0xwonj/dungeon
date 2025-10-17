//! Common type aliases used throughout the runtime.
//!
//! These type aliases provide semantic clarity for primitive types
//! that are used with specific meanings across the runtime.

/// Action sequence number (monotonically increasing)
pub type Nonce = u64;

/// Session identifier for game runs
pub type SessionId = String;

/// Hash of game state (for verification)
pub type StateHash = String;

/// Unix timestamp in seconds
pub type Timestamp = u64;

/// Duration in milliseconds
pub type DurationMs = u64;

/// Byte offset in a file
pub type ByteOffset = u64;

/// Proof file size in bytes
pub type ProofSize = u64;
