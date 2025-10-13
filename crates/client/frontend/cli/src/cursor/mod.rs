//! Cursor system for interactive tile and entity selection.
//!
//! This module provides a reusable cursor abstraction that can be used for
//! both Examine mode (free exploration) and Targeting mode (constrained selection).

mod movement;
mod target_selector;
mod targeting;

pub use movement::CursorMovement;
pub use target_selector::{ChainSelector, select_target};

use game_core::Position;

/// Cursor state for manual position selection.
///
/// Used in ExamineManual mode for free tile exploration.
/// Future: Can be extended for Targeting mode with constraint validation.
#[derive(Clone, Debug)]
pub struct CursorState {
    /// Current cursor position in world coordinates.
    pub position: Position,
}

impl CursorState {
    /// Creates a new cursor at the given position.
    pub fn new(position: Position) -> Self {
        Self { position }
    }

    /// Moves the cursor by the given delta, clamped to map bounds.
    pub fn move_by(&mut self, dx: i32, dy: i32, width: u32, height: u32) {
        let new_x = (self.position.x + dx).clamp(0, width as i32 - 1);
        let new_y = (self.position.y + dy).clamp(0, height as i32 - 1);
        self.position = Position::new(new_x, new_y);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cursor_movement_clamps_to_bounds() {
        let mut cursor = CursorState::new(Position::new(5, 5));
        cursor.move_by(-10, -10, 20, 20);
        assert_eq!(cursor.position, Position::new(0, 0));

        cursor.move_by(100, 100, 20, 20);
        assert_eq!(cursor.position, Position::new(19, 19));
    }
}
