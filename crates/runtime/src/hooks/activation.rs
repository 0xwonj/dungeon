//! Hook that manages entity activation based on player proximity.

use game_core::{Action, ActionKind, ActivationAction, EntityId};

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
        ctx.delta.action.actor == EntityId::PLAYER
            && ctx
                .delta
                .entities
                .player
                .as_ref()
                .and_then(|patch| patch.position)
                .is_some()
    }

    fn create_actions(&self, ctx: &HookContext<'_>) -> Vec<Action> {
        // Get player's current position
        let player_position = ctx.state.entities.player.position;

        // Create activation system action
        vec![Action::new(
            EntityId::SYSTEM,
            ActionKind::Activation(ActivationAction::new(player_position)),
        )]
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use game_core::{
        ActorState, ActorStats, CardinalDirection, GameState, InventoryState, MoveAction, Position,
        StateDelta,
    };

    use crate::oracle::OracleManager;

    fn create_test_context(
        action: Action,
        before: &GameState,
        after: &GameState,
    ) -> (StateDelta, OracleManager) {
        let delta = StateDelta::from_states(action, before, after);
        let oracles = OracleManager::test_manager();
        (delta, oracles)
    }

    #[test]
    fn triggers_when_player_moves() {
        let mut before = GameState::default();
        before.entities.player = ActorState::new(
            EntityId::PLAYER,
            Position::ORIGIN,
            ActorStats::default(),
            InventoryState::default(),
        );

        let mut after = before.clone();
        after.entities.player.position = Position::new(0, 1);

        let action = Action::new(
            EntityId::PLAYER,
            ActionKind::Move(MoveAction::new(
                EntityId::PLAYER,
                CardinalDirection::North,
                1,
            )),
        );

        let (delta, oracles) = create_test_context(action, &before, &after);
        let ctx = HookContext {
            delta: &delta,
            state: &after,
            oracles: &oracles,
        };

        let hook = ActivationHook;
        assert!(hook.should_trigger(&ctx));
    }

    #[test]
    fn does_not_trigger_for_non_movement() {
        let state = GameState::default();
        let action = Action::new(EntityId::PLAYER, ActionKind::Wait);

        let (delta, oracles) = create_test_context(action, &state, &state);
        let ctx = HookContext {
            delta: &delta,
            state: &state,
            oracles: &oracles,
        };

        let hook = ActivationHook;
        assert!(!hook.should_trigger(&ctx));
    }

    #[test]
    fn does_not_trigger_for_npc_movement() {
        let mut before = GameState::default();
        let mut npc = ActorState::new(
            EntityId(1),
            Position::ORIGIN,
            ActorStats::default(),
            InventoryState::default(),
        );
        before.entities.npcs.push(npc.clone()).unwrap();

        let mut after = before.clone();
        npc.position = Position::new(0, 1);
        after.entities.npcs[0] = npc;

        let action = Action::new(
            EntityId(1),
            ActionKind::Move(MoveAction::new(EntityId(1), CardinalDirection::North, 1)),
        );

        let (delta, oracles) = create_test_context(action, &before, &after);
        let ctx = HookContext {
            delta: &delta,
            state: &after,
            oracles: &oracles,
        };

        let hook = ActivationHook;
        assert!(!hook.should_trigger(&ctx));
    }

    #[test]
    fn creates_activation_action() {
        let mut before = GameState::default();
        before.entities.player = ActorState::new(
            EntityId::PLAYER,
            Position::ORIGIN,
            ActorStats::default(),
            InventoryState::default(),
        );

        let mut after = before.clone();
        after.entities.player.position = Position::new(0, 1);

        let action = Action::new(
            EntityId::PLAYER,
            ActionKind::Move(MoveAction::new(
                EntityId::PLAYER,
                CardinalDirection::North,
                1,
            )),
        );

        let (delta, oracles) = create_test_context(action, &before, &after);
        let ctx = HookContext {
            delta: &delta,
            state: &after,
            oracles: &oracles,
        };

        let hook = ActivationHook;
        let actions = hook.create_actions(&ctx);

        assert_eq!(actions.len(), 1);
        let action = &actions[0];
        assert_eq!(action.actor, EntityId::SYSTEM);
        assert!(matches!(action.kind, ActionKind::Activation(_)));
    }
}
