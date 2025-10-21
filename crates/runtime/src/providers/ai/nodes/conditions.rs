//! Condition nodes for AI behavior trees.
//!
//! Condition nodes check the game state and return Success or Failure.
//! They should not modify state or generate actions.

use behavior_tree::{Behavior, Status};
use game_core::{Position, StatsSnapshot};

use crate::providers::ai::AiContext;

/// Checks if the player is adjacent to this entity (Manhattan distance == 1).
///
/// # Example
///
/// ```rust,ignore
/// use behavior_tree::Sequence;
/// use runtime::providers::ai::nodes::*;
///
/// // Attack if player is adjacent
/// Sequence::new(vec![
///     Box::new(IsAdjacentToPlayer),
///     Box::new(AttackPlayer),
/// ])
/// ```
pub struct IsAdjacentToPlayer;

impl Behavior<AiContext<'_>> for IsAdjacentToPlayer {
    fn tick(&self, ctx: &mut AiContext) -> Status {
        let actor = match ctx.state.entities.actor(ctx.entity) {
            Some(a) => a,
            None => return Status::Failure,
        };

        let player_pos = ctx.state.entities.player.position;
        let distance = manhattan_distance(actor.position, player_pos);

        if distance == 1 {
            Status::Success
        } else {
            Status::Failure
        }
    }
}

/// Checks if this entity's health is below a threshold.
///
/// # Example
///
/// ```rust,ignore
/// // Flee if health is below 30%
/// Sequence::new(vec![
///     Box::new(IsHealthLow { threshold: 0.3 }),
///     Box::new(FleeFromPlayer { distance: 5 }),
/// ])
/// ```
pub struct IsHealthLow {
    /// Health ratio threshold (0.0 to 1.0).
    ///
    /// Returns Success if `current_hp / max_hp < threshold`.
    pub threshold: f32,
}

impl Behavior<AiContext<'_>> for IsHealthLow {
    fn tick(&self, ctx: &mut AiContext) -> Status {
        let actor = match ctx.state.entities.actor(ctx.entity) {
            Some(a) => a,
            None => return Status::Failure,
        };

        // Use StatsSnapshot to get accurate HP maximum including bonuses
        let snapshot = StatsSnapshot::create(&actor.core_stats, &actor.bonuses, &actor.resources);
        let (current, maximum) = snapshot.hp();

        if maximum == 0 {
            return Status::Failure;
        }

        let ratio = current as f32 / maximum as f32;

        if ratio < self.threshold {
            Status::Success
        } else {
            Status::Failure
        }
    }
}

/// Checks if the player is visible within a certain range.
///
/// This is a simple distance check. Future enhancements could add
/// line-of-sight validation.
///
/// # Example
///
/// ```rust,ignore
/// // Chase player if visible
/// Sequence::new(vec![
///     Box::new(IsPlayerVisible { range: 6 }),
///     Box::new(MoveTowardPlayer),
/// ])
/// ```
pub struct IsPlayerVisible {
    /// Maximum vision range (Manhattan distance).
    pub range: i32,
}

impl Behavior<AiContext<'_>> for IsPlayerVisible {
    fn tick(&self, ctx: &mut AiContext) -> Status {
        let actor = match ctx.state.entities.actor(ctx.entity) {
            Some(a) => a,
            None => return Status::Failure,
        };

        let player_pos = ctx.state.entities.player.position;
        let distance = manhattan_distance(actor.position, player_pos);

        if distance <= self.range {
            Status::Success
        } else {
            Status::Failure
        }
    }
}

// ============================================================================
// Helper Functions
// ============================================================================

/// Calculates Manhattan distance between two positions.
///
/// Manhattan distance is the sum of absolute differences in coordinates:
/// `|x1 - x2| + |y1 - y2|`
///
/// This is appropriate for grid-based movement where diagonal moves are
/// not allowed or cost the same as cardinal moves.
#[inline]
fn manhattan_distance(a: Position, b: Position) -> i32 {
    (a.x - b.x).abs() + (a.y - b.y).abs()
}
