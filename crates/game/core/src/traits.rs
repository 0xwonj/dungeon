//! Behavioral traits and identity for AI actors.
//!
//! Traits are stored in ActorState to enable deterministic re-execution during
//! challenge verification in the optimistic rollup system.
//!
//! # Design Rationale
//!
//! For challenge games to work, zkVM must be able to re-execute provider decisions
//! with identical inputs. Since the Utility AI provider uses trait profiles heavily
//! in decision-making, traits must be part of canonical state, not just runtime config.
//!
//! # Challenge Game Flow with Traits
//!
//! 1. **Optimistic execution**: Runtime uses UtilityAiProvider with trait profile to generate actions
//! 2. **Challenge**: Someone disputes that the action came from the declared provider with those traits
//! 3. **Resolution**: zkVM re-executes UtilityAiProvider with same state + provider_kind + trait_profile
//! 4. **Fraud detection**: If expected action ≠ submitted action, slash malicious player

/// Actor species (biological/existential identity).
///
/// Species is immutable and affects:
/// - Resistances and vulnerabilities (undead immune to poison)
/// - Species-specific interactions (human vs undead)
/// - Equipment restrictions (some items only for certain species)
#[derive(
    Clone,
    Copy,
    Debug,
    Default,
    PartialEq,
    Eq,
    Hash,
    strum::Display,
    strum::EnumString,
    strum::AsRefStr,
)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[strum(serialize_all = "snake_case", ascii_case_insensitive)]
pub enum Species {
    /// Human - versatile, balanced
    #[default]
    Human,
    /// Goblin - small, quick, cowardly
    Goblin,
    /// Orc - strong, aggressive
    Orc,
    /// Skeleton - undead, fearless
    Skeleton,
    /// Boss - unique powerful entity
    Boss,
    /// Elf - agile, perceptive
    Elf,
    /// Dwarf - sturdy, resistant
    Dwarf,
    /// Undead - risen dead
    Undead,
    /// Dragon - powerful flying creature
    Dragon,
}

/// Actor faction (relationship/allegiance).
///
/// Faction affects:
/// - AI targeting (friendly/neutral/hostile)
/// - Dialogue and trading availability
/// - Quest and event triggers
/// - Damage bonuses between factions
///
/// Faction can change during gameplay (betrayal, conversion).
#[derive(
    Clone,
    Copy,
    Debug,
    Default,
    PartialEq,
    Eq,
    Hash,
    strum::Display,
    strum::EnumString,
    strum::AsRefStr,
)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[strum(serialize_all = "snake_case", ascii_case_insensitive)]
pub enum Faction {
    /// No specific faction (default)
    #[default]
    None,
    /// Player's faction
    Player,
    /// Friendly to player
    Friendly,
    /// Neutral (won't attack unless provoked)
    Neutral,
    /// Hostile to player
    Hostile,
    /// Goblin tribes
    GoblinClan,
    /// Orc warbands
    OrcHorde,
    /// Undead forces
    UndeadLegion,
    /// Wildlife (animals, beasts)
    Wildlife,
}

impl Faction {
    /// Check if this faction is hostile to another faction.
    pub fn is_hostile_to(&self, other: &Faction) -> bool {
        match (self, other) {
            // Player vs Hostile factions
            (Faction::Player, Faction::Hostile)
            | (Faction::Hostile, Faction::Player)
            | (Faction::Player, Faction::GoblinClan)
            | (Faction::GoblinClan, Faction::Player)
            | (Faction::Player, Faction::OrcHorde)
            | (Faction::OrcHorde, Faction::Player)
            | (Faction::Player, Faction::UndeadLegion)
            | (Faction::UndeadLegion, Faction::Player) => true,

            // Friendly never fights player or each other
            (Faction::Friendly, Faction::Player)
            | (Faction::Player, Faction::Friendly)
            | (Faction::Friendly, Faction::Friendly) => false,

            // Neutral doesn't initiate combat
            (Faction::Neutral, _) | (_, Faction::Neutral) => false,

            // None doesn't fight
            (Faction::None, _) | (_, Faction::None) => false,

            // Inter-faction hostility
            (Faction::GoblinClan, Faction::OrcHorde) | (Faction::OrcHorde, Faction::GoblinClan) => {
                true
            }

            // Same faction doesn't fight
            _ if self == other => false,

            // Default: no hostility
            _ => false,
        }
    }
}

/// The 20 core behavioral traits.
///
/// Each trait ranges from 0 (minimum) to 240 (maximum) in the final composed profile.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[repr(u8)]
pub enum TraitKind {
    // ========================================================================
    // Psychology/Temperament
    // ========================================================================
    /// Risk tolerance and fear resistance.
    ///
    /// - High: Fights longer, flees at lower HP
    /// - Low: Flees early, avoids danger
    Bravery = 0,

    /// Plan adherence, panic resistance, focus.
    ///
    /// - High: Sticks to tactics, resists distraction
    /// - Low: Easily disrupted, changes plans
    Discipline = 1,

    /// Initiative and pursuit willingness.
    ///
    /// - High: Initiates combat, chases aggressively
    /// - Low: Passive, defensive
    Aggression = 2,

    /// Caution and risk avoidance.
    ///
    /// - High: Checks corners, avoids traps, prepares
    /// - Low: Rushes in, ignores preparation
    Caution = 3,

    /// Exploration and investigation drive.
    ///
    /// - High: Investigates sounds, searches thoroughly
    /// - Low: Ignores distractions
    Curiosity = 4,

    /// Ally protection and call response.
    ///
    /// - High: Protects allies, responds to calls
    /// - Low: Self-preservation over allies
    Loyalty = 5,

