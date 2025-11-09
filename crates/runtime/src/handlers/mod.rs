//! Handlers for reactive action generation.
//!
//! Handlers react to GameEvents and generate system actions in response.
//! These are used by SystemActionProvider to implement game logic.

mod action_cost;
mod activation;
mod death;

pub use action_cost::ActionCostHandler;
pub use activation::ActivationHandler;
pub use death::DeathHandler;

use game_core::GameState;

use crate::oracle::OracleManager;

/// Criticality level for handler errors.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HandlerCriticality {
    /// Handler failure causes the entire action to fail.
    Critical,
    /// Handler failure is logged but execution continues.
    Important,
    /// Handler failure can be ignored.
    Optional,
}

/// Context provided to handlers for reactive action generation.
pub struct EventContext<'a> {
    /// Game state before the action that produced the event
    pub state_before: &'a GameState,
    /// Game state after the action that produced the event
    pub state_after: &'a GameState,
    /// Oracle manager for accessing game content
    pub oracles: &'a OracleManager,
}
