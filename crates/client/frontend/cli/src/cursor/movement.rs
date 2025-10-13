//! Cursor movement utilities.

use game_core::{CardinalDirection, Position};

/// Helper trait for converting direction inputs to cursor deltas.
pub trait CursorMovement {
    fn to_delta(&self) -> (i32, i32);
}

impl CursorMovement for CardinalDirection {
    fn to_delta(&self) -> (i32, i32) {
        match self {
            CardinalDirection::North => (0, 1),
            CardinalDirection::South => (0, -1),
            CardinalDirection::East => (1, 0),
            CardinalDirection::West => (-1, 0),
        }
    }
}

/// Computes Manhattan distance between two positions.
pub fn manhattan_distance(a: Position, b: Position) -> u32 {
    ((a.x - b.x).abs() + (a.y - b.y).abs()) as u32
}
