//! Handler for applying action costs to actors.

use game_core::{Action, ActionCostAction, SystemActionKind};

use super::{EventContext, HandlerCriticality};
use crate::events::GameEvent;
use crate::providers::SystemActionHandler;

/// Handler that applies action costs to actor ready_at timestamps.
///
/// This handler is critical for turn scheduling - without it, actors could
/// take unlimited actions. It executes with high priority to ensure timing
/// is updated before other handlers run.
#[derive(Debug, Clone, Copy)]
pub struct ActionCostHandler;

impl SystemActionHandler for ActionCostHandler {
    fn name(&self) -> &'static str {
        "action_cost"
    }

    fn priority(&self) -> i32 {
        -100 // Execute very early
    }

    fn criticality(&self) -> HandlerCriticality {
        // Critical: This handler is essential for turn scheduling and game state consistency.
        // If it fails, actors could take unlimited actions or timing could become corrupted.
        HandlerCriticality::Critical
    }

    fn generate_actions(&self, event: &GameEvent, ctx: &EventContext) -> Vec<Action> {
        match event {
            GameEvent::ActionCompleted { actor, action, .. } => {
                // Get actor stats from BEFORE the action for cost calculation
                let Some(actor_state) = ctx.state_before.entities.actor(*actor) else {
                    return vec![];
                };
                let snapshot = actor_state.snapshot();

                // Calculate speed-scaled cost
                let env = ctx.oracles.as_game_env();
                let cost = action.cost(&snapshot, &env);

                // Create system action to apply the cost
                vec![Action::system(SystemActionKind::ActionCost(
                    ActionCostAction::new(*actor, cost),
                ))]
            }
            _ => vec![],
        }
    }
}
