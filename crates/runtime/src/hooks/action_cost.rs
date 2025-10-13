//! Hook that applies action costs to actor ready_at timestamps.

use game_core::{Action, ActionCostAction, ActionKind, EntityId};

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
        !ctx.delta.action.actor.is_system()
    }

    fn create_actions(&self, ctx: &HookContext<'_>) -> Vec<Action> {
        let actor_id = ctx.delta.action.actor;

        // Get actor stats for cost calculation
        let Some(actor) = ctx.state.entities.actor(actor_id) else {
            return vec![];
        };
        let stats = actor.stats.clone();

        // Calculate speed-scaled cost
        let cost = ctx.delta.action.cost(&stats);

        // Create system action to apply the cost
        vec![Action::new(
            EntityId::SYSTEM,
            ActionKind::ActionCost(ActionCostAction::new(actor_id, cost)),
        )]
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use game_core::{
        ActorState, ActorStats, CardinalDirection, GameState, InventoryState, MoveAction, Position,
        StateDelta, Tick,
    };

    use crate::oracle::OracleManager;

    fn create_test_context(action: Action, state: &GameState) -> (StateDelta, OracleManager) {
        let before = GameState::default();
        let delta = StateDelta::from_states(action, &before, state);
        let oracles = OracleManager::test_manager();
        (delta, oracles)
    }

    #[test]
    fn triggers_for_player_actions() {
        let mut state = GameState::default();
        state.entities.player = ActorState::new(
            EntityId::PLAYER,
            Position::ORIGIN,
            ActorStats::default(),
            InventoryState::default(),
        )
        .with_ready_at(Tick(0));

        let action = Action::new(
            EntityId::PLAYER,
            ActionKind::Move(MoveAction::new(
                EntityId::PLAYER,
                CardinalDirection::North,
                1,
            )),
        );

        let (delta, oracles) = create_test_context(action, &state);
        let ctx = HookContext {
            delta: &delta,
            state: &state,
            oracles: &oracles,
        };

        let hook = ActionCostHook;
        assert!(hook.should_trigger(&ctx));
    }

    #[test]
    fn does_not_trigger_for_system_actions() {
        let state = GameState::default();
        let action = Action::new(
            EntityId::SYSTEM,
            ActionKind::PrepareTurn(game_core::PrepareTurnAction),
        );

        let (delta, oracles) = create_test_context(action, &state);
        let ctx = HookContext {
            delta: &delta,
            state: &state,
            oracles: &oracles,
        };

        let hook = ActionCostHook;
        assert!(!hook.should_trigger(&ctx));
    }

    #[test]
    fn creates_action_cost_action() {
        let mut state = GameState::default();
        state.entities.player = ActorState::new(
            EntityId::PLAYER,
            Position::ORIGIN,
            ActorStats::default(),
            InventoryState::default(),
        )
        .with_ready_at(Tick(0));

        let action = Action::new(
            EntityId::PLAYER,
            ActionKind::Move(MoveAction::new(
                EntityId::PLAYER,
                CardinalDirection::North,
                1,
            )),
        );

        let (delta, oracles) = create_test_context(action, &state);
        let ctx = HookContext {
            delta: &delta,
            state: &state,
            oracles: &oracles,
        };

        let hook = ActionCostHook;
        let actions = hook.create_actions(&ctx);

        assert_eq!(actions.len(), 1);
        let action = &actions[0];
        assert_eq!(action.actor, EntityId::SYSTEM);
        assert!(matches!(action.kind, ActionKind::ActionCost(_)));
    }
}
