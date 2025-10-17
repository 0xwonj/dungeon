//! Hook that applies action costs to actor ready_at timestamps.

use game_core::{Action, ActionCostAction, SystemActionKind};

use super::{HookContext, HookCriticality, PostExecutionHook};

/// Hook that updates actor ready_at timestamps based on action costs.
///
/// This hook is critical for turn scheduling - without it, actors could
/// take unlimited actions. It executes early (priority -100) to ensure
/// timing is updated before any other hooks run.
///
/// # Behavior
///
/// For every non-system action, this hook:
/// 1. Calculates the speed-scaled cost of the action
/// 2. Creates an ActionCostAction system action
/// 3. Applies the cost to the actor's ready_at timestamp
///
/// System actions (priority, activation, etc.) have zero cost and don't trigger this hook.
#[derive(Debug, Clone, Copy)]
pub struct ActionCostHook;

impl PostExecutionHook for ActionCostHook {
    fn name(&self) -> &'static str {
        "action_cost"
    }

    fn priority(&self) -> i32 {
        -100 // Execute very early
    }

    fn criticality(&self) -> HookCriticality {
        // Critical: This hook is essential for turn scheduling and game state consistency.
        // If it fails, actors could take unlimited actions or timing could become corrupted.
        HookCriticality::Critical
    }

    fn should_trigger(&self, ctx: &HookContext<'_>) -> bool {
        // Only apply cost to non-system actions
        !ctx.delta.action.actor().is_system()
    }

    fn create_actions(&self, ctx: &HookContext<'_>) -> Vec<Action> {
        let actor_id = ctx.delta.action.actor();

        // Get actor stats for cost calculation
        let Some(actor) = ctx.state.entities.actor(actor_id) else {
            return vec![];
        };
        let snapshot = actor.snapshot();

        // Calculate speed-scaled cost
        let cost = ctx.delta.action.cost(&snapshot);

        // Create system action to apply the cost
        vec![Action::system(SystemActionKind::ActionCost(
            ActionCostAction::new(actor_id, cost),
        ))]
    }
}
