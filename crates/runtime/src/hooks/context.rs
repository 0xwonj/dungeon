//! Execution context provided to hooks during evaluation.

use game_core::{GameState, StateDelta};

use crate::oracle::OracleManager;

/// Context provided to hooks when evaluating whether to trigger.
///
/// This struct provides read-only access to the state delta and game state,
/// allowing hooks to make informed decisions about whether to generate system actions.
///
/// # Design
///
/// HookContext follows the Context Object pattern, bundling related information
/// that hooks need without exposing the entire worker state.
pub struct HookContext<'a> {
    /// The delta produced by the just-executed action
    pub delta: &'a StateDelta,

    /// Current game state (post-action, pre-hooks)
    pub state: &'a GameState,

    /// Oracle manager for accessing game content
    pub oracles: &'a OracleManager,
}
