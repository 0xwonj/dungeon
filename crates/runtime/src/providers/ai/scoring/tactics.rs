//! Tactic scoring functions (Layer 2).
//!
//! This module implements scoring for all 22 tactical approaches across 6 intent categories.
//! Each tactic evaluates whether it's feasible, how well it fits the current situation,
//! and how well it aligns with the NPC's personality traits.
//!
//! # Architecture
//!
//! ## Tactic Categories
//!
//! - **Combat (5)**: AggressiveMelee, DefensiveMelee, Ranged, Kiting, Ambush
//! - **Survival (4)**: Flee, Retreat, SeekCover, UseSurvivalItem
//! - **Exploration (3)**: Patrol, Investigate, Search
//! - **Social (3)**: HealAlly, BuffAlly, CoordinateAttack
//! - **Resource (2)**: Loot, GuardTreasure
//! - **Idle (2)**: Wait, Wander
//!
//! ## Scoring Pattern
//!
//! All tactics follow the same structured scoring approach:
//!
//! ```rust,ignore
//! pub fn tactic_name(ctx: &AiContext) -> Score {
//!     // 1. Feasibility check
//!     if !required_actions_available(ctx) {
//!         return Score::impossible();
//!     }
//!
//!     // 2. Situational favorability (0-100)
//!     let situation = compute_situation_score(ctx);
//!
//!     // 3. Personality alignment (0-100)
//!     let personality = compute_personality_score(ctx);
//!
//!     // 4. Contextual modifiers (0-200, typically 100)
//!     let modifier = compute_modifier(ctx);
//!
//!     Score::new(true, situation, personality, modifier)
//! }
//! ```
//!
//! # Combat Tactics Design
//!
//! Combat tactics are differentiated along three axes:
//!
//! ## 1. Range Preference
//!
//! Each tactic has an optimal distance range from the target:
//!
//! - **AggressiveMelee**: 0-1 tiles (face-to-face)
//! - **DefensiveMelee**: 1-2 tiles (close but cautious)
//! - **Ranged**: 3-7 tiles (medium range)
//! - **Kiting**: 4-6 tiles (maintain distance)
//! - **Ambush**: Situation-dependent (hidden position)
//!
//! ## 2. Risk Tolerance
//!
//! How much HP and safety margin each tactic requires:
//!
//! - **AggressiveMelee**: High risk (HP > 40%)
//! - **DefensiveMelee**: Medium risk (HP > 70%)
//! - **Ranged**: Low risk (prefer HP > 50%)
//! - **Kiting**: Very low risk (HP > 30%)
//! - **Ambush**: Calculated risk (full HP preferred)
//!
//! ## 3. Required Capabilities
//!
//! What actions must be available:
//!
//! - **AggressiveMelee**: Attack actions only
//! - **DefensiveMelee**: Attack + Movement (for retreat)
//! - **Ranged**: Ranged attack (TODO: distinguish from melee)
//! - **Kiting**: Ranged attack + Movement (both required)
//! - **Ambush**: Stealth abilities (not yet implemented)
//!
//! # Trait Usage
//!
//! Combat tactics consider these personality traits:
//!
//! - **Aggression**: Willingness to engage in combat
//! - **Bravery**: Tolerance for danger and low HP
//! - **Caution**: Preference for safe, defensive approaches
//! - **PreferredRange**: 0 (melee) to 240 (ranged)
//! - **TacticalSense**: Understanding of positioning and timing
//! - **Honor**: Aversion to underhanded tactics (affects Ambush)
//!
//! # Implementation Status
//!
//! - ✅ **Combat (5)**: Fully implemented with detailed logic
//! - ⏸️ **Survival (4)**: Placeholder (returns impossible)
//! - ⏸️ **Exploration (3)**: Placeholder (returns impossible)
//! - ⏸️ **Social (3)**: Placeholder (returns impossible)
//! - ⏸️ **Resource (2)**: Placeholder (returns impossible)
//! - ⏸️ **Idle (2)**: Placeholder (returns impossible)

use game_content::traits::TraitKind;

use super::Score;
use crate::providers::ai::AiContext;

// ============================================================================
// Combat Tactics (Fully Implemented)
// ============================================================================

