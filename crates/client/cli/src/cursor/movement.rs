//! Cursor movement utilities.

use game_core::CardinalDirection;

/// Helper trait for converting direction inputs to cursor deltas.
pub trait CursorMovement {
    fn to_delta(&self) -> (i32, i32);
}

impl CursorMovement for CardinalDirection {
    fn to_delta(&self) -> (i32, i32) {
        // Use the same coordinate system as game-core
        // Y-axis: positive = north (up), negative = south (down)
        match self {
            CardinalDirection::North => (0, 1),
            CardinalDirection::South => (0, -1),
            CardinalDirection::East => (1, 0),
            CardinalDirection::West => (-1, 0),
            CardinalDirection::NorthEast => (1, 1),
            CardinalDirection::NorthWest => (-1, 1),
            CardinalDirection::SouthEast => (1, -1),
            CardinalDirection::SouthWest => (-1, -1),
        }
    }
}
