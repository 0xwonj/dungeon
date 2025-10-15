use core::convert::Infallible;

use crate::action::ActionTransition;
use crate::env::GameEnv;
use crate::state::{EntityId, GameState, Tick};

/// Offensive action against a target entity.
#[derive(Clone, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct AttackAction {
    pub actor: EntityId,
    pub target: EntityId,
    pub style: AttackStyle,
}

impl AttackAction {
    pub fn new(actor: EntityId, target: EntityId, style: AttackStyle) -> Self {
        Self {
            actor,
            target,
            style,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum AttackStyle {
    Melee,
}

impl ActionTransition for AttackAction {
    type Error = Infallible;

    fn actor(&self) -> EntityId {
        self.actor
    }

    fn cost(&self) -> Tick {
        0
    }

    fn apply(&self, _state: &mut GameState, _env: &GameEnv<'_>) -> Result<(), Self::Error> {
        Ok(())
    }
}
