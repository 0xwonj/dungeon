//! State management errors.
//!
//! Errors related to game state operations, entity management, and capacity limits.

use crate::error::{ErrorSeverity, GameError};
use crate::state::{EntityId, Position};

/// Errors that occur during game state operations.
///
/// These errors indicate capacity limits, allocation failures, or invalid state transitions.
#[derive(Clone, Debug, PartialEq, Eq, thiserror::Error)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum StateError {
    /// Actor list is full (max capacity reached).
    #[error("Actor list is full (max: {max}, current: {current})")]
    ActorListFull {
        /// Maximum capacity.
        max: usize,
        /// Current count.
        current: usize,
    },

    /// Prop list is full (max capacity reached).
    #[error("Prop list is full (max: {max}, current: {current})")]
    PropListFull {
        /// Maximum capacity.
        max: usize,
        /// Current count.
        current: usize,
    },

    /// Item list is full (max capacity reached).
    #[error("Item list is full (max: {max}, current: {current})")]
    ItemListFull {
        /// Maximum capacity.
        max: usize,
        /// Current count.
        current: usize,
    },

    /// Entity ID allocation overflow (all IDs exhausted).
    #[error("Entity ID overflow (current: {current})")]
    EntityIdOverflow {
        /// Current ID value when overflow occurred.
        current: u32,
    },

    /// Position is already occupied by another entity.
    #[error("Position {position:?} is already occupied by entity {occupant:?}")]
    PositionOccupied {
        /// The position that is occupied.
        position: Position,
        /// The entity currently occupying the position.
        occupant: EntityId,
    },

    /// Position is outside the map bounds.
    #[error("Position {position:?} is out of bounds (map size: {map_width}x{map_height})")]
    PositionOutOfBounds {
        /// The invalid position.
        position: Position,
        /// Map width.
        map_width: u32,
        /// Map height.
        map_height: u32,
    },
}

impl GameError for StateError {
    fn severity(&self) -> ErrorSeverity {
        use StateError::*;
        match self {
            // Capacity errors are validation errors - invalid to add more entities
            ActorListFull { .. } | PropListFull { .. } | ItemListFull { .. } => {
                ErrorSeverity::Validation
            }

            // ID overflow is a fatal error - cannot continue
            EntityIdOverflow { .. } => ErrorSeverity::Fatal,

            // Position errors are validation errors
            PositionOccupied { .. } | PositionOutOfBounds { .. } => ErrorSeverity::Validation,
        }
    }

    fn error_code(&self) -> &'static str {
        use StateError::*;
        match self {
            ActorListFull { .. } => "STATE_ACTOR_LIST_FULL",
            PropListFull { .. } => "STATE_PROP_LIST_FULL",
            ItemListFull { .. } => "STATE_ITEM_LIST_FULL",
            EntityIdOverflow { .. } => "STATE_ENTITY_ID_OVERFLOW",
            PositionOccupied { .. } => "STATE_POSITION_OCCUPIED",
            PositionOutOfBounds { .. } => "STATE_POSITION_OUT_OF_BOUNDS",
        }
    }
}
