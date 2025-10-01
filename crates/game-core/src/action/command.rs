use core::convert::Infallible;

use crate::env::GameEnv;
use crate::state::{EntityId, GameState};

use super::{Action, ActionKind, AttackAction, InteractAction, MoveAction, UseItemAction};

/// Shared context available when materializing high-level commands into canonical actions.
pub struct CommandContext<'a> {
    state: &'a GameState,
    env: GameEnv<'a>,
}

impl<'a> CommandContext<'a> {
    pub fn new(state: &'a GameState, env: GameEnv<'a>) -> Self {
        Self { state, env }
    }

    pub fn state(&self) -> &'a GameState {
        self.state
    }

    pub fn env(&self) -> &GameEnv<'a> {
        &self.env
    }
}

/// Trait for higher-level commands that want to emit canonical `Action`s.
pub trait ActionCommand {
    type Error;

    fn into_action(
        self,
        actor: EntityId,
        ctx: CommandContext<'_>,
    ) -> Result<Action, Self::Error>;
}

impl ActionCommand for ActionKind {
    type Error = Infallible;

    fn into_action(
        self,
        actor: EntityId,
        _ctx: CommandContext<'_>,
    ) -> Result<Action, Self::Error> {
        Ok(Action::new(actor, self))
    }
}

impl ActionCommand for MoveAction {
    type Error = Infallible;

    fn into_action(
        self,
        actor: EntityId,
        _ctx: CommandContext<'_>,
    ) -> Result<Action, Self::Error> {
        Ok(Action::new(actor, self.into()))
    }
}

impl ActionCommand for AttackAction {
    type Error = Infallible;

    fn into_action(
        self,
        actor: EntityId,
        _ctx: CommandContext<'_>,
    ) -> Result<Action, Self::Error> {
        Ok(Action::new(actor, self.into()))
    }
}

impl ActionCommand for UseItemAction {
    type Error = Infallible;

    fn into_action(
        self,
        actor: EntityId,
        _ctx: CommandContext<'_>,
    ) -> Result<Action, Self::Error> {
        Ok(Action::new(actor, self.into()))
    }
}

impl ActionCommand for InteractAction {
    type Error = Infallible;

    fn into_action(
        self,
        actor: EntityId,
        _ctx: CommandContext<'_>,
    ) -> Result<Action, Self::Error> {
        Ok(Action::new(actor, self.into()))
    }
}
