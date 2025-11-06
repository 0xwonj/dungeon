//! Action and passive ability system for actors.
//!
//! This module defines all actions an entity can perform and passive traits
//! they possess. Actions correspond 1:1 with game actions, while passives
//! provide automatic benefits or capabilities.
//!
//! # Design
//!
//! - **ActionAbility**: Active abilities that can be used (Move, Attack, Fireball)
//! - **PassiveAbility**: Passive traits (Flight, Regeneration, SeeInvisible)
//! - Both can be enabled/disabled (e.g., by status effects)
//! - Actions have cooldowns, passives don't

use arrayvec::ArrayVec;

use crate::action::ActionKind;
use crate::config::GameConfig;
use crate::state::Tick;

// ============================================================================
// Action Abilities (Active)
// ============================================================================

/// An active ability that can be used by an actor.
///
/// Action abilities correspond to in-game actions. If an actor doesn't have
/// a specific action ability, they cannot perform that action.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct ActionAbility {
    pub kind: ActionKind,
    pub enabled: bool,
    pub cooldown_until: Tick,
}

impl ActionAbility {
    pub fn new(kind: ActionKind) -> Self {
        Self {
            kind,
            enabled: true,
            cooldown_until: 0,
        }
    }

    pub fn is_ready(&self, current_tick: Tick) -> bool {
        self.enabled && self.cooldown_until <= current_tick
    }
}

// ============================================================================
// Passive Abilities
// ============================================================================

/// A passive ability that provides automatic benefits.
///
/// Passive abilities don't need to be activated - they're always active
/// when enabled. They provide capabilities (Flight, Swim) or automatic
/// effects (Regeneration, Thorns).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct PassiveAbility {
    pub kind: PassiveKind,
    pub enabled: bool,
}

impl PassiveAbility {
    pub fn new(kind: PassiveKind) -> Self {
        Self {
            kind,
            enabled: true,
        }
    }
}

/// Types of passive abilities.
///
/// These provide automatic benefits or capabilities without explicit use.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum PassiveKind {
    // ========================================================================
    // Movement Capabilities
    // ========================================================================
    /// Can fly over obstacles and chasms.
    Flight,

    /// Can move through water tiles.
    Swim,

    /// Can climb walls and cliffs.
    WallClimb,

    // ========================================================================
    // Perception
    // ========================================================================
    /// Can see invisible entities.
    SeeInvisible,

    /// Can see in darkness.
    Darkvision,

    /// Can see through illusions.
    TrueSight,

    // ========================================================================
    // Combat Passives
    // ========================================================================
    /// Reflect damage to attackers.
    Thorns,

    /// Steal HP from attacks.
    LifeSteal,

    /// Chance for critical hits.
    CriticalStrike,

    // ========================================================================
    // Survival
    // ========================================================================
    /// Automatically regenerate HP over time.
    Regeneration,

    /// Immune to poison damage.
    PoisonImmunity,

    /// Resistance to fire damage.
    FireResistance,

    /// Resistance to cold damage.
    ColdResistance,

    // ========================================================================
    // Special
    // ========================================================================
    /// Undead creature (special rules apply).
    Undead,

    /// Construct (immune to poison, bleeding, etc.).
    Construct,

    /// Ethereal (can pass through walls).
    Ethereal,
}

// ============================================================================
// Helper Collections (for ActorState)
// ============================================================================

/// Collection of action abilities.
pub type ActionAbilities = ArrayVec<ActionAbility, { GameConfig::MAX_ACTIONS }>;

/// Collection of passive abilities.
pub type PassiveAbilities = ArrayVec<PassiveAbility, { GameConfig::MAX_PASSIVES }>;
