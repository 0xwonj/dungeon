use game_core::{Action, EntityId, Position};

/// Events emitted by the runtime during game simulation
#[derive(Debug, Clone)]
pub enum GameEvent {
    /// A turn was completed by an entity
    TurnCompleted { entity: EntityId },
    /// An action was executed
    ActionExecuted { action: Action },
    /// An entity moved
    EntityMoved {
        entity: EntityId,
        from: Position,
        to: Position,
    },
    /// Game state changed
    StateChanged,
}
