use core::convert::Infallible;

use crate::state::{EntityId, GameState};

use super::{Action, ActionKind, AttackAction, InteractAction, MoveAction, UseItemAction};

/// Shared context available when materializing high-level commands into canonical actions.
pub struct CommandContext<'a, Env> {
    state: &'a GameState,
    env: &'a Env,
}

impl<'a, Env> CommandContext<'a, Env> {
    pub fn new(state: &'a GameState, env: &'a Env) -> Self {
        Self { state, env }
    }

    pub fn state(&self) -> &'a GameState {
        self.state
    }

    pub fn env(&self) -> &'a Env {
        self.env
    }
}

impl<'a> CommandContext<'a, ()> {
    pub fn stateless(state: &'a GameState) -> Self {
        Self { state, env: &() }
    }
}

/// Trait for higher-level commands that want to emit canonical `Action`s.
pub trait ActionCommand<Env> {
    type Error;

    fn into_action(
        self,
        actor: EntityId,
        ctx: CommandContext<'_, Env>,
    ) -> Result<Action, Self::Error>;
}

impl<Env> ActionCommand<Env> for ActionKind {
    type Error = Infallible;

    fn into_action(
        self,
        actor: EntityId,
        _ctx: CommandContext<'_, Env>,
    ) -> Result<Action, Self::Error> {
        Ok(Action::new(actor, self))
    }
}

impl<Env> ActionCommand<Env> for MoveAction {
    type Error = Infallible;

    fn into_action(
        self,
        actor: EntityId,
        _ctx: CommandContext<'_, Env>,
    ) -> Result<Action, Self::Error> {
        Ok(Action::new(actor, self.into()))
    }
}

impl<Env> ActionCommand<Env> for AttackAction {
    type Error = Infallible;

    fn into_action(
        self,
        actor: EntityId,
        _ctx: CommandContext<'_, Env>,
    ) -> Result<Action, Self::Error> {
        Ok(Action::new(actor, self.into()))
    }
}

impl<Env> ActionCommand<Env> for UseItemAction {
    type Error = Infallible;

    fn into_action(
        self,
        actor: EntityId,
        _ctx: CommandContext<'_, Env>,
    ) -> Result<Action, Self::Error> {
        Ok(Action::new(actor, self.into()))
    }
}

impl<Env> ActionCommand<Env> for InteractAction {
    type Error = Infallible;

    fn into_action(
        self,
        actor: EntityId,
        _ctx: CommandContext<'_, Env>,
    ) -> Result<Action, Self::Error> {
        Ok(Action::new(actor, self.into()))
    }
}
