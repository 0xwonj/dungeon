use core::convert::Infallible;

use crate::env::GameEnv;
use crate::state::{EntityId, GameState};

use super::{
    Action, ActionKind, AttackAction, AttackCommand, InteractAction, InteractCommand, MoveAction,
    MoveCommand, UseItemAction, UseItemCommand,
};

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

    fn into_action(self, actor: EntityId, ctx: CommandContext<'_>) -> Result<Action, Self::Error>;
}

impl ActionCommand for ActionKind {
    type Error = Infallible;

    fn into_action(self, actor: EntityId, _ctx: CommandContext<'_>) -> Result<Action, Self::Error> {
        Ok(Action::new(actor, self))
    }
}

impl ActionCommand for MoveCommand {
    type Error = Infallible;

    fn into_action(self, actor: EntityId, _ctx: CommandContext<'_>) -> Result<Action, Self::Error> {
        debug_assert!(self.distance > 0, "distance must be positive");
        let action = MoveAction::new(actor, self.direction, self.distance);
        Ok(Action::new(actor, action.into()))
    }
}

impl ActionCommand for AttackCommand {
    type Error = Infallible;

    fn into_action(self, actor: EntityId, _ctx: CommandContext<'_>) -> Result<Action, Self::Error> {
        let action = AttackAction::new(actor, self.target, self.style);
        Ok(Action::new(actor, action.into()))
    }
}

impl ActionCommand for UseItemCommand {
    type Error = Infallible;

    fn into_action(self, actor: EntityId, _ctx: CommandContext<'_>) -> Result<Action, Self::Error> {
        let action = UseItemAction::new(actor, self.slot, self.target);
        Ok(Action::new(actor, action.into()))
    }
}

impl ActionCommand for InteractCommand {
    type Error = Infallible;

    fn into_action(self, actor: EntityId, _ctx: CommandContext<'_>) -> Result<Action, Self::Error> {
        let action = InteractAction::new(actor, self.target);
        Ok(Action::new(actor, action.into()))
    }
}
