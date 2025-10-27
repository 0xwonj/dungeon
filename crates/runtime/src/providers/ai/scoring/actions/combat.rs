//! Action scoring for combat tactics (Layer 3).
//!
//! This module implements action selection logic for all 5 combat tactics.
//! Each function scores individual actions based on how well they fulfill
//! the tactical intent.
//!
//! # Combat Tactics
//!
//! - **AggressiveMelee**: Always attack if possible, else close distance
//! - **DefensiveMelee**: Attack at 1-2 tiles, maintain escape route
//! - **Ranged**: Attack at 3-7 tiles, maintain medium distance
//! - **Kiting**: Attack at 4-6 tiles, retreat if too close
//! - **Ambush**: Placeholder (requires stealth system)

use game_core::CharacterActionKind;

use super::utilities;
use crate::providers::ai::AiContext;
use crate::providers::ai::scoring::Score;

// ============================================================================
// AggressiveMelee: Rush in and attack
// ============================================================================

/// Scores actions for AggressiveMelee tactic.
///
/// # Strategy
///
/// 1. **Attack always preferred** (if available): 100 situation
/// 2. **Move towards player**: 100 if closing gap, 20 if not
/// 3. **Wait**: Last resort (20 situation)
///
/// # Scoring Breakdown
///
/// - **Attack**: situation=100, personality=100, modifier=100 → value=100
/// - **Move(towards)**: situation=100, personality=80, modifier=100 → value=80
/// - **Move(away)**: situation=20, personality=80, modifier=100 → value=16
/// - **Wait**: situation=20, personality=50, modifier=100 → value=10
///
/// # Example
///
/// ```rust,ignore
/// let score = score_for_aggressive_melee(&CharacterActionKind::Attack(...), &ctx);
/// // Attack always scores 100
/// ```
pub fn score_for_aggressive_melee(action: &CharacterActionKind, ctx: &AiContext) -> Score {
    match action {
        CharacterActionKind::Attack(_) => {
            // Always attack if possible - highest priority
            Score::new(true, 100, 100, 100)
        }
        CharacterActionKind::Move(move_action) => {
            // Prefer moves that close distance
            let situation = if utilities::move_towards_player(move_action, ctx) {
                100 // Closing gap
            } else {
                20 // Not ideal, but still possible
            };

            Score::new(true, situation, 80, 100)
        }
        CharacterActionKind::Wait => {
            // Wait is last resort
            Score::new(true, 20, 50, 100)
        }
        _ => Score::impossible(),
    }
}

// ============================================================================
// DefensiveMelee: Cautious engagement
// ============================================================================

/// Scores actions for DefensiveMelee tactic.
///
/// # Strategy
///
/// 1. **Attack at 1-2 tiles**: High score in ideal range
/// 2. **Maintain 1-2 tile distance**: Move to stay in range
/// 3. **Retreat if too close** (distance 0): Create breathing room
/// 4. **Approach if too far** (distance 3+): Close to ideal range
///
/// # Scoring Breakdown
///
/// - **Attack (dist 1-2)**: situation=100, personality=90, modifier=100 → value=90
/// - **Attack (dist 0)**: situation=60, personality=90, modifier=100 → value=54
/// - **Attack (dist 3+)**: situation=40, personality=90, modifier=100 → value=36
/// - **Move(to ideal)**: situation=90-100, personality=70, modifier=100 → value=63-70
/// - **Wait**: situation=30, personality=60, modifier=100 → value=18
///
/// # Example
///
/// ```rust,ignore
/// let score = score_for_defensive_melee(&action, &ctx);
/// // Attack at distance 1-2 scores highest
/// ```
pub fn score_for_defensive_melee(action: &CharacterActionKind, ctx: &AiContext) -> Score {
    match action {
        CharacterActionKind::Attack(_) => {
            // Prefer attacking at 1-2 tile range
            let dist = ctx.distance_to_player();
            let situation = match dist {
                0 => 60,      // Too close, but still engage
                1..=2 => 100, // Ideal range
                3 => 70,      // A bit far
                4 => 40,      // Too far
                _ => 20,      // Way too far
            };

            Score::new(true, situation, 90, 100)
        }
        CharacterActionKind::Move(move_action) => {
            // Move to maintain 1-2 tile range
            let situation = utilities::score_distance_to_ideal(move_action, ctx, 1, 2);

            Score::new(true, situation, 70, 100)
        }
        CharacterActionKind::Wait => {
            // Wait is acceptable if already in good position
            Score::new(true, 30, 60, 100)
        }
        _ => Score::impossible(),
    }
}

// ============================================================================
// Ranged: Maintain medium distance
// ============================================================================

