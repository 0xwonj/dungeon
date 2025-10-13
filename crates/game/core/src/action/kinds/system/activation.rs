//! Entity activation system action.
//!
//! Manages NPC activation and deactivation based on proximity to the player,
//! implementing the activation radius game mechanic.

use crate::action::ActionTransition;
use crate::env::GameEnv;
use crate::state::{EntityId, GameState, Position, Tick};

/// System action that updates entity activation status based on player proximity.
///
/// NPCs within the activation radius are added to the active set and scheduled
/// to act. NPCs outside the radius are deactivated and removed from scheduling.
///
/// This action is typically triggered after the player moves, ensuring that only
/// nearby entities consume processing time and maintain responsiveness in large maps.
///
/// # Invariants
///
/// - Player must exist at the specified position
/// - Activated entities receive an initial `ready_at` based on Wait action cost
/// - Deactivated entities have their `ready_at` cleared
/// - Active set and `ready_at` timestamps remain synchronized
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ActivationAction {
    /// Player's current position (center of activation radius)
    pub player_position: Position,
}

impl ActivationAction {
    /// Creates a new activation update for the given player position.
    pub fn new(player_position: Position) -> Self {
        Self { player_position }
    }

    /// Calculates the grid distance between two positions (Chebyshev distance).
    fn grid_distance(a: Position, b: Position) -> u32 {
        let dx = (a.x - b.x).unsigned_abs();
        let dy = (a.y - b.y).unsigned_abs();
        dx.max(dy)
    }
}

impl ActionTransition for ActivationAction {
    type Error = ActivationError;

    fn pre_validate(&self, state: &GameState, _env: &GameEnv<'_>) -> Result<(), Self::Error> {
        // Verify player exists (should always be true, but defensive check)
        if state.entities.player.id != EntityId::PLAYER {
            return Err(ActivationError::PlayerNotFound);
        }

        Ok(())
    }

    fn apply(&self, state: &mut GameState, env: &GameEnv<'_>) -> Result<(), Self::Error> {
        let activation_radius = env.activation_radius();
        let clock = state.turn.clock;

        // Collect NPC data to avoid borrow checker issues
        let npc_data: Vec<_> = state
            .entities
            .npcs
            .iter()
            .map(|npc| {
                let is_active = state.turn.active_actors.contains(&npc.id);
                (npc.id, npc.position, is_active, npc.stats.clone())
            })
            .collect();

        // Process each NPC's activation status
        for (entity_id, npc_position, is_active, stats) in npc_data {
            let distance = Self::grid_distance(self.player_position, npc_position);

            if distance <= activation_radius {
                // Within activation radius - activate if not already active
                if !is_active {
                    state.turn.active_actors.insert(entity_id);

                    // Set initial ready_at using Wait action cost (100 ticks scaled by speed)
                    // This gives the NPC time to "wake up" before acting
                    let speed = stats.speed.max(1) as u64;
                    let delay = Tick(100 * 100 / speed);

                    if let Some(actor) = state.entities.actor_mut(entity_id) {
                        actor.ready_at = Some(Tick(clock.0 + delay.0));
                    }
                }
            } else if is_active {
                // Outside activation radius - deactivate if currently active
                state.turn.active_actors.remove(&entity_id);

                if let Some(actor) = state.entities.actor_mut(entity_id) {
                    actor.ready_at = None;
                }
            }
        }

        Ok(())
    }

    fn post_validate(&self, state: &GameState, _env: &GameEnv<'_>) -> Result<(), Self::Error> {
        // Verify invariant: all entities in active_actors must have ready_at
        for &entity_id in &state.turn.active_actors {
            if let Some(actor) = state.entities.actor(entity_id) {
                debug_assert!(
                    actor.ready_at.is_some(),
                    "active actor {:?} must have ready_at timestamp",
                    entity_id
                );
            }
        }

        Ok(())
    }

    fn cost(&self) -> Tick {
        // System actions have no time cost
        Tick::ZERO
    }
}

/// Errors that can occur during activation updates.
#[derive(Clone, Copy, Debug, PartialEq, Eq, thiserror::Error)]
pub enum ActivationError {
    #[error("player not found in game state")]
    PlayerNotFound,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::env::Env;
    use crate::state::{ActorState, ActorStats, InventoryState, ResourceMeter};

    fn create_test_actor(id: EntityId, position: Position) -> ActorState {
        ActorState::new(
            id,
            position,
            ActorStats::default(),
            InventoryState::default(),
        )
    }

    struct TestConfig;
    impl crate::env::ConfigOracle for TestConfig {
        fn activation_radius(&self) -> u32 {
            5
        }
    }

