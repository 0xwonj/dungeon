//! Combat actions.
//!
//! Provides attack actions that work with equipped weapons and the combat
//! resolution system.

use crate::action::ActionTransition;
use crate::combat;
use crate::env::{GameEnv, ItemKind};
use crate::state::{AttackType, EntityId, GameState, Position, Tick, WeaponKind};

/// Basic attack action - works with any equipped weapon.
///
/// Attack type (melee/ranged/magic) is automatically determined by the
/// equipped weapon:
/// - Sword, Axe, Dagger, Spear, Unarmed → melee (adjacent)
/// - Bow, Crossbow → ranged (long distance)
/// - Staff, Wand → magic
///
/// # Combat Resolution
///
/// 1. **Pre-validate**: Range check based on weapon type
/// 2. **Apply**: Snapshot stats, roll for hit, calculate damage, apply to HP
/// 3. **RNG**: Uses deterministic RNG oracle with `compute_seed()`
#[derive(Clone, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct AttackAction {
    pub actor: EntityId,
    pub target: EntityId,
}

impl AttackAction {
    pub fn new(actor: EntityId, target: EntityId) -> Self {
        Self { actor, target }
    }
}

/// Error types for attack validation.
#[derive(Clone, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum AttackError {
    ActorNotFound,
    TargetNotFound,
    OutOfRange,
    InvalidTarget,
}

impl core::fmt::Display for AttackError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            AttackError::ActorNotFound => write!(f, "actor not found"),
            AttackError::TargetNotFound => write!(f, "target not found"),
            AttackError::OutOfRange => write!(f, "target out of range"),
            AttackError::InvalidTarget => write!(f, "invalid target"),
        }
    }
}

impl ActionTransition for AttackAction {
    type Error = AttackError;
    type Result = crate::combat::AttackResult;

    fn actor(&self) -> EntityId {
        self.actor
    }

    fn cost(&self) -> Tick {
        6 // Base cost (modified by speed stats)
    }

    fn pre_validate(&self, state: &GameState, env: &GameEnv<'_>) -> Result<(), Self::Error> {
        let attacker = state
            .entities
            .actor(self.actor)
            .ok_or(AttackError::ActorNotFound)?;

        let defender = state
            .entities
            .actor(self.target)
            .ok_or(AttackError::TargetNotFound)?;

        // Get weapon info to determine attack type
        let weapon_kind = get_weapon_kind(attacker, env)?;
        let attack_type = weapon_kind.attack_type();

        // Range check based on attack type
        match attack_type {
            AttackType::Melee => {
                let range = weapon_kind.melee_range();
                if !is_within_range(attacker.position, defender.position, range) {
                    return Err(AttackError::OutOfRange);
                }
            }
            AttackType::Ranged => {
                // TODO: Implement ranged range check (10 tiles? line of sight?)
                // For now, allow any distance
            }
            AttackType::Magic => {
                // TODO: Implement magic range check
                // For now, allow any distance
            }
        }

        Ok(())
    }

    fn apply(&self, state: &mut GameState, env: &GameEnv<'_>) -> Result<Self::Result, Self::Error> {
        // === Pass 1: Gather data (immutable) ===

        let attacker = state
            .entities
            .actor(self.actor)
            .ok_or(AttackError::ActorNotFound)?;

        let defender = state
            .entities
            .actor(self.target)
            .ok_or(AttackError::TargetNotFound)?;

        // Snapshot stats
        let attacker_stats = attacker.snapshot();
        let defender_stats = defender.snapshot();

        // Get weapon damage
        let weapon_damage = get_weapon_damage(attacker, env)?;

        // Generate deterministic seed for this attack roll
        let roll = if let Some(rng_oracle) = env.rng() {
            use crate::env::compute_seed;
            let seed = compute_seed(
                state.game_seed,
                state.turn.nonce,
                self.actor.0,
                0, // context: 0 = hit roll
            );
            rng_oracle.roll_d100(seed)
        } else {
            // No RNG oracle: always hit (roll=0, hit_chance >= 5%)
            0
        };

        // === Combat resolution (pure function) ===

        let result = combat::resolve_attack(&attacker_stats, &defender_stats, weapon_damage, roll);

        // === Pass 2: Apply damage (mutable) ===

        if let Some(damage) = result.damage {
            let defender = state
                .entities
                .actor_mut(self.target)
                .ok_or(AttackError::TargetNotFound)?;

            defender.resources.hp = combat::apply_damage(defender.resources.hp, damage);
        }

        Ok(result)
    }
}

// ============================================================================
// Helper Functions
// ============================================================================

/// Get the weapon kind for an actor (Unarmed if no weapon equipped).
fn get_weapon_kind(
    actor: &crate::state::ActorState,
    env: &GameEnv<'_>,
) -> Result<WeaponKind, AttackError> {
    if let Some(weapon_handle) = actor.equipment.weapon {
        // env.items() returns Option<&dyn ItemOracle>, need to unwrap first
        if let Some(items_oracle) = env.items()
            && let Some(item_def) = items_oracle.definition(weapon_handle)
            && let ItemKind::Weapon(weapon_data) = item_def.kind
        {
            return Ok(weapon_data.kind);
        }
    }

    // Default: unarmed
    Ok(WeaponKind::Unarmed)
}

/// Get weapon damage for an actor.
///
/// If weapon is equipped, returns weapon damage.
/// If unarmed, returns base unarmed damage (1 + STR/10).
fn get_weapon_damage(
    actor: &crate::state::ActorState,
    env: &GameEnv<'_>,
) -> Result<u32, AttackError> {
    if let Some(weapon_handle) = actor.equipment.weapon {
        // env.items() returns Option<&dyn ItemOracle>, need to unwrap first
        if let Some(items_oracle) = env.items()
            && let Some(item_def) = items_oracle.definition(weapon_handle)
            && let ItemKind::Weapon(weapon_data) = item_def.kind
        {
            return Ok(weapon_data.damage as u32);
        }
    }

    // Unarmed damage: 1 + STR/10
    let str_bonus = actor.core_stats.str / 10;
    Ok(1 + str_bonus as u32)
}

/// Check if target is within range using Chebyshev distance.
///
/// Chebyshev distance = max(|dx|, |dy|) (8-directional movement)
fn is_within_range(from: Position, to: Position, range: u32) -> bool {
    let dx = (from.x - to.x).abs();
    let dy = (from.y - to.y).abs();
    let distance = dx.max(dy); // Chebyshev distance
    distance <= range as i32
}
