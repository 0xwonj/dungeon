use crate::env::GameEnv;
use crate::state::{GameState, Tick};

/// Defines how a concrete action variant mutates game state while mirroring
/// the constraint checks enforced inside zk circuits.
///
/// Implementors can override the validation hooks to surface pre- and
/// post-conditions that must hold around the state mutation. All hooks receive
/// read-only access to deterministic environment facts via `Env` and must stay
/// side-effect free.
pub trait ActionTransition {
    type Error;

    /// Returns the time cost of this action in ticks.
    /// This cost is used to advance the actor's ready_at value.
    fn cost(&self) -> Tick;

    /// Validates pre-conditions using the state **before** mutation.
    fn pre_validate(&self, _state: &GameState, _env: &GameEnv<'_>) -> Result<(), Self::Error> {
        Ok(())
    }

    /// Applies the action by mutating the game state directly. Implementations should
    /// assume that `pre_validate` has already run successfully.
    fn apply(&self, state: &mut GameState, env: &GameEnv<'_>) -> Result<(), Self::Error>;

    /// Validates post-conditions using the state **after** mutation.
    fn post_validate(&self, _state: &GameState, _env: &GameEnv<'_>) -> Result<(), Self::Error> {
        Ok(())
    }
}