/// Scores actions for Ranged tactic.
///
/// # Strategy
///
/// 1. **Attack at 3-7 tiles**: Maximum score in ideal range
/// 2. **Maintain 3-7 tile distance**: Move to stay in range
/// 3. **Avoid melee range** (distance 0-2): Low attack priority
/// 4. **Close in if too far** (distance 8+): Get back in range
///
/// # Scoring Breakdown
///
/// - **Attack (dist 4-6)**: situation=100, personality=100, modifier=100 → value=100
/// - **Attack (dist 3,7)**: situation=90, personality=100, modifier=100 → value=90
/// - **Attack (dist 0-2)**: situation=20, personality=100, modifier=100 → value=20
/// - **Move(to ideal)**: situation=80-100, personality=50, modifier=100 → value=40-50
/// - **Wait**: situation=40, personality=60, modifier=100 → value=24
///
/// # Example
///
/// ```rust,ignore
/// let score = score_for_ranged(&action, &ctx);
/// // Attack at distance 4-6 scores 100
/// ```
pub fn score_for_ranged(action: &CharacterActionKind, ctx: &AiContext) -> Score {
    match action {
        CharacterActionKind::Attack(_) => {
            // Prefer attacking at medium range (3-7 tiles)
            let dist = ctx.distance_to_player();
            let situation = match dist {
                0..=2 => 20,  // Too close
                3 => 90,      // Good
                4..=6 => 100, // Optimal
                7 => 85,      // Still good
                8 => 50,      // Getting far
                _ => 20,      // Too far
            };

            Score::new(true, situation, 100, 100)
        }
        CharacterActionKind::Move(move_action) => {
            // Move to maintain 3-7 tile range
            let situation = utilities::score_distance_to_ideal(move_action, ctx, 3, 7);

            // Ranged units prioritize attacking over repositioning
            Score::new(true, situation, 50, 100)
        }
        CharacterActionKind::Wait => {
            // Wait if already in good position
            Score::new(true, 40, 60, 100)
        }
        _ => Score::impossible(),
    }
}

// ============================================================================
// Kiting: Hit and run
// ============================================================================

/// Scores actions for Kiting tactic.
///
/// # Strategy
///
/// 1. **Attack at 4-6 tiles**: Optimal kiting range
/// 2. **Retreat if too close** (distance <4): Create space
/// 3. **Close in if too far** (distance >6): Get back in range
/// 4. **Never attack at melee range** (distance 0-2): Retreat instead
///
/// # Scoring Breakdown
///
/// - **Attack (dist 4-6)**: situation=100, personality=100, modifier=100 → value=100
/// - **Attack (dist 3)**: situation=80, personality=100, modifier=100 → value=80
/// - **Attack (dist 0-2)**: situation=20, personality=100, modifier=100 → value=20
/// - **Move(away, dist <4)**: situation=100, personality=90, modifier=100 → value=90
/// - **Move(to ideal)**: situation=80-100, personality=80, modifier=100 → value=64-80
/// - **Wait**: situation=20, personality=40, modifier=100 → value=8
///
/// # Example
///
/// ```rust,ignore
/// let score = score_for_kiting(&action, &ctx);
/// // At distance 2: Move(away) scores 90, Attack scores only 20
/// ```
pub fn score_for_kiting(action: &CharacterActionKind, ctx: &AiContext) -> Score {
    let current_dist = ctx.distance_to_player();

    match action {
        CharacterActionKind::Attack(_) => {
            // Only attack when in ideal kiting range (4-6 tiles)
            let situation = match current_dist {
                0..=2 => 20,  // Too close - should be retreating!
                3 => 80,      // Acceptable
                4..=6 => 100, // Perfect
                7 => 70,      // A bit far
                _ => 30,      // Too far
            };

            Score::new(true, situation, 100, 100)
        }
        CharacterActionKind::Move(move_action) => {
            // Prioritize maintaining 4-6 tile range
            // Special: If too close, strongly prefer moves that increase distance
            if current_dist < 4 && utilities::move_away_from_player(move_action, ctx) {
                // Retreating when too close - high priority
                Score::new(true, 100, 90, 100)
            } else {
                // Normal: Move to maintain ideal range
                let situation = utilities::score_distance_to_ideal(move_action, ctx, 4, 6);
                Score::new(true, situation, 80, 100)
            }
        }
        CharacterActionKind::Wait => {
            // Kiting should rarely wait
            Score::new(true, 20, 40, 100)
        }
        _ => Score::impossible(),
    }
}

// ============================================================================
// Ambush: Placeholder (requires stealth)
// ============================================================================

/// Scores actions for Ambush tactic.
///
/// # Current Status
///
/// **PLACEHOLDER**: Always returns impossible because stealth system
/// is not yet implemented in game-core.
///
/// # Future Implementation
///
/// When stealth is added:
/// 1. **Wait if hidden**: High score (preserve stealth)
/// 2. **Attack if in backstab position**: Maximum damage
/// 3. **Move to better position**: Only if undetected
/// 4. **Never attack from poor position**: Lose surprise advantage
pub fn score_for_ambush(_action: &CharacterActionKind, _ctx: &AiContext) -> Score {
    // TODO: Implement when stealth system is added
    Score::impossible()
}
