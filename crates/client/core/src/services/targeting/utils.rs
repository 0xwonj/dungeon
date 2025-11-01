//! Common utilities for targeting strategies.

use game_core::Position;

/// Calculate Manhattan distance between two positions.
///
/// Manhattan distance is the sum of absolute differences of coordinates,
/// representing grid-based movement distance in a 2D game.
pub fn manhattan_distance(a: Position, b: Position) -> u32 {
    ((a.x - b.x).abs() + (a.y - b.y).abs()) as u32
}

/// Calculate health percentage for an actor.
///
/// Returns a value in range [0, 100] representing current HP as percentage of max HP.
pub fn health_percentage(current: u32, maximum: u32) -> u32 {
    if maximum > 0 {
        (current * 100) / maximum
    } else {
        100 // No max HP means full health (edge case)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_manhattan_distance() {
        let a = Position { x: 0, y: 0 };
        let b = Position { x: 3, y: 4 };
        assert_eq!(manhattan_distance(a, b), 7);

        let c = Position { x: -2, y: -2 };
        let d = Position { x: 2, y: 2 };
        assert_eq!(manhattan_distance(c, d), 8);
    }

    #[test]
    fn test_health_percentage() {
        assert_eq!(health_percentage(50, 100), 50);
        assert_eq!(health_percentage(0, 100), 0);
        assert_eq!(health_percentage(100, 100), 100);
        assert_eq!(health_percentage(75, 100), 75);
        assert_eq!(health_percentage(0, 0), 100); // Edge case
    }
}
