//! Reusable utility functions for action scoring (Layer 3).
//!
//! This module provides common utility functions that can be used across
//! different tactic action scorers to avoid code duplication.
//!
//! # Categories
//!
//! - **Movement Analysis**: Check if movement goes towards/away from player
//! - **Distance Scoring**: Score actions based on ideal distance ranges
//! - **Attack Scoring**: Basic attack action evaluation
//!
//! # Design Principles
//!
//! - Pure functions (no side effects)
//! - Reusable across multiple tactics
//! - Clear, focused responsibility

use game_core::{CardinalDirection, MoveAction};

use crate::providers::ai::AiContext;

// ============================================================================
// Movement Direction Analysis
// ============================================================================

/// Checks if a move action brings the entity closer to the player.
///
/// # Arguments
///
/// * `move_action` - The move action to evaluate
/// * `ctx` - The AI context containing positions
///
/// # Returns
///
/// `true` if the move reduces distance to player, `false` otherwise.
pub fn move_towards_player(move_action: &MoveAction, ctx: &AiContext) -> bool {
    let new_pos = ctx.position_after_move(move_action.direction);
    let current_dist = ctx.distance_to_player();
    let new_dist = ctx.distance_from_to_player(new_pos);
    new_dist < current_dist
}

/// Checks if a move action increases distance from the player.
///
/// # Arguments
///
/// * `move_action` - The move action to evaluate
/// * `ctx` - The AI context containing positions
///
/// # Returns
///
/// `true` if the move increases distance to player, `false` otherwise.
pub fn move_away_from_player(move_action: &MoveAction, ctx: &AiContext) -> bool {
    let new_pos = ctx.position_after_move(move_action.direction);
    let current_dist = ctx.distance_to_player();
    let new_dist = ctx.distance_from_to_player(new_pos);
    new_dist > current_dist
}

/// Checks if a move action maintains approximately the same distance to player.
///
/// "Approximately" means within ±1 tile.
///
/// # Arguments
///
/// * `move_action` - The move action to evaluate
/// * `ctx` - The AI context containing positions
///
/// # Returns
///
/// `true` if the move maintains distance (±1 tile), `false` otherwise.
pub fn move_maintains_distance(move_action: &MoveAction, ctx: &AiContext) -> bool {
    let new_pos = ctx.position_after_move(move_action.direction);
    let current_dist = ctx.distance_to_player();
    let new_dist = ctx.distance_from_to_player(new_pos);
    new_dist.abs_diff(current_dist) <= 1
}

// ============================================================================
// Distance-based Scoring
// ============================================================================

/// Scores a move action based on how close it brings us to an ideal distance range.
///
/// # Arguments
///
/// * `move_action` - The move action to evaluate
/// * `ctx` - The AI context containing positions
/// * `ideal_min` - Minimum ideal distance (inclusive)
/// * `ideal_max` - Maximum ideal distance (inclusive)
///
/// # Returns
///
/// Score from 0-100:
/// - 100: Lands exactly in ideal range
/// - 80: Within 1 tile of ideal range
/// - 60: Within 2 tiles of ideal range
/// - 40: Within 3 tiles of ideal range
/// - 20: Within 4 tiles of ideal range
/// - 0: 5+ tiles away from ideal range
pub fn score_distance_to_ideal(
    move_action: &MoveAction,
    ctx: &AiContext,
    ideal_min: u32,
    ideal_max: u32,
) -> u32 {
    let new_pos = ctx.position_after_move(move_action.direction);
    let new_dist = ctx.distance_from_to_player(new_pos);

    // Perfect: Inside ideal range
    if new_dist >= ideal_min && new_dist <= ideal_max {
        return 100;
    }

    // Calculate distance from ideal range
    let distance_from_ideal = if new_dist < ideal_min {
        ideal_min - new_dist
    } else {
        new_dist - ideal_max
    };

    // Score degrades by 20 per tile away from ideal
    match distance_from_ideal {
        0 => 100, // Shouldn't happen (covered above), but defensive
        1 => 80,
        2 => 60,
        3 => 40,
        4 => 20,
        _ => 0,
    }
}

/// Scores a position's distance to player for attack purposes.
///
/// Closer is generally better for attacks, with optimal range at 0-2 tiles.
///
/// # Arguments
///
/// * `distance` - The distance to the player
///
/// # Returns
///
/// Score from 0-100:
/// - 100: Distance 0-1 (melee range)
/// - 90: Distance 2 (close)
/// - 70: Distance 3
/// - 50: Distance 4
/// - 30: Distance 5
/// - 20: Distance 6-7
/// - 10: Distance 8+
pub fn score_attack_by_distance(distance: u32) -> u32 {
    match distance {
        0..=1 => 100,
        2 => 90,
        3 => 70,
        4 => 50,
        5 => 30,
        6..=7 => 20,
        _ => 10,
    }
}

// ============================================================================
// Action Type Helpers
// ============================================================================

/// Extracts the direction from a move action if it's actually a move.
///
/// # Arguments
///
/// * `action` - The character action to check
///
/// # Returns
///
/// - `Some(CardinalDirection)` if the action is a Move
/// - `None` otherwise
pub fn extract_move_direction(
    action: &game_core::CharacterActionKind,
) -> Option<CardinalDirection> {
    match action {
        game_core::CharacterActionKind::Move(move_action) => Some(move_action.direction),
        _ => None,
    }
}

/// Checks if an action is an attack action.
///
/// # Arguments
///
/// * `action` - The character action to check
///
/// # Returns
///
/// `true` if the action is an Attack, `false` otherwise.
pub fn is_attack_action(action: &game_core::CharacterActionKind) -> bool {
    matches!(action, game_core::CharacterActionKind::Attack(_))
}

/// Checks if an action is a movement action.
///
/// # Arguments
///
/// * `action` - The character action to check
///
/// # Returns
///
/// `true` if the action is a Move, `false` otherwise.
pub fn is_move_action(action: &game_core::CharacterActionKind) -> bool {
    matches!(action, game_core::CharacterActionKind::Move(_))
}

/// Checks if an action is a Wait action.
///
/// # Arguments
///
/// * `action` - The character action to check
///
/// # Returns
///
/// `true` if the action is Wait, `false` otherwise.
pub fn is_wait_action(action: &game_core::CharacterActionKind) -> bool {
    matches!(action, game_core::CharacterActionKind::Wait(_))
}

// ============================================================================
// Situational Helpers
// ============================================================================

/// Calculates a simple threat level based on distance and HP.
///
/// Used for determining when to switch from offensive to defensive tactics.
///
/// # Arguments
///
/// * `ctx` - The AI context
///
/// # Returns
///
/// Threat level from 0-100:
/// - 100: Critical threat (enemy close + low HP)
/// - 70-90: High threat
/// - 40-60: Medium threat
/// - 10-30: Low threat
/// - 0: No threat
pub fn calculate_threat_level(ctx: &AiContext) -> u32 {
    let distance = ctx.distance_to_player();
    let hp = ctx.hp_ratio();

    // Threat increases as enemy gets closer and HP decreases
    let distance_threat = match distance {
        0..=1 => 50,
        2 => 40,
        3 => 30,
        4 => 20,
        5 => 10,
        _ => 0,
    };

    let hp_threat = match hp {
        0..=20 => 50,
        21..=40 => 40,
        41..=60 => 30,
        61..=80 => 20,
        _ => 10,
    };

    (distance_threat + hp_threat).min(100)
}