/// Rush into melee range and overwhelm with aggression.
///
/// This tactic handles both:
/// - **Closing distance**: Move toward enemy when far
/// - **Attacking**: Strike when adjacent
///
/// # Optimal Conditions
///
/// - **Distance**: 0-2 tiles (closer is better)
/// - **HP**: > 40% (can take hits)
/// - **Traits**: High Aggression (>150), High Bravery (>120)
///
/// # Situational Scoring
///
/// - Distance 0-1: 100 (perfect - can attack)
/// - Distance 2: 90 (very close - can reach next turn)
/// - Distance 3: 70
/// - Distance 4: 40
/// - Distance 5+: 20 (poor, but possible if very aggressive)
///
/// # Personality Weighting
///
/// - Aggression: 60%
/// - Bravery: 30%
/// - Anti-Caution: 10%
///
/// # Modifiers
///
/// - HP < 40%: 50 (risky)
/// - HP < 70%: 80
/// - HP >= 70%: 100
///
/// # Example
///
/// ```rust,ignore
/// // High-aggression berserker at close range with 80% HP
/// let score = aggressive_melee(&ctx);
/// // situation = 100 (distance 1)
/// // personality = 90 (aggression 200, bravery 150, caution 50)
/// // modifier = 100 (HP 80%)
/// // value = (100 * 90 * 100) / 10000 = 90
/// ```
pub fn aggressive_melee(ctx: &AiContext) -> Score {
    // Feasibility: Need attack actions (if adjacent) OR movement actions (if far)
    let has_capabilities = ctx.has_attack_actions() || ctx.has_movement_actions();
    if !has_capabilities {
        return Score::impossible();
    }

    // Situation: Closer is better
    let distance = ctx.distance_to_player();
    let situation = match distance {
        0..=1 => 100,
        2 => 90,
        3 => 70,
        4 => 40,
        5 => 20,
        _ => 10,
    };

    // Personality: High aggression + bravery, low caution
    let personality = if let Some(profile) = ctx.trait_profile() {
        let aggression = profile.get(TraitKind::Aggression) as u32;
        let bravery = profile.get(TraitKind::Bravery) as u32;
        let caution = profile.get(TraitKind::Caution) as u32;

        // Aggression 60%, Bravery 30%, Anti-Caution 10%
        // Max: (240*60 + 240*30 + 240*10) / 240 = 100
        (aggression * 60 + bravery * 30 + (240 - caution) * 10) / 24000
    } else {
        50
    };

    // Modifier: HP threshold (need health to tank)
    let hp = ctx.hp_ratio();
    let modifier = if hp < 40 {
        50 // Too risky
    } else if hp < 70 {
        80
    } else {
        100
    };

    Score::new(true, situation, personality, modifier)
}

/// Engage in melee with caution and defensive positioning.
///
/// This tactic handles both:
/// - **Closing distance**: Move toward enemy when far (cautiously)
/// - **Attacking**: Strike when at ideal range (1-2 tiles)
///
/// # Optimal Conditions
///
/// - **Distance**: 1-3 tiles (maintain escape route)
/// - **HP**: > 70% (need buffer)
/// - **Traits**: Medium Bravery (80-150), High Caution (>120)
///
/// # Situational Scoring
///
/// - Distance 1-2: 100 (ideal range - can attack or retreat)
/// - Distance 3: 90 (close but safe)
/// - Distance 0: 70 (too close, no room to retreat)
/// - Distance 4: 50
/// - Distance 5+: 20
///
/// # Personality Weighting
///
/// - Caution: 50%
/// - Bravery: 30%
/// - Discipline: 20%
///
/// # Modifiers
///
/// - HP < 70%: 60 (need high HP for defensive)
/// - HP < 85%: 90
/// - HP >= 85%: 110 (bonus for full health)
/// - Has movement actions: +10 modifier (escape route available)
///
/// # Example
///
/// ```rust,ignore
/// // Cautious guard at distance 2 with 90% HP and movement options
/// let score = defensive_melee(&ctx);
/// // situation = 100 (distance 2)
/// // personality = 75 (caution 180, bravery 120, discipline 100)
/// // modifier = 110 + 10 = 120 (high HP + movement)
/// // value = (100 * 75 * 120) / 10000 = 90
/// ```
pub fn defensive_melee(ctx: &AiContext) -> Score {
    // Feasibility: Need attack actions (if adjacent) OR movement actions (if far)
    let has_capabilities = ctx.has_attack_actions() || ctx.has_movement_actions();
    if !has_capabilities {
        return Score::impossible();
    }

    // Situation: Prefer 1-3 tiles (close but with escape room)
    let distance = ctx.distance_to_player();
    let situation = match distance {
        0 => 70,      // Too close, boxed in
        1..=2 => 100, // Ideal
        3 => 90,
        4 => 50,
        5 => 20,
        _ => 10,
    };

    // Personality: High caution + moderate bravery + discipline
    let personality = if let Some(profile) = ctx.trait_profile() {
        let caution = profile.get(TraitKind::Caution) as u32;
        let bravery = profile.get(TraitKind::Bravery) as u32;
        let discipline = profile.get(TraitKind::Discipline) as u32;

        // Caution 50%, Bravery 30%, Discipline 20%
        (caution * 50 + bravery * 30 + discipline * 20) / 24000
    } else {
        50
    };

    // Modifier: HP threshold (defensive requires buffer) + movement bonus
    let hp = ctx.hp_ratio();
    let mut modifier = if hp < 70 {
        60 // Not safe enough
    } else if hp < 85 {
        90
    } else {
        110 // Bonus for high HP
    };

    // Bonus if movement available (can retreat if needed)
    if ctx.has_movement_actions() {
        modifier += 10;
    }

    Score::new(true, situation, personality, modifier)
}

