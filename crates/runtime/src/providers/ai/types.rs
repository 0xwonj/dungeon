//! Core types for the utility-based AI system.
//!
//! This module defines the strategic (Intent) and tactical (Tactic) decision types
//! used throughout the three-layer AI architecture.

/// High-level strategic intent representing what the NPC wants to accomplish.
///
/// Intents are selected in Layer 1 based on:
/// - Current game situation (enemy nearby? low HP? loot visible?)
/// - NPC personality traits (Aggression, Bravery, Curiosity, etc.)
///
/// Each intent maps to a set of tactics that can achieve it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Intent {
    /// Engage enemies in combat.
    ///
    /// Selected when:
    /// - Enemies are visible
    /// - HP is sufficient
    /// - NPC has high Aggression/Bravery
    Combat,

    /// Preserve life and escape danger.
    ///
    /// Selected when:
    /// - HP is low
    /// - Outnumbered or outmatched
    /// - NPC has low Bravery (high cowardice)
    Survival,

    /// Investigate surroundings and search for points of interest.
    ///
    /// Selected when:
    /// - No immediate threats
    /// - NPC has high Curiosity
    /// - Unexplored areas nearby
    Exploration,

    /// Interact with allies (heal, buff, coordinate).
    ///
    /// Selected when:
    /// - Allies nearby
    /// - NPC has high Loyalty
    /// - Ally needs assistance
    Social,

    /// Acquire or protect resources and loot.
    ///
    /// Selected when:
    /// - Valuable items visible
    /// - NPC has high Greed
    /// - Treasure to guard
    Resource,

    /// Default fallback when no other intent applies.
    ///
    /// Selected when:
    /// - No threats, allies, or loot
    /// - All other intents score very low
    Idle,
}

impl Intent {
    /// Returns all intent variants in priority order.
    pub const fn all() -> [Intent; 6] {
        [
            Intent::Combat,
            Intent::Survival,
            Intent::Exploration,
            Intent::Social,
            Intent::Resource,
            Intent::Idle,
        ]
    }
}

/// Tactical approach for achieving a specific intent.
///
/// Tactics are selected in Layer 2 based on:
/// - Available actions (do I have ranged weapons? healing items?)
/// - Tactical situation (am I surrounded? at optimal range?)
/// - NPC tactical traits (PreferredRange, TacticalSense, Caution, etc.)
///
/// Each tactic defines how to score individual actions in Layer 3.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Tactic {
    // ========================================================================
    // Combat Tactics
    // ========================================================================
    /// Rush into melee range and use high-damage attacks.
    ///
    /// Requires: Melee attack actions
    /// Favored by: High Aggression, Bravery; Low PreferredRange
    /// Situation: Close range, high HP
    AggressiveMelee,

    /// Engage in melee cautiously, maintaining escape routes.
    ///
    /// Requires: Melee attack actions, movement options
    /// Favored by: High Caution, moderate Bravery
    /// Situation: Medium HP, escape route available
    DefensiveMelee,

    /// Attack from range while maintaining optimal distance.
    ///
    /// Requires: Ranged attack actions
    /// Favored by: High PreferredRange
    /// Situation: Medium range (3-7 tiles)
    Ranged,

    /// Hit-and-run: attack and retreat to maintain range.
    ///
    /// Requires: Ranged attack + movement actions
    /// Favored by: High PreferredRange, TacticalSense
    /// Situation: Player advancing, need to maintain distance
    Kiting,

    /// Wait for opportunity, use stealth and backstab.
    ///
    /// Requires: Stealth + backstab actions
    /// Favored by: Low Honor, high TacticalSense
    /// Situation: Player unaware, good positioning available
    Ambush,

    // ========================================================================
    // Survival Tactics
    // ========================================================================
    /// Run away from threats as fast as possible.
    ///
    /// Requires: Movement actions
    /// Favored by: Low Bravery, high self-preservation
    /// Situation: Low HP, outnumbered, outmatched
    Flee,

    /// Tactical withdrawal while defending.
    ///
    /// Requires: Movement + defensive actions
    /// Favored by: Moderate Bravery, high Discipline
    /// Situation: Disadvantaged but not desperate
    Retreat,

    /// Find cover or hide from enemies.
    ///
    /// Requires: Movement actions, cover available
    /// Favored by: High Caution
    /// Situation: Ranged attacks incoming, cover nearby
    SeekCover,

    /// Use consumable items for survival (healing potions, etc.).
    ///
    /// Requires: Survival items in inventory
    /// Favored by: Pragmatism
    /// Situation: Low HP, items available
    UseSurvivalItem,

    // ========================================================================
    // Social Tactics
    // ========================================================================
    /// Heal injured allies.
    ///
    /// Requires: Healing actions
    /// Favored by: High Loyalty, Empathy
    /// Situation: Ally below HP threshold
    HealAlly,

    /// Buff allies with beneficial effects.
    ///
    /// Requires: Buff actions
    /// Favored by: High Loyalty, TacticalSense
    /// Situation: Allies about to engage
    BuffAlly,

    /// Coordinate attack with nearby allies.
    ///
    /// Requires: Allies nearby, communication
    /// Favored by: High Obedience, TacticalSense
    /// Situation: Multiple allies vs single target
    CoordinateAttack,

    // ========================================================================
    // Exploration Tactics
    // ========================================================================
    /// Follow preset patrol route.
    ///
    /// Requires: Movement actions
    /// Favored by: High Discipline, Obedience
    /// Situation: No threats, on-duty
    Patrol,

    /// Investigate suspicious sounds or sights.
    ///
    /// Requires: Movement actions
    /// Favored by: High Curiosity
    /// Situation: Disturbance detected
    Investigate,

    /// Search area thoroughly for hidden items/enemies.
    ///
    /// Requires: Movement + perception
    /// Favored by: High Curiosity, Perception
    /// Situation: Safe area, time available
    Search,

    // ========================================================================
    // Resource Tactics
    // ========================================================================
    /// Collect nearby loot and items.
    ///
    /// Requires: Movement + interact actions
    /// Favored by: High Greed
    /// Situation: Valuable items nearby, safe
    Loot,

    /// Guard treasure or resources from intruders.
    ///
    /// Requires: Combat actions
    /// Favored by: High Territoriality, Greed
    /// Situation: Intruder near treasure, on-duty
    GuardTreasure,

    // ========================================================================
    // Idle Tactics
    // ========================================================================
    /// Wait in place.
    ///
    /// Requires: None
    /// Situation: Nothing to do, conserving energy
    Wait,

    /// Move randomly or aimlessly.
    ///
    /// Requires: Movement actions
    /// Favored by: High Impulsivity
    /// Situation: Bored, no objectives
    Wander,
}

impl Tactic {
    /// Returns all tactics for a given intent.
    pub const fn for_intent(intent: Intent) -> &'static [Tactic] {
        match intent {
            Intent::Combat => &[
                Tactic::AggressiveMelee,
                Tactic::DefensiveMelee,
                Tactic::Ranged,
                Tactic::Kiting,
                Tactic::Ambush,
            ],
            Intent::Survival => &[
                Tactic::Flee,
                Tactic::Retreat,
                Tactic::SeekCover,
                Tactic::UseSurvivalItem,
            ],
            Intent::Social => &[Tactic::HealAlly, Tactic::BuffAlly, Tactic::CoordinateAttack],
            Intent::Exploration => &[Tactic::Patrol, Tactic::Investigate, Tactic::Search],
            Intent::Resource => &[Tactic::Loot, Tactic::GuardTreasure],
            Intent::Idle => &[Tactic::Wait, Tactic::Wander],
        }
    }
}
