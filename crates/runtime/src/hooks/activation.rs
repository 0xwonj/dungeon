//! Hook that manages entity activation based on player proximity.

use game_core::{Action, ActivationAction, ActorFields, EntityId, SystemActionKind};

use super::{HookContext, HookCriticality, PostExecutionHook};

/// Hook that activates/deactivates NPCs based on player proximity.
///
/// This hook implements the activation radius mechanic, ensuring only nearby
/// entities are scheduled for turns. This improves performance in large maps
/// and provides a natural "fog of war" effect.
///
/// # Behavior
///
/// When the player moves:
/// 1. NPCs within activation radius are added to the active set
/// 2. NPCs outside activation radius are removed from the active set
/// 3. Activated NPCs receive an initial ready_at based on Wait action cost
///
/// This hook runs at priority -10, after ActionCostHook but before most other hooks.
#[derive(Debug, Clone, Copy)]
pub struct ActivationHook;

impl PostExecutionHook for ActivationHook {
    fn name(&self) -> &'static str {
        "activation"
    }

    fn priority(&self) -> i32 {
        -10 // After cost, before optional hooks
    }

    fn criticality(&self) -> HookCriticality {
        // Important: Activation affects gameplay but isn't critical for state consistency.
        // If it fails, NPCs might not activate/deactivate correctly, but game state
        // remains valid. This is the default level.
        HookCriticality::Important
    }

    fn should_trigger(&self, ctx: &HookContext<'_>) -> bool {
        // Trigger only when player moves
        ctx.delta.action.actor() == EntityId::PLAYER
            && ctx
                .delta
                .entities
                .actors
                .updated
                .iter()
                .find(|changes| changes.id == EntityId::PLAYER)
                .map(|changes| changes.fields.contains(ActorFields::POSITION))
                .unwrap_or(false)
    }

    fn create_actions(&self, _ctx: &HookContext<'_>) -> Vec<Action> {
        // Get player's current position

        // Create activation system action
        vec![Action::system(SystemActionKind::Activation(
            ActivationAction,
        ))]
    }
}