    /// Loot obsession and resource hoarding.
    ///
    /// - High: Prioritizes looting, guards treasure
    /// - Low: Ignores loot
    Greed = 6,

    /// Spontaneity and explosive behavior.
    ///
    /// - High: Acts on impulse, sudden actions
    /// - Low: Deliberate, controlled
    Impulsivity = 7,

    /// Non-lethal and negotiation preference.
    ///
    /// - High: Prefers capture, negotiates, shows mercy
    /// - Low: Lethal force, no negotiation
    Empathy = 8,

    // ========================================================================
    // Cognition/Tactics
    // ========================================================================
    /// Vision, hearing, stealth detection baseline.
    ///
    /// - High: Wide vision, detects stealth easily
    /// - Low: Narrow perception, easily flanked
    Perception = 9,

    /// Positioning, cooldown timing, synergy awareness.
    ///
    /// - High: Optimal positioning, uses abilities well
    /// - Low: Poor positioning, wastes cooldowns
    TacticalSense = 10,

    /// Alertness retention and suspicion accumulation.
    ///
    /// - High: Remembers events, stays alert longer
    /// - Low: Forgets quickly, drops guard
    Memory = 11,

    // ========================================================================
    // Physical/Movement/Range
    // ========================================================================
    /// Acceleration, dodge, terrain adaptation.
    ///
    /// - High: Fast movement, good dodging
    /// - Low: Slow, clumsy
    Mobility = 12,

    /// Long pursuit, patrol, alertness duration.
    ///
    /// - High: Chases long distances, patrols far
    /// - Low: Gives up quickly, short patrols
    Stamina = 13,

    /// Combat range preference (0=melee, 240=long range/kiting).
    ///
    /// - High: Prefers ranged combat, kites
    /// - Low: Prefers melee engagement
    PreferredRange = 14,

    // ========================================================================
    // Social/Norms/Territory
    // ========================================================================
    /// Refusal to leave territory, area rage.
    ///
    /// - High: Never leaves home area, enraged if invaded
    /// - Low: Roams freely
    Territoriality = 15,

    /// Doctrine/command/formation adherence.
    ///
    /// - High: Follows orders strictly, maintains formation
    /// - Low: Acts independently
    Obedience = 16,

    /// Frontal combat preference, cowardly tactic aversion.
    ///
    /// - High: Prefers fair fights, dislikes ambushes
    /// - Low: Uses any tactic to win
    Honor = 17,

    /// Magic/light/taboo biases.
    ///
    /// - High: Affected by curses, fears holy symbols
    /// - Low: Rational, ignores superstitions
    Superstition = 18,

    /// Intimidation and command success tendency.
    ///
    /// - High: Successfully intimidates, commands respect
    /// - Low: Fails to intimidate
    Dominance = 19,
}

impl TraitKind {
    /// Total number of traits.
    pub const COUNT: usize = 20;

    /// Returns all trait kinds in order.
    pub const fn all() -> [TraitKind; Self::COUNT] {
        [
            TraitKind::Bravery,
            TraitKind::Discipline,
            TraitKind::Aggression,
            TraitKind::Caution,
            TraitKind::Curiosity,
            TraitKind::Loyalty,
            TraitKind::Greed,
            TraitKind::Impulsivity,
            TraitKind::Empathy,
            TraitKind::Perception,
            TraitKind::TacticalSense,
            TraitKind::Memory,
            TraitKind::Mobility,
            TraitKind::Stamina,
            TraitKind::PreferredRange,
            TraitKind::Territoriality,
            TraitKind::Obedience,
            TraitKind::Honor,
            TraitKind::Superstition,
            TraitKind::Dominance,
        ]
    }

    /// Returns the trait as a u8 index.
    #[inline]
    pub const fn as_index(self) -> usize {
        self as usize
    }
}

/// Final composed trait profile for an AI actor.
///
/// Each value is 0..240 (computed from 4 layers: Species × Archetype × Faction × Temperament).
/// This profile is used by the Utility AI provider to compute utility scores and thresholds.
///
/// # Storage in State
///
/// TraitProfile is stored directly in ActorState to enable deterministic re-execution
/// during challenge verification. The zkVM can access the exact traits used during
/// optimistic execution to prove/disprove fraud.
///
/// # Composition
///
/// Profiles are composed in game-content from 4 trait layers with weighted averaging.
/// The final composed profile is stored here as a simple array for ZK efficiency.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct TraitProfile {
    /// Composed trait values (0..240).
    pub values: [u8; TraitKind::COUNT],
}

impl Default for TraitProfile {
    fn default() -> Self {
        Self {
            values: [120; TraitKind::COUNT], // Middle value (neutral)
        }
    }
}

impl TraitProfile {
    /// Creates a profile with all traits set to a specific value.
    pub fn uniform(value: u8) -> Self {
        let clamped = if value > 240 { 240 } else { value };
        Self {
            values: [clamped; TraitKind::COUNT],
        }
    }

    /// Creates a profile from raw values, clamping to 0..240.
    pub fn from_raw(values: [u8; TraitKind::COUNT]) -> Self {
        let mut clamped = [0u8; TraitKind::COUNT];
        let mut i = 0;
        while i < TraitKind::COUNT {
            clamped[i] = if values[i] > 240 { 240 } else { values[i] };
            i += 1;
        }
        Self { values: clamped }
    }

    /// Gets the composed value for a specific trait (0..240).
    #[inline]
    pub fn get(&self, trait_kind: TraitKind) -> u8 {
        self.values[trait_kind.as_index()]
    }

    /// Returns the raw trait values array.
    #[inline]
    pub fn values(&self) -> &[u8; TraitKind::COUNT] {
        &self.values
    }
}
