//! Placeholder tables oracle implementation.
//!
//! This oracle is currently a no-op placeholder.
//! See [`game_core::TablesOracle`] documentation for future use cases.

use game_core::TablesOracle;

/// Placeholder TablesOracle implementation.
///
/// This struct exists to satisfy the oracle manager requirements,
/// but contains no data or logic. When TablesOracle gains methods,
/// this implementation will be expanded.
#[derive(Debug, Clone, Default)]
pub struct TablesOracleImpl;

impl TablesOracleImpl {
    /// Create a new placeholder tables oracle.
    pub fn new() -> Self {
        Self
    }

    /// Create with default (empty) rules.
    pub fn test_tables() -> Self {
        Self::new()
    }
}

impl TablesOracle for TablesOracleImpl {
    // No methods to implement yet - trait is empty
}
