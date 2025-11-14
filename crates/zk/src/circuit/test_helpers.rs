//! Shared test helpers for circuit testing.
//!
//! This module provides reusable test state builders to reduce code duplication
//! across test files and benchmarks.

use game_core::state::BoundedVec;
use game_core::{
    ActorState, CoreStats, EntitiesState, EntityId, GameState, InventoryState, Position, TurnState,
    WorldState,
};

/// Create a simple test state with a single actor at the given position.
///
/// # Arguments
/// * `actor_position` - Position for the actor
///
/// # Returns
/// A GameState with one actor (PLAYER) at the specified position
pub fn create_test_state_at_position(actor_position: Position) -> GameState {
    let actor = ActorState::new(
        EntityId::PLAYER,
        actor_position,
        CoreStats::default(),
        InventoryState::default(),
    );

    let entities = EntitiesState::new(
        unsafe { BoundedVec::from_vec_unchecked(vec![actor]) },
        BoundedVec::new(),
        BoundedVec::new(),
    );

    GameState::new(TurnState::default(), entities, WorldState::default())
}

/// Create a test state with a player at (5, 5) and optionally an enemy at (6, 6).
///
/// # Arguments
/// * `with_enemy` - If true, adds an enemy actor at (6, 6)
///
/// # Returns
/// A GameState with player and optionally one enemy
pub fn create_test_state_with_enemy(with_enemy: bool) -> GameState {
    let mut entities = EntitiesState::empty();

    let default_stats = CoreStats {
        str: 10,
        con: 10,
        dex: 10,
        int: 10,
        wil: 10,
        ego: 10,
        level: 1,
    };

    // Add player actor at (5, 5)
    let player = ActorState::new(
        EntityId::PLAYER,
        Position::new(5, 5),
        default_stats.clone(),
        InventoryState::empty(),
    );
    let _ = entities.actors.push(player);

    // Optionally add enemy actor at (6, 6)
    if with_enemy {
        let enemy = ActorState::new(
            EntityId(1),
            Position::new(6, 6),
            default_stats.clone(),
            InventoryState::empty(),
        );
        let _ = entities.actors.push(enemy);
    }

    GameState::new(TurnState::default(), entities, WorldState::default())
}

/// Create a test state with multiple actors for benchmarking.
///
/// # Arguments
/// * `num_actors` - Number of actors to create (including the player)
///
/// # Returns
/// A GameState with the specified number of actors
pub fn create_test_state_with_actors(num_actors: usize) -> GameState {
    let mut entities = EntitiesState::empty();

    let default_stats = CoreStats {
        str: 10,
        con: 10,
        dex: 10,
        int: 10,
        wil: 10,
        ego: 10,
        level: 1,
    };

    // Add player
    let player = ActorState::new(
        EntityId::PLAYER,
        Position::new(5, 5),
        default_stats.clone(),
        InventoryState::empty(),
    );
    let _ = entities.actors.push(player);

    // Add additional actors
    for i in 1..num_actors {
        let actor = ActorState::new(
            EntityId(i as u32),
            Position::new((5 + i) as i32, 5),
            default_stats.clone(),
            InventoryState::empty(),
        );
        let _ = entities.actors.push(actor);
    }

    GameState::new(TurnState::default(), entities, WorldState::default())
}

/// Create a basic test state with just a player at (5, 5).
///
/// This is a convenience function for tests that just need a simple state.
pub fn create_basic_test_state() -> GameState {
    create_test_state_with_enemy(false)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_create_test_state_at_position() {
        let state = create_test_state_at_position(Position::new(3, 7));
        assert_eq!(state.entities.actors.len(), 1);
        assert_eq!(state.entities.actors[0].id, EntityId::PLAYER);
        assert_eq!(state.entities.actors[0].position, Position::new(3, 7));
    }

    #[test]
    fn test_create_test_state_with_enemy() {
        let state_no_enemy = create_test_state_with_enemy(false);
        assert_eq!(state_no_enemy.entities.actors.len(), 1);

        let state_with_enemy = create_test_state_with_enemy(true);
        assert_eq!(state_with_enemy.entities.actors.len(), 2);
        assert_eq!(state_with_enemy.entities.actors[1].id, EntityId(1));
    }

    #[test]
    fn test_create_test_state_with_actors() {
        let state = create_test_state_with_actors(5);
        assert_eq!(state.entities.actors.len(), 5);
        assert_eq!(state.entities.actors[0].id, EntityId::PLAYER);
        assert_eq!(state.entities.actors[4].id, EntityId(4));
    }

    #[test]
    fn test_create_basic_test_state() {
        let state = create_basic_test_state();
        assert_eq!(state.entities.actors.len(), 1);
        assert_eq!(state.entities.actors[0].position, Position::new(5, 5));
    }
}
