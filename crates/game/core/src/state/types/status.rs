//! Status effect system for actors.
//!
//! Status effects are temporary conditions that modify actor capabilities,
//! provide bonuses/penalties, or restrict actions.
//!
//! # Tick-based Duration
//!
//! Effects store `expires_at: Tick` to handle the tick-based turn system
//! where multiple ticks can pass at once. Effects are removed when
//! `current_tick >= expires_at`.

use arrayvec::ArrayVec;

use crate::config::GameConfig;
use crate::state::Tick;

/// Active status effects on an actor.
///
/// Status effects include:
/// - Crowd control (Stunned, Rooted, Silenced)
/// - Buffs (Hasted, Shielded, Invisible)
/// - Debuffs (Poisoned, Weakened, Burning)
/// - Special states (Berserk, Frightened)
#[derive(Clone, Debug, Default, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct StatusEffects {
    effects: ArrayVec<StatusEffect, { GameConfig::MAX_STATUS_EFFECTS }>,
}

/// A single status effect with expiration time.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct StatusEffect {
    pub kind: StatusEffectKind,
    /// Tick at which this effect expires.
    pub expires_at: Tick,
}

/// Types of status effects.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum StatusEffectKind {
    // ========================================================================
    // Crowd Control (restricts actions)
    // ========================================================================
    /// Cannot act at all (only Wait).
    Stunned,

    /// Cannot move.
    Rooted,

    /// Cannot use magic.
    Silenced,

    /// Cannot attack.
    Disarmed,

    // ========================================================================
    // Buffs (positive effects)
    // ========================================================================
    /// Cannot be targeted by enemies.
    Invisible,

    /// Increased movement speed.
    Hasted,

    /// Defense bonus.
    Shielded,

    /// HP recovery over time.
    Regenerating,

    // ========================================================================
    // Debuffs (negative effects)
    // ========================================================================
    /// HP loss over time.
    Poisoned,

    /// Attack damage reduced.
    Weakened,

    /// Movement restricted.
    Slowed,

    /// Fire damage over time.
    Burning,

    // ========================================================================
    // Special States
    // ========================================================================
    /// Can only attack (no defensive actions).
    Berserk,

    /// Must flee from enemies.
    Frightened,
}

impl StatusEffects {
    /// Creates an empty status effect set.
    pub fn empty() -> Self {
        Self {
            effects: ArrayVec::new(),
        }
    }

    /// Checks if a specific status effect is active at the given tick.
    pub fn has(&self, kind: StatusEffectKind, current_tick: Tick) -> bool {
        self.effects
            .iter()
            .any(|e| e.kind == kind && e.expires_at > current_tick)
    }

    /// Gets the expiration tick of a status effect.
    ///
    /// Returns None if the effect is not active.
    pub fn expires_at(&self, kind: StatusEffectKind, current_tick: Tick) -> Option<Tick> {
        self.effects
            .iter()
            .find(|e| e.kind == kind && e.expires_at > current_tick)
            .map(|e| e.expires_at)
    }

    /// Adds a status effect with expiration time.
    ///
    /// If the effect already exists, extends to the later expiration time.
    ///
    /// # Arguments
    ///
    /// * `kind` - The type of status effect
    /// * `expires_at` - Tick at which the effect expires
    pub fn add(&mut self, kind: StatusEffectKind, expires_at: Tick) {
        // Check if already present
        if let Some(existing) = self.effects.iter_mut().find(|e| e.kind == kind) {
            // Extend to later expiration
            existing.expires_at = existing.expires_at.max(expires_at);
            return;
        }

        // Add new effect if space available
        if !self.effects.is_full() {
            self.effects.push(StatusEffect { kind, expires_at });
        }
    }

    /// Removes a status effect immediately.
    pub fn remove(&mut self, kind: StatusEffectKind) {
        self.effects.retain(|e| e.kind != kind);
    }

    /// Removes all expired status effects at the current tick.
    ///
    /// Call this when the game tick advances to clean up expired effects.
    pub fn remove_expired(&mut self, current_tick: Tick) {
        self.effects.retain(|e| e.expires_at > current_tick);
    }

    /// Returns an iterator over all active effects at the given tick.
    pub fn active_at(&self, current_tick: Tick) -> impl Iterator<Item = &StatusEffect> + '_ {
        self.effects
            .iter()
            .filter(move |e| e.expires_at > current_tick)
    }

    /// Returns an iterator over all effects (including expired).
    pub fn iter(&self) -> impl Iterator<Item = &StatusEffect> {
        self.effects.iter()
    }

    /// Returns true if no status effects are active at the given tick.
    pub fn is_empty_at(&self, current_tick: Tick) -> bool {
        !self.effects.iter().any(|e| e.expires_at > current_tick)
    }

    /// Returns true if no status effects exist (including expired).
    pub fn is_empty(&self) -> bool {
        self.effects.is_empty()
    }
}