/// Attack from range while maintaining optimal distance.
///
/// # Optimal Conditions
///
/// - **Distance**: 3-7 tiles (medium range)
/// - **HP**: > 30% (less risky than melee)
/// - **Traits**: High PreferredRange (>150), Medium TacticalSense (>80)
///
/// # Situational Scoring
///
/// - Distance 4-6: 100 (perfect range)
/// - Distance 3: 90
/// - Distance 7: 85
/// - Distance 2: 60
/// - Distance 8: 50
/// - Distance 0-1: 20 (too close for ranged)
/// - Distance 9+: 10 (too far)
///
/// # Personality Weighting
///
/// - PreferredRange: 70%
/// - TacticalSense: 20%
/// - Anti-Aggression: 10%
///
/// # Modifiers
///
/// - HP < 30%: 90 (ranged is safer)
/// - HP < 60%: 100
/// - HP >= 60%: 105
///
/// # TODO
///
/// Currently cannot distinguish ranged attacks from melee attacks.
/// For now, assumes any attack action can be used at range.
/// Future: Add `ActionKind::RangedAttack` distinction.
///
/// # Example
///
/// ```rust,ignore
/// // Archer at distance 5 with 70% HP
/// let score = ranged(&ctx);
/// // situation = 100 (distance 5)
/// // personality = 85 (preferred_range 200, tactical 120, low aggression)
/// // modifier = 105 (HP 70%)
/// // value = (100 * 85 * 105) / 10000 = 89
/// ```
pub fn ranged(ctx: &AiContext) -> Score {
    // Feasibility: Need attack actions
    // TODO: Distinguish ranged from melee attacks
    if !ctx.has_attack_actions() {
        return Score::impossible();
    }

    // Situation: Prefer medium range (3-7 tiles)
    let distance = ctx.distance_to_player();
    let situation = match distance {
        0..=1 => 20, // Too close
        2 => 60,
        3 => 90,
        4..=6 => 100, // Optimal
        7 => 85,
        8 => 50,
        _ => 10, // Too far
    };

    // Personality: High preferred range + tactical sense
    let personality = if let Some(profile) = ctx.trait_profile() {
        let preferred_range = profile.get(TraitKind::PreferredRange) as u32;
        let tactical_sense = profile.get(TraitKind::TacticalSense) as u32;
        let aggression = profile.get(TraitKind::Aggression) as u32;

        // PreferredRange 70%, TacticalSense 20%, Anti-Aggression 10%
        (preferred_range * 70 + tactical_sense * 20 + (240 - aggression) * 10) / 24000
    } else {
        50
    };

    // Modifier: Ranged is safer, slight bonus for low HP
    let hp = ctx.hp_ratio();
    let modifier = if hp < 30 {
        90 // Still risky but better than melee
    } else if hp < 60 {
        100
    } else {
        105 // Healthy, can focus on precision
    };

    Score::new(true, situation, personality, modifier)
}

