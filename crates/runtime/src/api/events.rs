//! Events emitted during simulation for front-ends to observe.
//!
//! Consumers subscribe to [`GameEvent`] to react to state changes without
//! blocking the worker loop.
use game_core::{Action, EntityId, StateDelta, Tick, engine::TransitionPhase};

/// Events emitted by the runtime during game simulation
#[derive(Debug, Clone)]
pub enum GameEvent {
    /// A turn was completed by an entity
    TurnCompleted { entity: EntityId },
    /// An action was executed with resulting state changes
    ActionExecuted {
        action: Action,
        delta: Box<StateDelta>,
        clock: Tick,
    },
    /// An action failed during execution pipeline
    ActionFailed {
        action: Action,
        phase: TransitionPhase,
        error: String,
        clock: Tick,
    },
}
