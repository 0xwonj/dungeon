pub mod command;
pub mod kinds;
pub mod transition;

use crate::state::EntityId;

pub use command::{ActionCommand, CommandContext};
pub use kinds::{
    AttackAction, AttackStyle, CardinalDirection, InteractAction, InventorySlot, ItemTarget,
    MoveAction, UseItemAction,
};
pub use transition::ActionTransition;

/// Describes a single intent issued by an entity for the current turn.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Action {
    pub actor: EntityId,
    pub kind: ActionKind,
}

impl Action {
    pub fn new(actor: EntityId, kind: ActionKind) -> Self {
        Self { actor, kind }
    }

    pub fn from_command<Env, C>(
        actor: EntityId,
        command: C,
        ctx: CommandContext<'_, Env>,
    ) -> Result<Self, C::Error>
    where
        C: ActionCommand<Env>,
    {
        command.into_action(actor, ctx)
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ActionKind {
    Move(MoveAction),
    Attack(AttackAction),
    UseItem(UseItemAction),
    Interact(InteractAction),
    Wait,
}

impl From<MoveAction> for ActionKind {
    fn from(action: MoveAction) -> Self {
        Self::Move(action)
    }
}

impl From<AttackAction> for ActionKind {
    fn from(action: AttackAction) -> Self {
        Self::Attack(action)
    }
}

impl From<UseItemAction> for ActionKind {
    fn from(action: UseItemAction) -> Self {
        Self::UseItem(action)
    }
}

impl From<InteractAction> for ActionKind {
    fn from(action: InteractAction) -> Self {
        Self::Interact(action)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::state::GameState;

    #[test]
    fn move_action_materializes_via_command() {
        let actor = EntityId(7);
        let state = GameState::default();
        let ctx = CommandContext::stateless(&state);
        let command = MoveAction::new(CardinalDirection::North);

        let action = Action::from_command(actor, command, ctx).expect("MoveAction is infallible");

        assert_eq!(action.actor, actor);
        match action.kind {
            ActionKind::Move(move_action) => {
                assert_eq!(move_action.direction, CardinalDirection::North)
            }
            other => panic!("expected move action, got {other:?}"),
        }
    }

    #[test]
    fn custom_command_uses_env_before_emitting_action() {
        struct TestEnv {
            allow_wait: bool,
        }

        struct WaitIfAllowed;

        impl ActionCommand<TestEnv> for WaitIfAllowed {
            type Error = &'static str;

            fn into_action(
                self,
                actor: EntityId,
                ctx: CommandContext<'_, TestEnv>,
            ) -> Result<Action, Self::Error> {
                if ctx.env().allow_wait {
                    Ok(Action::new(actor, ActionKind::Wait))
                } else {
                    Err("wait not permitted")
                }
            }
        }

        let actor = EntityId(3);
        let state = GameState::default();
        let env = TestEnv { allow_wait: true };
        let ctx = CommandContext::new(&state, &env);

        let action = Action::from_command(actor, WaitIfAllowed, ctx)
            .expect("env allows waiting, so command should succeed");
        assert!(matches!(action.kind, ActionKind::Wait));

        let env = TestEnv { allow_wait: false };
        let ctx = CommandContext::new(&state, &env);
        let result = Action::from_command(actor, WaitIfAllowed, ctx);
        assert!(result.is_err());
    }
}
