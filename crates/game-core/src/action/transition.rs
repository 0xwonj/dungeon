use crate::env::GameEnv;
use crate::state::GameState;

/// Defines how a concrete action variant mutates game state while mirroring
/// the constraint checks enforced inside zk circuits.
///
/// Implementors can override the validation hooks to surface pre- and
/// post-conditions that must hold around the state mutation. All hooks receive
/// read-only access to deterministic environment facts via `Env` and must stay
/// side-effect free.
pub trait ActionTransition {
    type Error;

    /// Validates pre-conditions using the state **before** mutation.
    fn pre_validate(&self, _state: &GameState, _env: &GameEnv<'_>) -> Result<(), Self::Error> {
        Ok(())
    }

    /// Applies the action to the state. Implementations should assume that
    /// `pre_validate` has already run successfully.
    fn apply(&self, state: &mut GameState, env: &GameEnv<'_>) -> Result<(), Self::Error>;

    /// Validates post-conditions using the state **after** mutation.
    fn post_validate(&self, _state: &GameState, _env: &GameEnv<'_>) -> Result<(), Self::Error> {
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use core::cell::Cell;

    use super::ActionTransition;
    use crate::env::{AttackProfile, Env, GameEnv, ItemCategory, ItemDefinition, ItemOracle,
        MapDimensions, MapOracle, MovementRules, StaticTile, TablesOracle, TerrainKind};
    use crate::state::GameState;

    struct NoopAction;

    impl ActionTransition for NoopAction {
        type Error = core::convert::Infallible;

        fn apply(&self, _state: &mut GameState, _env: &GameEnv<'_>) -> Result<(), Self::Error> {
            Ok(())
        }
    }

    #[test]
    fn default_hooks_allow_apply_to_run() {
        let mut state = GameState::default();
        let env = test_env();
        let action = NoopAction;

        action.pre_validate(&state, &env).unwrap();
        action.apply(&mut state, &env).unwrap();
        action.post_validate(&state, &env).unwrap();
    }

    struct CountingAction<'a> {
        pre_count: &'a Cell<u32>,
        post_count: &'a Cell<u32>,
    }

    impl<'a> ActionTransition for CountingAction<'a> {
        type Error = core::convert::Infallible;

        fn pre_validate(&self, _state: &GameState, _env: &GameEnv<'_>) -> Result<(), Self::Error> {
            self.pre_count.set(self.pre_count.get() + 1);
            Ok(())
        }

        fn apply(&self, _state: &mut GameState, _env: &GameEnv<'_>) -> Result<(), Self::Error> {
            Ok(())
        }

        fn post_validate(&self, _state: &GameState, _env: &GameEnv<'_>) -> Result<(), Self::Error> {
            self.post_count.set(self.post_count.get() + 1);
            Ok(())
        }
    }

    #[test]
    fn custom_hooks_run_in_expected_order() {
        let mut state = GameState::default();
        let env = test_env();
        let pre = Cell::new(0);
        let post = Cell::new(0);
        let action = CountingAction {
            pre_count: &pre,
            post_count: &post,
        };

        action.pre_validate(&state, &env).unwrap();
        assert_eq!(pre.get(), 1);
        assert_eq!(post.get(), 0);

        action.apply(&mut state, &env).unwrap();
        assert_eq!(pre.get(), 1);
        assert_eq!(post.get(), 0);

        action.post_validate(&state, &env).unwrap();
        assert_eq!(pre.get(), 1);
        assert_eq!(post.get(), 1);
    }

    fn test_env() -> GameEnv<'static> {
        static MAP: StubMap = StubMap;
        static ITEMS: StubItems = StubItems;
        static TABLES: StubTables = StubTables;
        Env::with_all(&MAP, &ITEMS, &TABLES).into_game_env()
    }

    #[derive(Debug)]
    struct StubMap;

    impl MapOracle for StubMap {
        fn dimensions(&self) -> MapDimensions {
            MapDimensions::new(1, 1)
        }

        fn tile(&self, _position: crate::state::Position) -> Option<StaticTile> {
            Some(StaticTile::new(TerrainKind::Floor))
        }
    }

    #[derive(Debug)]
    struct StubItems;

    impl ItemOracle for StubItems {
        fn definition(&self, _handle: crate::state::ItemHandle) -> Option<ItemDefinition> {
            Some(ItemDefinition::new(
                crate::state::ItemHandle(0),
                ItemCategory::Utility,
                None,
                None,
            ))
        }
    }

    #[derive(Debug)]
    struct StubTables;

    impl TablesOracle for StubTables {
        fn movement_rules(&self) -> MovementRules {
            MovementRules::new(1, 1)
        }

        fn attack_profile(
            &self,
            _style: crate::action::AttackStyle,
        ) -> Option<AttackProfile> {
            Some(AttackProfile::new(1, 0))
        }
    }
}
