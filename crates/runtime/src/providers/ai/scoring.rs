//! Goal scoring functions for evaluating action candidates.
//!
//! Each goal has its own scoring function that evaluates how well
//! an action+input combination serves that goal.
//!
//! All scoring functions are pure and return a score from 0-100.

use game_core::{ActionInput, ActionKind, CardinalDirection, EntityId, Position};

use super::AiContext;

/// Scores actions for the Attack goal.
pub fn score_for_attack(
    kind: ActionKind,
    input: &ActionInput,
    target: EntityId,
    ctx: &AiContext,
) -> u32 {
    let profile = match ctx.env.actions() {
        Ok(actions) => actions.action_profile(kind),
        Err(_) => return 0,
    };

    // Attack actions get highest priority
    if profile.tags.contains(&game_core::ActionTag::Attack) {
        // Check if input targets the right entity/direction
        match input {
            ActionInput::Target(id) if *id == target => 100, // Perfect match
            ActionInput::Direction(dir) => {
                // Check if direction points towards target
                if let Some(my_pos) = ctx.my_position() {
                    if let Some(target_dir) = direction_to_entity(my_pos, target, ctx) {
                        if target_dir == *dir {
                            95 // Attack in right direction
                        } else {
                            30 // Attack but wrong direction
                        }
                    } else {
                        50 // Can't determine direction
                    }
                } else {
                    10 // No position
                }
            }
            _ => 50, // Attack action but unclear targeting
        }
    }
    // Movement to close distance
    else if profile.tags.contains(&game_core::ActionTag::Movement) {
        if let ActionInput::Direction(dir) = input {
            let Some(my_pos) = ctx.my_position() else {
                return 10; // No position
            };
            let target_pos = match ctx.state.entities.actor(target) {
                Some(actor) => match actor.position {
                    Some(pos) => pos,
                    None => return 10,
                },
                None => return 10,
            };

            let (dx, dy) = dir.offset();
            let new_pos = Position::new(my_pos.x + dx, my_pos.y + dy);

            let current_dist = my_pos.manhattan_distance(target_pos);
            let new_dist = new_pos.manhattan_distance(target_pos);

            // Prefer moving closer to target
            if new_dist < current_dist {
                70 // Good: approaching target
            } else if new_dist == current_dist {
                30 // Neutral: circling
            } else {
                10 // Bad: moving away
            }
        } else {
            20
        }
    } else {
        10 // Other actions (Wait, etc.) are low priority for attacking
    }
}

/// Scores actions for the FleeFrom goal.
pub fn score_for_flee(
    kind: ActionKind,
    input: &ActionInput,
    threat: EntityId,
    ctx: &AiContext,
) -> u32 {
    let profile = match ctx.env.actions() {
        Ok(actions) => actions.action_profile(kind),
        Err(_) => return 0,
    };

    // Movement is highest priority for fleeing
    if profile.tags.contains(&game_core::ActionTag::Movement) {
        if let ActionInput::Direction(dir) = input {
            let Some(my_pos) = ctx.my_position() else {
                return 10; // No position
            };
            let threat_pos = match ctx.state.entities.actor(threat) {
                Some(actor) => match actor.position {
                    Some(pos) => pos,
                    None => return 10,
                },
                None => return 10,
            };

            let (dx, dy) = dir.offset();
            let new_pos = Position::new(my_pos.x + dx, my_pos.y + dy);

            let current_dist = my_pos.manhattan_distance(threat_pos);
            let new_dist = new_pos.manhattan_distance(threat_pos);

            // Prefer moving away from threat
            if new_dist > current_dist {
                100 // Perfect: fleeing successfully
            } else if new_dist == current_dist {
                30 // Neutral: circling
            } else {
                0 // Bad: moving towards threat
            }
        } else {
            20
        }
    }
    // Attacking while fleeing is low priority (but not zero - might need to fight through)
    else if profile.tags.contains(&game_core::ActionTag::Attack) {
        10
    } else {
        20 // Wait
    }
}

