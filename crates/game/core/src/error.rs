//! Common error infrastructure for game-core.
//!
//! This module provides shared types and traits used across all error types in game-core.
//! Domain-specific errors (e.g., `MoveError`, `AttackError`) are defined in their
//! respective modules alongside the actions they validate.
//!
//! # Design Principles
//!
//! - **Type Safety**: Each action has its own error type with specific variants
//! - **Rich Context**: Errors include actor, position, and nonce for debugging
//! - **Severity Classification**: Errors are categorized for recovery strategies
//! - **ZK-Friendly**: All types are `no_std` compatible and deterministic

use crate::state::{EntityId, Position};

/// Severity level of an error, used for categorization and recovery strategies.
///
/// Errors are classified by their recoverability and expected handling:
/// - **Recoverable**: Temporary conditions that may succeed on retry or with alternative actions
/// - **Validation**: Invalid input that should be rejected without retry
/// - **Internal**: Unexpected state inconsistencies that require investigation
/// - **Fatal**: Unrecoverable errors indicating corrupted game state
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum ErrorSeverity {
    /// Recoverable error - can retry with same or alternative action.
    ///
    /// Examples: destination blocked, target out of range
    Recoverable,

    /// Validation error - invalid input, should not retry without changes.
    ///
    /// Examples: actor not found, invalid target
    Validation,

    /// Internal error - unexpected state inconsistency.
    ///
    /// Examples: occupancy map desync, missing expected entity
    /// These indicate bugs and should be investigated.
    Internal,

    /// Fatal error - game state corrupted, cannot continue.
    ///
    /// Examples: missing required oracle, state invariant violated
    Fatal,
}

impl ErrorSeverity {
    /// Returns a human-readable description of this severity level.
    pub const fn as_str(&self) -> &'static str {
        match self {
            Self::Recoverable => "recoverable",
            Self::Validation => "validation",
            Self::Internal => "internal",
            Self::Fatal => "fatal",
        }
    }

    /// Returns true if this error is potentially recoverable.
    pub const fn is_recoverable(&self) -> bool {
        matches!(self, Self::Recoverable)
    }

    /// Returns true if this error indicates an internal bug.
    pub const fn is_internal(&self) -> bool {
        matches!(self, Self::Internal | Self::Fatal)
    }
}

/// Contextual information attached to errors for debugging and diagnostics.
///
/// Context is captured at the point of error creation and includes relevant
/// game state information that helps diagnose the failure.
#[derive(Clone, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct ErrorContext {
    /// Entity that triggered the error (if applicable).
    pub actor: Option<EntityId>,

    /// Position where the error occurred (if applicable).
    pub position: Option<Position>,

    /// Game nonce at the time of error.
    ///
    /// The nonce uniquely identifies the action sequence and is useful
    /// for correlating errors with specific game states in logs or proofs.
    pub nonce: u64,

    /// Optional static message providing additional context.
    pub message: Option<&'static str>,
}

impl ErrorContext {
    /// Creates a new error context with the given nonce.
    #[must_use]
    pub const fn new(nonce: u64) -> Self {
        Self {
            actor: None,
            position: None,
            nonce,
            message: None,
        }
    }

    /// Attaches an actor to this context (builder pattern).
    #[must_use]
    pub const fn with_actor(mut self, actor: EntityId) -> Self {
        self.actor = Some(actor);
        self
    }

    /// Attaches a position to this context (builder pattern).
    #[must_use]
    pub const fn with_position(mut self, position: Position) -> Self {
        self.position = Some(position);
        self
    }

    /// Attaches a static message to this context (builder pattern).
    #[must_use]
    pub const fn with_message(mut self, message: &'static str) -> Self {
        self.message = Some(message);
        self
    }
}

impl Default for ErrorContext {
    fn default() -> Self {
        Self::new(0)
    }
}

/// Common trait for all game-core errors.
///
/// This trait provides a uniform interface for error classification and context
/// retrieval across all error types in the crate.
///
/// # Implementation Guidelines
///
/// - All error enums should implement this trait
/// - Use `#[derive(thiserror::Error)]` for Display/Error impl
/// - Include `ErrorContext` in variants that need debugging info
/// - Classify severity based on recoverability, not impact
pub trait GameError: core::fmt::Display + core::fmt::Debug {
    /// Returns the severity level of this error.
    ///
    /// This is used for error handling strategies and logging priorities.
    fn severity(&self) -> ErrorSeverity;

    /// Returns the context information for this error, if available.
    ///
    /// Not all errors have context (e.g., errors delegated from other crates).
    fn context(&self) -> Option<&ErrorContext> {
        None
    }

    /// Returns a static string identifier for this error variant.
    ///
    /// This is useful for error categorization, metrics, and testing.
    /// Default implementation uses the error type name.
    fn error_code(&self) -> &'static str {
        // Default: use the type name as error code
        core::any::type_name::<Self>()
    }
}

/// Error type for actions that never fail.
#[derive(Clone, Debug, PartialEq, Eq, thiserror::Error)]
#[error("this error should never be constructed")]
pub enum NeverError {}

impl GameError for NeverError {
    fn severity(&self) -> ErrorSeverity {
        match *self {} // Empty match - this is never constructed
    }

    fn context(&self) -> Option<&ErrorContext> {
        match *self {}
    }

    fn error_code(&self) -> &'static str {
        match *self {}
    }
}

// Custom serde implementation to prevent accidental serialization
#[cfg(feature = "serde")]
impl serde::Serialize for NeverError {
    fn serialize<S>(&self, _serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        match *self {} // Cannot serialize something that doesn't exist
    }
}

#[cfg(feature = "serde")]
impl<'de> serde::Deserialize<'de> for NeverError {
    fn deserialize<D>(_deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        Err(serde::de::Error::custom(
            "NeverError cannot be deserialized as it represents an impossible error",
        ))
    }
}
