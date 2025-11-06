//! Handler for entity activation based on player proximity.

use game_core::{Action, ActivationAction, EntityId, SystemActionKind};

use super::{EventContext, HandlerCriticality};
use crate::events::GameEvent;
use crate::providers::SystemActionHandler;

/// Handler that activates/deactivates NPCs based on player proximity.
///
/// This handler implements the activation radius mechanic, ensuring only nearby
/// entities are scheduled for turns. This improves performance in large maps
/// and provides a natural "fog of war" effect.
///
/// # Behavior
///
/// When the player moves:
/// 1. Trigger an Activation system action
/// 2. That action updates the active set based on proximity
#[derive(Debug, Clone, Copy)]
pub struct ActivationHandler;

impl SystemActionHandler for ActivationHandler {
    fn name(&self) -> &'static str {
        "activation"
    }

    fn priority(&self) -> i32 {
        -10 // After cost and death, before optional handlers
    }

    fn criticality(&self) -> HandlerCriticality {
        // Important: Activation affects gameplay but isn't critical for state consistency.
        // If it fails, NPCs might not activate/deactivate correctly, but game state
        // remains valid.
        HandlerCriticality::Important
    }

    fn generate_actions(&self, event: &GameEvent, _ctx: &EventContext) -> Vec<Action> {
        match event {
            GameEvent::EntityMoved { entity, .. } => {
                // Only trigger on player movement
                if *entity == EntityId::PLAYER {
                    vec![Action::system(SystemActionKind::Activation(
                        ActivationAction,
                    ))]
                } else {
                    vec![]
                }
            }
            _ => vec![],
        }
    }
}
