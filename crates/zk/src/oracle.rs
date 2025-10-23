//! Host-side utilities for creating oracle snapshots from runtime oracles.
//!
//! This module provides conversion functions to create snapshots from
//! runtime oracle implementations. The snapshot structures themselves
//! are defined in `game-core` and are shared between host and guest.
//!
//! # Design
//!
//! - **Snapshots**: Defined in `game-core` (shared with guest)
//! - **Conversion**: `from_oracle()` functions in `game-core` (std-only)
//! - **Guest adapters**: Defined in `game-core` (no_std compatible)

// Re-export snapshots from game-core for convenience
pub use game_core::{
    ActorsSnapshot, ConfigSnapshot, ItemsSnapshot, MapSnapshot, OracleSnapshot,
    SnapshotActorOracle, SnapshotConfigOracle, SnapshotItemOracle, SnapshotMapOracle,
    SnapshotOracleBundle, SnapshotTablesOracle, TablesSnapshot,
};
