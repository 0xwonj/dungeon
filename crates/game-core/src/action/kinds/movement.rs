use core::convert::Infallible;

use crate::action::ActionTransition;
use crate::env::GameEnv;
use crate::state::GameState;

/// Cardinal grid movement action.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct MoveAction {
    pub direction: CardinalDirection,
}

impl MoveAction {
    pub fn new(direction: CardinalDirection) -> Self {
        Self { direction }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CardinalDirection {
    North,
    South,
    East,
    West,
}

impl CardinalDirection {
    pub fn delta(self) -> (i32, i32) {
        match self {
            CardinalDirection::North => (0, 1),
            CardinalDirection::South => (0, -1),
            CardinalDirection::East => (1, 0),
            CardinalDirection::West => (-1, 0),
        }
    }
}

impl ActionTransition for MoveAction {
    type Error = Infallible;

    fn apply(&self, _state: &mut GameState, _env: &GameEnv<'_>) -> Result<(), Self::Error> {
        Ok(())
    }
}