/// Hit-and-run: attack and retreat to maintain optimal range.
///
/// # Optimal Conditions
///
/// - **Distance**: 4-6 tiles (maintain distance)
/// - **HP**: > 30%
/// - **Traits**: High PreferredRange (>180), High TacticalSense (>150)
/// - **Required**: Both attack AND movement actions
///
/// # Situational Scoring
///
/// - Distance 4-6: 100 (perfect kiting range)
/// - Distance 3: 85 (need to retreat)
/// - Distance 7: 80 (need to close in)
/// - Distance 2: 60 (player getting too close)
/// - Distance 0-1: 30 (danger zone)
/// - Distance 8+: 40 (losing engagement)
///
/// # Personality Weighting
///
/// - TacticalSense: 50% (key skill)
/// - PreferredRange: 40%
/// - Caution: 10%
///
/// # Modifiers
///
/// - HP < 30%: 80 (risky)
/// - HP < 70%: 100
/// - HP >= 70%: 110
///
/// # Example
///
/// ```rust,ignore
/// // Tactical archer at distance 5, retreating as player advances
/// let score = kiting(&ctx);
/// // situation = 100 (distance 5)
/// // personality = 92 (tactical 220, preferred_range 200, caution 150)
/// // modifier = 110 (HP 85%)
/// // value = (100 * 92 * 110) / 10000 = 101 (exceeds 100, capped)
/// ```
pub fn kiting(ctx: &AiContext) -> Score {
    // Feasibility: Need BOTH attack and movement
    if !ctx.has_attack_actions() || !ctx.has_movement_actions() {
        return Score::impossible();
    }

    // Situation: Maintain 4-6 tile range
    let distance = ctx.distance_to_player();
    let situation = match distance {
        0..=1 => 30, // Danger zone
        2 => 60,
        3 => 85,
        4..=6 => 100, // Perfect
        7 => 80,
        8 => 40,
        _ => 20,
    };

    // Personality: TacticalSense + PreferredRange + Caution
    let personality = if let Some(profile) = ctx.trait_profile() {
        let tactical_sense = profile.get(TraitKind::TacticalSense) as u32;
        let preferred_range = profile.get(TraitKind::PreferredRange) as u32;
        let caution = profile.get(TraitKind::Caution) as u32;

        // TacticalSense 50%, PreferredRange 40%, Caution 10%
        (tactical_sense * 50 + preferred_range * 40 + caution * 10) / 24000
    } else {
        50
    };

    // Modifier: HP
    let hp = ctx.hp_ratio();
    let modifier = if hp < 30 {
        80
    } else if hp < 70 {
        100
    } else {
        110
    };

    Score::new(true, situation, personality, modifier)
}

/// Wait for opportunity, use stealth and positioning.
///
/// # Optimal Conditions
///
/// - **Distance**: Any (wait for perfect moment)
/// - **HP**: > 50%
/// - **Traits**: High TacticalSense (>180), Low Honor (<80)
/// - **Required**: Stealth abilities (not yet implemented)
///
/// # Current Status
///
/// **PLACEHOLDER**: Always returns impossible because stealth system
/// is not yet implemented in game-core.
///
/// # Future Implementation
///
/// When stealth is added, scoring will consider:
///
/// - Is player unaware? (situation +50)
/// - Good ambush position available? (situation +30)
/// - High TacticalSense? (personality boost)
/// - Low Honor? (personality boost - dishonorable tactic)
///
/// # Example (Future)
///
/// ```rust,ignore
/// // Assassin waiting in shadows with high tactical sense
/// let score = ambush(&ctx);
/// // situation = 80 (player unaware + good position)
/// // personality = 85 (tactical 220, low honor 40)
/// // modifier = 100
/// // value = (80 * 85 * 100) / 10000 = 68
/// ```
pub fn ambush(ctx: &AiContext) -> Score {
    // TODO: Implement when stealth system is added to game-core
    // Required capabilities:
    // - Stealth ability check: ctx.has_stealth()
    // - Player awareness: ctx.player_is_aware()
    // - Cover/position: ctx.has_ambush_position()

    let _ = ctx; // Suppress unused warning
    Score::impossible()
}

// ============================================================================
// Survival Tactics (Placeholders)
// ============================================================================

/// Run away from threats as fast as possible.
///
/// **PLACEHOLDER**: Will be implemented in Phase 2B.
pub fn flee(_ctx: &AiContext) -> Score {
    Score::impossible()
}

/// Tactical withdrawal while defending.
///
/// **PLACEHOLDER**: Will be implemented in Phase 2B.
pub fn retreat(_ctx: &AiContext) -> Score {
    Score::impossible()
}

/// Find cover or hide from enemies.
///
/// **PLACEHOLDER**: Requires cover system in game-core.
pub fn seek_cover(_ctx: &AiContext) -> Score {
    Score::impossible()
}

/// Use consumable items for survival (healing potions, etc.).
///
/// **PLACEHOLDER**: Requires item system integration.
pub fn use_survival_item(_ctx: &AiContext) -> Score {
    Score::impossible()
}

// ============================================================================
// Social Tactics (Placeholders)
// ============================================================================

