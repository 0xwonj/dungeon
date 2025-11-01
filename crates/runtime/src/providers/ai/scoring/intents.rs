//! Intent scoring functions (Layer 1).
//!
//! This module provides scoring functions for high-level strategic intents.
//! Each function evaluates how desirable a particular intent is for an NPC
//! based on the current game state and the NPC's personality traits.
//!
//! # Structured Scoring
//!
//! All intent scoring functions return [`Score`] from the parent module.
//! See [`super::Score`] for details on the scoring structure.

use game_content::traits::TraitKind;

use super::Score;
use crate::providers::ai::AiContext;

// ============================================================================
// Intent Scoring Functions
// ============================================================================

/// Combat intent scoring.
///
/// Selected when the NPC wants to engage enemies.
///
/// # Factors
///
/// - **is_possible**: Can see enemies (will move toward them if not adjacent)
/// - **situation**: Distance to enemies (closer = higher)
/// - **personality**: Aggression (70%) + Bravery (30%)
/// - **modifier**: HP penalty if below 30%
///
/// # Returns
///
/// [`Score`] with all components. Use `.value()` for final value.
///
/// # Examples
///
/// ```text
/// Brave Orc (Aggression=180, Bravery=150) at distance 2, HP 80%:
///   is_possible=true, situation=90, personality=70, modifier=100
///   value() = 90 × 70 × 100 / 10000 = 63
///
/// Cowardly Goblin (Aggression=60, Bravery=40) at distance 2, HP 25%:
///   is_possible=true, situation=90, personality=27, modifier=50
///   score() = 90 × 27 × 50 / 10000 = 12
/// ```
pub fn combat(ctx: &AiContext) -> Score {
    // Is it possible? Only need to see the enemy
    // Combat tactics will decide whether to attack or move closer
    if !ctx.can_see_player() {
        return Score::impossible();
    }

    // Situation: Distance to player (closer = more favorable for combat)
    let distance = ctx.distance_to_player();
    let situation = match distance {
        0..=1 => 100, // Adjacent - perfect
        2 => 90,      // Very close
        3 => 80,      // Close
        4 => 60,      // Medium
        5 => 40,      // Far
        6 => 20,      // Very far
        _ => 5,       // Too far
    };

    // Personality: Aggression (primary) + Bravery (secondary)
    let personality = if let Some(profile) = ctx.trait_profile() {
        let aggression = profile.get(TraitKind::Aggression) as u32;
        let bravery = profile.get(TraitKind::Bravery) as u32;

        // Aggression is 70%, Bravery is 30%
        // Scale from 0-240 to 0-100
        let aggr_score = (aggression * 70) / 240;
        let brave_score = (bravery * 30) / 240;

        aggr_score + brave_score
    } else {
        50 // Default moderate combat preference
    };

    // HP Modifier: Low HP reduces combat intent
    let hp = ctx.hp_ratio();
    let modifier = if hp < 30 {
        50 // Severely wounded - half combat intent
    } else if hp < 60 {
        80 // Wounded - reduced intent
    } else {
        100 // Healthy - full intent
    };

    Score::new(true, situation, personality, modifier)
}

/// Survival intent scoring.
///
/// Selected when the NPC wants to preserve their life.
///
/// # Factors
///
/// - **is_possible**: Always possible (if movement available)
/// - **situation**: Low HP = high score
/// - **personality**: Inverse of Bravery (cowardice)
/// - **modifier**: Enemy visibility (sees enemy = higher urgency)
///
/// # Returns
///
/// [`Score`] with all components. Use `.value()` for final value.
///
/// # Examples
///
/// ```text
/// Cowardly Goblin (Bravery=40) at HP 20%, enemy visible:
///   is_possible=true, situation=100, personality=83, modifier=120
///   score() = 100 × 83 × 120 / 10000 = 99
///
/// Brave Orc (Bravery=150) at HP 70%, no enemy:
///   is_possible=true, situation=30, personality=37, modifier=80
///   score() = 30 × 37 × 80 / 10000 = 8
/// ```
pub fn survival(ctx: &AiContext) -> Score {
    // Always possible if we can move (even Wait is survival)
    // But prefer movement for actual fleeing
    let is_possible = ctx.has_movement_actions() || ctx.can_wait();
    if !is_possible {
        return Score::impossible();
    }

    // Situation: Low HP = desperate need for survival
    let hp = ctx.hp_ratio();
    let situation = if hp < 20 {
        100 // Critical
    } else if hp < 40 {
        80 // Desperate
    } else if hp < 60 {
        50 // Wounded
    } else if hp < 80 {
        20 // Slightly hurt
    } else {
        5 // Healthy - low survival priority
    };

    // Personality: Inverse of Bravery (cowards flee more easily)
    let personality = if let Some(profile) = ctx.trait_profile() {
        let bravery = profile.get(TraitKind::Bravery) as u32;

        // Invert bravery: 240 → 0, 0 → 100
        100 - (bravery * 100 / 240)
    } else {
        50 // Default moderate cowardice
    };

    // Enemy Modifier: Seeing enemies increases survival urgency
    let modifier = if ctx.can_see_player() {
        120 // Enemy present - flee!
    } else {
        80 // No immediate threat - lower urgency
    };

    Score::new(is_possible, situation, personality, modifier)
}