    #[test]
    fn activates_npcs_within_radius() {
        let mut state = GameState::default();

        // Player at origin
        state.entities.player = create_test_actor(EntityId::PLAYER, Position::ORIGIN);

        // NPC within activation radius (5 tiles)
        state
            .entities
            .npcs
            .push(create_test_actor(EntityId(1), Position::new(3, 3)))
            .unwrap();

        let action = ActivationAction::new(Position::ORIGIN);
        static CONFIG: TestConfig = TestConfig;
        let env: GameEnv = Env::new(
            None,
            None,
            None,
            None,
            Some(&CONFIG as &dyn crate::env::ConfigOracle),
        );

        action.apply(&mut state, &env).unwrap();

        // NPC should be activated
        assert!(state.turn.active_actors.contains(&EntityId(1)));
        assert!(state.entities.npcs[0].ready_at.is_some());
    }

    #[test]
    fn does_not_activate_npcs_outside_radius() {
        let mut state = GameState::default();

        state.entities.player = create_test_actor(EntityId::PLAYER, Position::ORIGIN);

        // NPC outside activation radius
        state
            .entities
            .npcs
            .push(create_test_actor(EntityId(1), Position::new(10, 10)))
            .unwrap();

        let action = ActivationAction::new(Position::ORIGIN);
        static CONFIG: TestConfig = TestConfig;
        let env: GameEnv = Env::new(
            None,
            None,
            None,
            None,
            Some(&CONFIG as &dyn crate::env::ConfigOracle),
        );

        action.apply(&mut state, &env).unwrap();

        // NPC should not be activated
        assert!(!state.turn.active_actors.contains(&EntityId(1)));
        assert!(state.entities.npcs[0].ready_at.is_none());
    }

    #[test]
    fn deactivates_npcs_outside_radius() {
        let mut state = GameState::default();

        state.entities.player = create_test_actor(EntityId::PLAYER, Position::ORIGIN);

        // NPC that is currently active
        let mut npc = create_test_actor(EntityId(1), Position::new(10, 10));
        npc.ready_at = Some(Tick(100));
        state.entities.npcs.push(npc).unwrap();
        state.turn.active_actors.insert(EntityId(1));

        let action = ActivationAction::new(Position::ORIGIN);
        static CONFIG: TestConfig = TestConfig;
        let env: GameEnv = Env::new(
            None,
            None,
            None,
            None,
            Some(&CONFIG as &dyn crate::env::ConfigOracle),
        );

        action.apply(&mut state, &env).unwrap();

        // NPC should be deactivated
        assert!(!state.turn.active_actors.contains(&EntityId(1)));
        assert!(state.entities.npcs[0].ready_at.is_none());
    }

    #[test]
    fn uses_chebyshev_distance() {
        let mut state = GameState::default();

        state.entities.player = create_test_actor(EntityId::PLAYER, Position::ORIGIN);

        // NPC at diagonal position - Chebyshev distance = max(5, 5) = 5
        state
            .entities
            .npcs
            .push(create_test_actor(EntityId(1), Position::new(5, 5)))
            .unwrap();

        let action = ActivationAction::new(Position::ORIGIN);
        static CONFIG: TestConfig = TestConfig;
        let env: GameEnv = Env::new(
            None,
            None,
            None,
            None,
            Some(&CONFIG as &dyn crate::env::ConfigOracle),
        );

        action.apply(&mut state, &env).unwrap();

        // Should be activated (distance == radius)
        assert!(state.turn.active_actors.contains(&EntityId(1)));
    }

    #[test]
    fn scales_ready_at_by_speed() {
        let mut state = GameState::default();

        state.entities.player = create_test_actor(EntityId::PLAYER, Position::ORIGIN);
        state.turn.clock = Tick(1000);

        // Fast NPC (speed 200)
        let stats = ActorStats::new(
            ResourceMeter::new(100, 100),
            ResourceMeter::new(100, 100),
            200,
        );
        let npc = ActorState::new(
            EntityId(1),
            Position::new(3, 3),
            stats,
            InventoryState::default(),
        );
        state.entities.npcs.push(npc).unwrap();

        let action = ActivationAction::new(Position::ORIGIN);
        static CONFIG: TestConfig = TestConfig;
        let env: GameEnv = Env::new(
            None,
            None,
            None,
            None,
            Some(&CONFIG as &dyn crate::env::ConfigOracle),
        );

        action.apply(&mut state, &env).unwrap();

        // With speed 200, delay should be 100 * 100 / 200 = 50
        // ready_at = clock + delay = 1000 + 50 = 1050
        assert_eq!(state.entities.npcs[0].ready_at, Some(Tick(1050)));
    }
}
