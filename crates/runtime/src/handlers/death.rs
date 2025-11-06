//! Handler for entity death.

use game_core::action::{Action, DeactivateAction, RemoveFromWorldAction, SystemActionKind};

use super::{EventContext, HandlerCriticality};
use crate::events::GameEvent;
use crate::providers::SystemActionHandler;

/// Handler that cleans up dead entities.
///
/// This handler reacts to EntityDied events and generates system actions
/// to properly remove dead entities from the game world:
/// 1. RemoveFromWorld - Clears position and world occupancy
/// 2. Deactivate - Removes from active set and turn scheduling
///
/// # Design Philosophy
///
/// Actions are single-responsibility primitives. The handler composes them
/// based on the entity's current state, generating only necessary actions.
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
        // If this fails, dead actors could still take turns or remain on the map.
        HandlerCriticality::Critical
    }

    fn generate_actions(&self, event: &GameEvent, ctx: &EventContext) -> Vec<Action> {
        match event {
            GameEvent::EntityDied { entity, .. } => {
                let mut actions = Vec::new();

                // Check entity state to determine which cleanup actions are needed
                if let Some(actor) = ctx.state_after.entities.actor(*entity) {
                    // IMPORTANT: Deactivate FIRST, then remove from world
                    // This prevents ActivationAction from seeing a half-cleaned state
                    // (position = None but still in active_actors)

                    // If entity is in active set, deactivate
                    if actor.ready_at.is_some()
                        || ctx.state_after.turn.active_actors.contains(entity)
                    {
                        tracing::info!(
                            target: "runtime::handlers::death",
                            entity = ?entity,
                            "DeathHandler: Generating DeactivateAction (entity is active)"
                        );
                        actions.push(Action::system(SystemActionKind::Deactivate(
                            DeactivateAction::new(*entity),
                        )));
                    }

                    // If entity has a position, remove from world
                    if actor.position.is_some() {
                        tracing::info!(
                            target: "runtime::handlers::death",
                            entity = ?entity,
                            "DeathHandler: Generating RemoveFromWorldAction (entity has position)"
                        );
                        actions.push(Action::system(SystemActionKind::RemoveFromWorld(
                            RemoveFromWorldAction::new(*entity),
                        )));
                    }

                    if actions.is_empty() {
                        tracing::debug!(
                            target: "runtime::handlers::death",
                            entity = ?entity,
                            "DeathHandler: No cleanup needed (entity already removed)"
                        );
                    }
                } else {
                    tracing::warn!(
                        target: "runtime::handlers::death",
                        entity = ?entity,
                        "DeathHandler: Entity not found in state"
                    );
                }

                actions
            }
            _ => vec![],
        }
    }
}