/// Exploration intent scoring.
///
/// Selected when the NPC wants to investigate surroundings.
///
/// # Factors
///
/// - **is_possible**: No immediate threats (can't see enemies)
/// - **situation**: Safe environment (high HP, no combat)
/// - **personality**: Curiosity
/// - **modifier**: Has movement actions available
///
/// # Returns
///
/// [`Score`] with all components. Use `.value()` for final value.
///
/// # Examples
///
/// ```text
/// Curious Goblin (Curiosity=180) at HP 90%, no enemies:
///   is_possible=true, situation=90, personality=75, modifier=100
///   score() = 90 × 75 × 100 / 10000 = 67
///
/// Incurious Orc (Curiosity=30) at HP 90%, no enemies:
///   is_possible=true, situation=90, personality=12, modifier=100
///   score() = 90 × 12 × 100 / 10000 = 10
/// ```
pub fn exploration(ctx: &AiContext) -> Score {
    // Only explore when safe (no visible enemies)
    if ctx.can_see_player() {
        return Score::impossible();
    }

    // Situation: Safe and healthy = good time to explore
    let hp = ctx.hp_ratio();
    let situation = if hp >= 80 {
        90 // Healthy and safe
    } else if hp >= 60 {
        60 // Okay but cautious
    } else {
        20 // Wounded - should rest/heal instead
    };

    // Personality: Curiosity drives exploration
    let personality = if let Some(profile) = ctx.trait_profile() {
        let curiosity = profile.get(TraitKind::Curiosity) as u32;

        // Scale from 0-240 to 0-100
        (curiosity * 100) / 240
    } else {
        50 // Default moderate curiosity
    };

    // Movement Modifier: Need to be able to move to explore
    let modifier = if ctx.has_movement_actions() {
        100 // Can explore
    } else {
        30 // Limited exploration (just look around)
    };

    Score::new(true, situation, personality, modifier)
}

/// Social intent scoring.
///
/// Selected when the NPC wants to help allies.
///
/// # Factors
///
/// - **is_possible**: Allies present nearby
/// - **situation**: Ally needs help (low HP)
/// - **personality**: Loyalty + Empathy
/// - **modifier**: Own HP (can't help if dying)
///
/// # Returns
///
/// [`Score`] with all components. Use `.value()` for final value.
///
/// # Note
///
/// Currently returns impossible score as ally system is not yet implemented.
/// Will be enabled when EntitiesState supports ally iteration.
pub fn social(ctx: &AiContext) -> Score {
    // TODO: Implement when ally system is available
    // Need EntitiesState to provide:
    // - all_actors() iterator
    // - Faction/alliance system
    // - Ally HP checking

    let ally_count = ctx.count_nearby_allies(5);
    if ally_count == 0 {
        return Score::impossible();
    }

    // Placeholder: Calculate personality but return impossible
    let _personality = if let Some(profile) = ctx.trait_profile() {
        let loyalty = profile.get(TraitKind::Loyalty) as u32;
        let empathy = profile.get(TraitKind::Empathy) as u32;

        // Loyalty 60%, Empathy 40%
        (loyalty * 60 + empathy * 40) / 240
    } else {
        50
    };

    // Return impossible for now - will implement properly when allies exist
    Score::impossible()
}

/// Resource intent scoring.
///
/// Selected when the NPC wants to acquire or guard loot.
///
/// # Factors
///
/// - **is_possible**: Loot visible or treasure to guard
/// - **situation**: Distance to loot, safety
/// - **personality**: Greed + Territoriality
/// - **modifier**: Combat ongoing (won't loot during fight)
///
/// # Returns
///
/// [`Score`] with all components. Use `.value()` for final value.
///
/// # Note
///
/// Currently returns impossible score as item/loot system is not yet implemented.
/// Will be enabled when world items are queryable from GameState.
pub fn resource(ctx: &AiContext) -> Score {
    // TODO: Implement when loot system is available
    // Need GameState to provide:
    // - entities.items visibility
    // - Item value/rarity information
    // - Treasure location data

    // Don't loot during combat
    if ctx.can_see_player() {
        return Score::impossible();
    }

    // Placeholder personality calculation
    let _personality = if let Some(profile) = ctx.trait_profile() {
        let greed = profile.get(TraitKind::Greed) as u32;
        let territoriality = profile.get(TraitKind::Territoriality) as u32;

        // Greed 70%, Territoriality 30%
        (greed * 70 + territoriality * 30) / 240
    } else {
        50
    };

    // Return impossible for now - will implement when loot is queryable
    Score::impossible()
}

/// Idle intent scoring.
///
/// Default fallback when no other intent applies.
///
/// # Returns
///
/// [`Score`] with constant low score (10).
///
/// This ensures NPCs always have a valid intent to fall back to,
/// even when all other intents score 0.
pub fn idle(_ctx: &AiContext) -> Score {
    // Always possible, constant low score
    Score::new(
        true, // is_possible
        100,  // situation (always neutral)
        10,   // personality (low preference)
        100,  // modifier (no adjustment)
    )
    // value() = 100 × 10 × 100 / 10000 = 10
}