/// Heal injured allies.
///
/// **PLACEHOLDER**: Requires ally detection system.
pub fn heal_ally(_ctx: &AiContext) -> Score {
    Score::impossible()
}

/// Buff allies with beneficial effects.
///
/// **PLACEHOLDER**: Requires ally detection system.
pub fn buff_ally(_ctx: &AiContext) -> Score {
    Score::impossible()
}

/// Coordinate attack with nearby allies.
///
/// **PLACEHOLDER**: Requires ally detection and coordination system.
pub fn coordinate_attack(_ctx: &AiContext) -> Score {
    Score::impossible()
}

// ============================================================================
// Exploration Tactics (Placeholders)
// ============================================================================

/// Follow preset patrol route.
///
/// **PLACEHOLDER**: Requires patrol route system.
pub fn patrol(_ctx: &AiContext) -> Score {
    Score::impossible()
}

/// Investigate suspicious sounds or sights.
///
/// **PLACEHOLDER**: Requires disturbance detection system.
pub fn investigate(_ctx: &AiContext) -> Score {
    Score::impossible()
}

/// Search area thoroughly for hidden items/enemies.
///
/// **PLACEHOLDER**: Requires search/perception system.
pub fn search(_ctx: &AiContext) -> Score {
    Score::impossible()
}

// ============================================================================
// Resource Tactics (Placeholders)
// ============================================================================

/// Collect nearby loot and items.
///
/// **PLACEHOLDER**: Requires loot detection system.
pub fn loot(_ctx: &AiContext) -> Score {
    Score::impossible()
}

/// Guard treasure or resources from intruders.
///
/// **PLACEHOLDER**: Requires treasure/territory system.
pub fn guard_treasure(_ctx: &AiContext) -> Score {
    Score::impossible()
}

// ============================================================================
// Idle Tactics (Placeholders)
// ============================================================================

/// Wait in place.
///
/// Simple fallback tactic that is always possible when there's a wait action.
/// Low scoring to ensure other tactics are preferred when available.
///
/// # Optimal Conditions
///
/// - Always available (no special requirements)
/// - Used as fallback when no better option exists
///
/// # Situational Scoring
///
/// - Always 100 (no situational factors)
///
/// # Personality Weighting
///
/// - Discipline: 50% (controlled behavior)
/// - Anti-Impulsivity: 50% (patient, not hasty)
///
/// # Modifiers
///
/// - Always 100 (no modifiers)
pub fn wait(ctx: &AiContext) -> Score {
    // Feasibility: Always possible (fallback)
    // Note: Even if no explicit wait action exists, NPC can always do nothing
    let situation = 100;

    // Personality: Disciplined and non-impulsive NPCs prefer waiting
    let personality = if let Some(profile) = ctx.trait_profile() {
        let discipline = profile.get(TraitKind::Discipline) as u32;
        let impulsivity = profile.get(TraitKind::Impulsivity) as u32;

        // Discipline 50%, Anti-Impulsivity 50%
        (discipline * 50 + (240 - impulsivity) * 50) / 24000
    } else {
        10 // Default: low preference for waiting
    };

    // Modifier: Always neutral
    let modifier = 100;

    Score::new(true, situation, personality, modifier)
}

/// Move randomly or aimlessly.
///
/// Wander around without purpose, exploring nearby areas.
/// Requires movement actions to be available.
///
/// # Optimal Conditions
///
/// - Movement actions available
/// - No immediate threats or goals
///
/// # Situational Scoring
///
/// - Always 100 (no situational factors)
///
/// # Personality Weighting
///
/// - Curiosity: 60%
/// - Anti-Discipline: 40% (wandering is undisciplined)
///
/// # Modifiers
///
/// - Always 100 (no modifiers)
pub fn wander(ctx: &AiContext) -> Score {
    // Feasibility: Need movement actions
    if !ctx.has_movement_actions() {
        return Score::impossible();
    }

    let situation = 100;

    // Personality: Curious and undisciplined NPCs prefer wandering
    let personality = if let Some(profile) = ctx.trait_profile() {
        let curiosity = profile.get(TraitKind::Curiosity) as u32;
        let discipline = profile.get(TraitKind::Discipline) as u32;

        // Curiosity 60%, Anti-Discipline 40%
        (curiosity * 60 + (240 - discipline) * 40) / 24000
    } else {
        50 // Default: moderate preference
    };

    // Modifier: Always neutral
    let modifier = 100;

    Score::new(true, situation, personality, modifier)
}
