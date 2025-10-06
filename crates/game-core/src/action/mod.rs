pub mod command;
pub mod kinds;
pub mod transition;

use crate::state::{EntityId, Tick};

pub use command::{ActionCommand, CommandContext};
pub use kinds::{
    AttackAction, AttackCommand, AttackStyle, CardinalDirection, InteractAction, InteractCommand,
    InventorySlot, ItemTarget, MoveAction, MoveCommand, MoveError, UseItemAction, UseItemCommand,
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
        debug_assert!(match &kind {
            ActionKind::Move(move_action) => move_action.actor == actor,
            ActionKind::Attack(attack_action) => attack_action.actor == actor,
            ActionKind::UseItem(use_item_action) => use_item_action.actor == actor,
            ActionKind::Interact(interact_action) => interact_action.actor == actor,
            _ => true,
        });
        Self { actor, kind }
    }

    pub fn from_command<C>(
        actor: EntityId,
        command: C,
        ctx: CommandContext<'_>,
    ) -> Result<Self, C::Error>
    where
        C: ActionCommand,
    {
        command.into_action(actor, ctx)
    }

    /// Returns the time cost (in ticks) for this action.
    /// This determines how much the entity's ready_at advances after execution.
    pub fn cost(&self) -> Tick {
        use crate::action::ActionTransition;
        match &self.kind {
            ActionKind::Move(action) => action.cost(),
            ActionKind::Attack(action) => action.cost(),
            ActionKind::UseItem(action) => action.cost(),
            ActionKind::Interact(action) => action.cost(),
            ActionKind::Wait => Tick(5),
        }
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
    use crate::env::{
        AttackProfile, Env, GameEnv, InitialEntityKind, InitialEntitySpec, ItemCategory,
        ItemDefinition, ItemOracle, MapDimensions, MapOracle, MovementRules, NpcOracle,
        NpcTemplate, StaticTile, TablesOracle, TerrainKind,
    };
    use crate::state::{EntityId, GameState, ItemHandle, Position};

    #[derive(Debug, Default)]
    struct StubMap;

    impl MapOracle for StubMap {
        fn dimensions(&self) -> MapDimensions {
            MapDimensions::new(4, 4)
        }

        fn tile(&self, position: Position) -> Option<StaticTile> {
            if self.dimensions().contains(position) {
                Some(StaticTile::new(TerrainKind::Floor))
            } else {
                None
            }
        }

        fn initial_entities(&self) -> Vec<InitialEntitySpec> {
            vec![InitialEntitySpec {
                id: EntityId::PLAYER,
                position: Position::new(0, 0),
                kind: InitialEntityKind::Player,
            }]
        }
    }

    #[derive(Debug, Default)]
    struct StubItems;

    impl ItemOracle for StubItems {
        fn definition(&self, handle: ItemHandle) -> Option<ItemDefinition> {
            Some(ItemDefinition::new(
                handle,
                ItemCategory::Utility,
                None,
                None,
            ))
        }
    }

    #[derive(Debug)]
    struct StubTables {
        max_step: u8,
    }

    impl StubTables {
        fn new(max_step: u8) -> Self {
            Self { max_step }
        }
    }

    impl Default for StubTables {
        fn default() -> Self {
            Self::new(1)
        }
    }

    impl TablesOracle for StubTables {
        fn movement_rules(&self) -> MovementRules {
            MovementRules::new(self.max_step, 1)
        }

        fn attack_profile(&self, _style: AttackStyle) -> Option<AttackProfile> {
            Some(AttackProfile::new(1, 0))
        }
    }

    #[derive(Debug, Default)]
    struct StubNpcs;

    impl NpcOracle for StubNpcs {
        fn template(&self, _template_id: u16) -> Option<NpcTemplate> {
            Some(NpcTemplate::simple(100, 50))
        }
    }

    fn build_env<'a>(
        map: &'a StubMap,
        items: &'a StubItems,
        tables: &'a StubTables,
        npcs: &'a StubNpcs,
    ) -> GameEnv<'a> {
        Env::with_all(map, items, tables, npcs).into_game_env()
    }

    #[test]
    fn move_action_materializes_via_command() {
        let actor = EntityId(7);
        let state = GameState::default();
        let map = StubMap::default();
        let items = StubItems::default();
        let tables = StubTables::default();
        let npcs = StubNpcs;
        let env = build_env(&map, &items, &tables, &npcs);
        let ctx = CommandContext::new(&state, env);
        let command = MoveCommand::new(CardinalDirection::North, 1);

        let action = Action::from_command(actor, command, ctx).expect("MoveCommand is infallible");

        assert_eq!(action.actor, actor);
        match action.kind {
            ActionKind::Move(move_action) => {
                assert_eq!(move_action.actor, actor);
                assert_eq!(move_action.direction, CardinalDirection::North);
                assert_eq!(move_action.distance, 1);
            }
            other => panic!("expected move action, got {other:?}"),
        }
    }

    #[test]
    fn custom_command_uses_env_before_emitting_action() {
        struct WaitIfAllowed;

        impl ActionCommand for WaitIfAllowed {
            type Error = &'static str;

            fn into_action(
                self,
                actor: EntityId,
                ctx: CommandContext<'_>,
            ) -> Result<Action, Self::Error> {
                let can_wait = ctx
                    .env()
                    .tables()
                    .expect("tables oracle should exist")
                    .movement_rules()
                    .max_step_distance
                    > 0;

                if can_wait {
                    Ok(Action::new(actor, ActionKind::Wait))
                } else {
                    Err("wait not permitted")
                }
            }
        }

        let actor = EntityId(3);
        let state = GameState::default();
        let map = StubMap::default();
        let items = StubItems::default();
        let tables = StubTables::new(1);
        let npcs = StubNpcs;
        let env = build_env(&map, &items, &tables, &npcs);
        let ctx = CommandContext::new(&state, env);

        let action = Action::from_command(actor, WaitIfAllowed, ctx)
            .expect("env allows waiting, so command should succeed");
        assert!(matches!(action.kind, ActionKind::Wait));

        let tables = StubTables::new(0);
        let npcs = StubNpcs;
        let env = build_env(&map, &items, &tables, &npcs);
        let ctx = CommandContext::new(&state, env);
        let result = Action::from_command(actor, WaitIfAllowed, ctx);
        assert!(result.is_err());
    }
}