/// Scores actions for the HealSelf goal.
///
/// NOTE: Currently disabled - requires Heal and UseItem ActionKinds to be implemented.
#[allow(dead_code)]
pub fn score_for_heal_self(_kind: ActionKind, _input: &ActionInput, _ctx: &AiContext) -> u32 {
    // TODO: Re-enable when Heal and UseItem are implemented
    // if kind == ActionKind::Heal {
    //     100
    // } else if kind == ActionKind::UseItem {
    //     // Check if item is actually a healing item
    //     80
    // } else {
    //     10
    // }
    10 // Default low priority for all actions
}

/// Scores actions for the Idle goal.
pub fn score_for_idle(kind: ActionKind, _input: &ActionInput, _ctx: &AiContext) -> u32 {
    // Wait is best for idling
    if kind == ActionKind::Wait {
        100
    } else {
        30 // Other actions are acceptable but not preferred
    }
}

/// Scores actions for the MoveTo goal.
pub fn score_for_move_to(
    kind: ActionKind,
    input: &ActionInput,
    target_pos: Position,
    ctx: &AiContext,
) -> u32 {
    let profile = match ctx.env.actions() {
        Ok(actions) => actions.action_profile(kind),
        Err(_) => return 0,
    };

    if profile.tags.contains(&game_core::ActionTag::Movement) {
        if let ActionInput::Direction(dir) = input {
            let Some(my_pos) = ctx.my_position() else {
                return 0; // No position
            };
            let (dx, dy) = dir.offset();
            let new_pos = Position::new(my_pos.x + dx, my_pos.y + dy);

            let current_dist = my_pos.manhattan_distance(target_pos);
            let new_dist = new_pos.manhattan_distance(target_pos);

            if new_dist < current_dist {
                100 // Moving closer
            } else if new_dist == current_dist {
                50 // Circling
            } else {
                10 // Moving away
            }
        } else {
            20
        }
    } else {
        10
    }
}

/// Scores actions for the ProtectAlly goal.
pub fn score_for_protect_ally(
    kind: ActionKind,
    input: &ActionInput,
    ally: EntityId,
    ctx: &AiContext,
) -> u32 {
    let profile = match ctx.env.actions() {
        Ok(actions) => actions.action_profile(kind),
        Err(_) => return 0,
    };

    // TODO: Re-enable when Heal ActionKind is implemented
    // // Healing the ally
    // if kind == ActionKind::Heal {
    //     match input {
    //         ActionInput::Target(id) if *id == ally => 100,
    //         _ => 50,
    //     }
    // }
    // Moving towards ally
    if profile.tags.contains(&game_core::ActionTag::Movement) {
        if let ActionInput::Direction(dir) = input {
            let Some(my_pos) = ctx.my_position() else {
                return 10; // No position
            };
            let ally_pos = match ctx.state.entities.actor(ally) {
                Some(actor) => match actor.position {
                    Some(pos) => pos,
                    None => return 10,
                },
                None => return 10,
            };

            let (dx, dy) = dir.offset();
            let new_pos = Position::new(my_pos.x + dx, my_pos.y + dy);

            let current_dist = my_pos.manhattan_distance(ally_pos);
            let new_dist = new_pos.manhattan_distance(ally_pos);

            // Stay close to ally (ideal range: 1-2 tiles)
            if (1..=2).contains(&new_dist) {
                100 // Perfect distance
            } else if new_dist < current_dist {
                70 // Moving closer
            } else {
                30 // Moving away
            }
        } else {
            20
        }
    } else {
        20
    }
}

// ============================================================================
// Helper Functions
// ============================================================================

/// Calculates the cardinal direction from one position to an entity.
pub fn direction_to_entity(
    from: Position,
    target: EntityId,
    ctx: &AiContext,
) -> Option<CardinalDirection> {
    let target_pos = ctx.state.actor_position(target)?;
    let dx = target_pos.x - from.x;
    let dy = target_pos.y - from.y;

    // Find the closest cardinal direction
    use CardinalDirection::*;
    match (dx.signum(), dy.signum()) {
        (0, -1) => Some(North),
        (0, 1) => Some(South),
        (1, 0) => Some(East),
        (-1, 0) => Some(West),
        (1, -1) => Some(NorthEast),
        (-1, -1) => Some(NorthWest),
        (1, 1) => Some(SouthEast),
        (-1, 1) => Some(SouthWest),
        (0, 0) => None, // Already at target
        _ => None,      // Shouldn't happen
    }
}
