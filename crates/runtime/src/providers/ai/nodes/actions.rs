//! Action nodes for AI behavior trees.
//!
//! Action nodes generate `Action` objects and store them in the context.
//! They represent concrete things an entity can do.

use behavior_tree::{Behavior, Status};
use game_core::{
    Action, AttackAction, AttackStyle, CardinalDirection, CharacterActionKind, MoveAction,
    Position,
};

use crate::providers::ai::AiContext;

/// Moves toward the player by one step.
///
/// Calculates the direction that brings the entity closer to the player
/// and generates a Move action. Uses Manhattan distance heuristic.
///
/// # Example
///
/// ```rust,ignore
/// use behavior_tree::Selector;
/// use runtime::providers::ai::nodes::*;
///
/// // Try to attack, otherwise move toward player
/// Selector::new(vec![
///     Box::new(AttackPlayer),
///     Box::new(MoveTowardPlayer),
/// ])
/// ```
pub struct MoveTowardPlayer;

impl Behavior<AiContext<'_>> for MoveTowardPlayer {
    fn tick(&self, ctx: &mut AiContext) -> Status {
        let actor = match ctx.state.entities.actor(ctx.entity) {
            Some(a) => a,
            None => return Status::Failure,
        };

        let player_pos = ctx.state.entities.player.position;
        let npc_pos = actor.position;

        // Calculate direction toward player
        let direction =
            calculate_direction_toward(npc_pos.x, npc_pos.y, player_pos.x, player_pos.y);

        match direction {
            Some(dir) => {
                // Calculate destination position
                let (dx, dy) = dir.delta();
                let destination = Position {
                    x: npc_pos.x + dx,
                    y: npc_pos.y + dy,
                };

                // Check if destination is valid (passable terrain + not occupied)
                // This requires the map oracle from GameEnv
                let map = match ctx.env.map() {
                    Some(m) => m,
                    None => {
                        // No map oracle available, can't validate - fail safely
                        return Status::Failure;
                    }
                };

                if !ctx.state.can_enter(map, destination) {
                    // Destination is blocked (wall or occupied), fail so we fall back to Wait
                    return Status::Failure;
                }

                // Destination is valid, create move action
                let move_action = MoveAction::new(ctx.entity, dir, 1);
                ctx.set_action(Action::character(
                    ctx.entity,
                    CharacterActionKind::Move(move_action),
                ));
                Status::Success
            }
            None => {
                // Already at player position (shouldn't happen)
                Status::Failure
            }
        }
    }
}

/// Attacks the player.
///
/// Generates an Attack action targeting the player. The attack will only
/// succeed if the player is adjacent (the game engine will validate this).
///
/// # Example
///
/// ```rust,ignore
/// use behavior_tree::Sequence;
/// use runtime::providers::ai::nodes::*;
///
/// // Attack if adjacent
/// Sequence::new(vec![
///     Box::new(IsAdjacentToPlayer),
///     Box::new(AttackPlayer),
/// ])
/// ```
pub struct AttackPlayer;

impl Behavior<AiContext<'_>> for AttackPlayer {
    fn tick(&self, ctx: &mut AiContext) -> Status {
        let player_id = ctx.state.entities.player.id;

        let attack = AttackAction::new(
            ctx.entity,
            player_id,
            AttackStyle::Melee, // Default to melee
        );

        ctx.set_action(Action::character(
            ctx.entity,
            CharacterActionKind::Attack(attack),
        ));

        Status::Success
    }
}

/// Waits (does nothing this turn).
///
/// Generates a Wait action, which consumes the entity's turn without
/// performing any action.
///
/// # Example
///
/// ```rust,ignore
/// use behavior_tree::Selector;
/// use runtime::providers::ai::nodes::*;
///
/// // Try various strategies, fallback to waiting
/// Selector::new(vec![
///     Box::new(AttackPlayer),
///     Box::new(MoveTowardPlayer),
///     Box::new(Wait),  // Fallback
/// ])
/// ```
pub struct Wait;

impl Behavior<AiContext<'_>> for Wait {
    fn tick(&self, ctx: &mut AiContext) -> Status {
        ctx.set_action(Action::character(ctx.entity, CharacterActionKind::Wait));
        Status::Success
    }
}

// ============================================================================
// Helper Functions
// ============================================================================

/// Calculates the best direction to move toward a target.
///
/// Uses Manhattan distance (L1 norm) to determine which axis brings us
/// closer to the target. This creates a simple "chase" behavior.
///
/// # Returns
///
/// - `Some(direction)` if there's a direction to move
/// - `None` if already at the target position
fn calculate_direction_toward(
    from_x: i32,
    from_y: i32,
    to_x: i32,
    to_y: i32,
) -> Option<CardinalDirection> {
    let dx = to_x - from_x;
    let dy = to_y - from_y;

    // Already at target
    if dx == 0 && dy == 0 {
        return None;
    }

    // Move along the axis with greater distance
    // This creates a simple "chase" behavior
    if dx.abs() > dy.abs() {
        if dx > 0 {
            Some(CardinalDirection::East)
        } else {
            Some(CardinalDirection::West)
        }
    } else if dy > 0 {
        Some(CardinalDirection::North)
    } else {
        Some(CardinalDirection::South)
    }
}
