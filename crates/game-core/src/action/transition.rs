use crate::state::GameState;

/// Defines how a concrete action variant mutates game state while mirroring
/// the constraint checks enforced inside zk circuits.
///
/// Implementors can override the validation hooks to surface pre- and
/// post-conditions that must hold around the state mutation. All hooks receive
/// read-only access to deterministic environment facts via `Env` and must stay
/// side-effect free.
pub trait ActionTransition<Env> {
    type Error;

    /// Validates pre-conditions using the state **before** mutation.
    fn pre_validate(&self, _state: &GameState, _env: &Env) -> Result<(), Self::Error> {
        Ok(())
    }

    /// Applies the action to the state. Implementations should assume that
    /// `pre_validate` has already run successfully.
    fn apply(&self, state: &mut GameState, env: &Env) -> Result<(), Self::Error>;

    /// Validates post-conditions using the state **after** mutation.
    fn post_validate(&self, _state: &GameState, _env: &Env) -> Result<(), Self::Error> {
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use core::cell::Cell;

    use super::ActionTransition;
    use crate::state::GameState;

    struct NoopAction;

    impl ActionTransition<()> for NoopAction {
        type Error = core::convert::Infallible;

        fn apply(&self, _state: &mut GameState, _env: &()) -> Result<(), Self::Error> {
            Ok(())
        }
    }

    #[test]
    fn default_hooks_allow_apply_to_run() {
        let mut state = GameState::default();
        let env = ();
        let action = NoopAction;

        action.pre_validate(&state, &env).unwrap();
        action.apply(&mut state, &env).unwrap();
        action.post_validate(&state, &env).unwrap();
    }

    struct CountingAction<'a> {
        pre_count: &'a Cell<u32>,
        post_count: &'a Cell<u32>,
    }

    impl<'a> ActionTransition<()> for CountingAction<'a> {
        type Error = core::convert::Infallible;

        fn pre_validate(&self, _state: &GameState, _env: &()) -> Result<(), Self::Error> {
            self.pre_count.set(self.pre_count.get() + 1);
            Ok(())
        }

        fn apply(&self, _state: &mut GameState, _env: &()) -> Result<(), Self::Error> {
            Ok(())
        }

        fn post_validate(&self, _state: &GameState, _env: &()) -> Result<(), Self::Error> {
            self.post_count.set(self.post_count.get() + 1);
            Ok(())
        }
    }

    #[test]
    fn custom_hooks_run_in_expected_order() {
        let mut state = GameState::default();
        let env = ();
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
}
