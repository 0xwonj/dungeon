//! Oracle access errors.
//!
//! Errors related to oracle availability and data access.

use crate::error::{ErrorSeverity, GameError};
use crate::state::{ItemHandle, Position};

/// Errors that occur when accessing Oracle data.
///
/// Oracle errors indicate that required game data is unavailable or invalid.
/// These are typically fatal errors since the game engine cannot proceed without
/// access to maps, items, or balance tables.
#[derive(Clone, Debug, PartialEq, Eq, thiserror::Error)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum OracleError {
    /// MapOracle is not available in the environment.
    #[error("MapOracle not available")]
    MapNotAvailable,

    /// ItemOracle is not available in the environment.
    #[error("ItemOracle not available")]
    ItemsNotAvailable,

    /// ActorOracle is not available in the environment.
    #[error("ActorOracle not available")]
    ActorsNotAvailable,

    /// TablesOracle is not available in the environment.
    #[error("TablesOracle not available")]
    TablesNotAvailable,

    /// ConfigOracle is not available in the environment.
    #[error("ConfigOracle not available")]
    ConfigNotAvailable,

    /// RngOracle is not available in the environment.
    #[error("RngOracle not available")]
    RngNotAvailable,

    /// Position is outside the map bounds.
    #[error("position {0:?} is out of map bounds")]
    PositionOutOfBounds(Position),

    /// Tile at the given position was not found.
    #[error("tile at position {0:?} not found")]
    TileNotFound(Position),

    /// Item definition was not found by handle.
    #[error("item definition {0:?} not found")]
    ItemNotFound(ItemHandle),

    /// Actor template was not found by ID.
    #[error("actor template '{0}' not found")]
    ActorTemplateNotFound(&'static str),
}

impl GameError for OracleError {
    fn severity(&self) -> ErrorSeverity {
        use OracleError::*;
        match self {
            // Missing oracles are fatal - engine cannot proceed
            MapNotAvailable | ItemsNotAvailable | ActorsNotAvailable | TablesNotAvailable
            | ConfigNotAvailable | RngNotAvailable => ErrorSeverity::Fatal,

            // Not found errors are validation errors - invalid references
            PositionOutOfBounds(_)
            | TileNotFound(_)
            | ItemNotFound(_)
            | ActorTemplateNotFound(_) => ErrorSeverity::Validation,
        }
    }

    fn error_code(&self) -> &'static str {
        use OracleError::*;
        match self {
            MapNotAvailable => "ORACLE_MAP_NOT_AVAILABLE",
            ItemsNotAvailable => "ORACLE_ITEMS_NOT_AVAILABLE",
            ActorsNotAvailable => "ORACLE_ACTORS_NOT_AVAILABLE",
            TablesNotAvailable => "ORACLE_TABLES_NOT_AVAILABLE",
            ConfigNotAvailable => "ORACLE_CONFIG_NOT_AVAILABLE",
            RngNotAvailable => "ORACLE_RNG_NOT_AVAILABLE",
            PositionOutOfBounds(_) => "ORACLE_POSITION_OUT_OF_BOUNDS",
            TileNotFound(_) => "ORACLE_TILE_NOT_FOUND",
            ItemNotFound(_) => "ORACLE_ITEM_NOT_FOUND",
            ActorTemplateNotFound(_) => "ORACLE_ACTOR_TEMPLATE_NOT_FOUND",
        }
    }
}
