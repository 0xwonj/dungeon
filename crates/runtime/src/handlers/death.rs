//! Handler for entity death.

use game_core::action::{Action, RemoveFromActiveAction, SystemActionKind};

use super::{EventContext, HandlerCriticality};
use crate::events::GameEvent;
use crate::providers::SystemActionHandler;

/// Handler that removes dead entities from the active set.
///
/// This handler reacts to EntityDied events and generates RemoveFromActive
/// system actions to clean up dead actors from turn scheduling.
#[derive(Debug, Clone, Copy)]
pub struct DeathHandler;

impl SystemActionHandler for DeathHandler {
    fn name(&self) -> &'static str {
        "death"
    }

    fn priority(&self) -> i32 {
        -50 // After ActionCost, before optional handlers
    }

    fn criticality(&self) -> HandlerCriticality {
        // Critical: Death handling is essential for game state consistency.
        // If this fails, dead actors could still take turns.
        HandlerCriticality::Critical
    }

    fn generate_actions(&self, event: &GameEvent, _ctx: &EventContext) -> Vec<Action> {
        match event {
            GameEvent::EntityDied { entity, .. } => {
                // Remove dead entity from active set
                vec![Action::system(SystemActionKind::RemoveFromActive(
                    RemoveFromActiveAction::new(*entity),
                ))]
            }
            _ => vec![],
        }
    }
}
