use core::convert::Infallible;

use crate::action::ActionTransition;
use crate::engine::StateReducer;
use crate::env::GameEnv;
use crate::state::EntityId;

/// Offensive action against a target entity.
#[derive(Clone, Debug, PartialEq, Eq)]
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

/// High-level command describing an attack intent.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AttackCommand {
    pub target: EntityId,
    pub style: AttackStyle,
}

impl AttackCommand {
    pub fn new(target: EntityId, style: AttackStyle) -> Self {
        Self { target, style }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum AttackStyle {
    Melee,
}

impl ActionTransition for AttackAction {
    type Error = Infallible;

    fn cost(&self) -> crate::state::Tick {
        crate::state::Tick(15)
    }

    fn apply(
        &self,
        _reducer: &mut StateReducer<'_>,
        _env: &GameEnv<'_>,
    ) -> Result<(), Self::Error> {
        Ok(())
    }
}
