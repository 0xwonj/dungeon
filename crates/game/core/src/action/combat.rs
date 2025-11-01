//! Combat actions.
//!
//! Provides attack actions that work with equipped weapons and the combat
//! resolution system.

use crate::action::ActionTransition;
use crate::combat;
use crate::env::{GameEnv, ItemKind, OracleError};
use crate::error::{ErrorContext, ErrorSeverity, GameError};
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

/// Errors that can occur during attack actions.
#[derive(Clone, Debug, thiserror::Error)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum AttackError {
    /// Oracle error (items/tables not available, etc.)
    #[error(transparent)]
    Oracle(#[from] OracleError),

    /// Attacker not found in game state.
    #[error("attacker {actor:?} not found")]
    ActorNotFound {
        actor: EntityId,
        #[cfg_attr(feature = "serde", serde(skip))]
        context: ErrorContext,
    },

    /// Target not found in game state.
    #[error("target {target:?} not found")]
    TargetNotFound {
        target: EntityId,
        #[cfg_attr(feature = "serde", serde(skip))]
        context: ErrorContext,
    },

    /// Target is out of range for the weapon.
    #[error("target at {target_pos:?} out of range (max: {max_range}, distance: {distance})")]
    OutOfRange {
        target_pos: Position,
        max_range: u32,
        distance: u32,
        #[cfg_attr(feature = "serde", serde(skip))]
        context: ErrorContext,
    },

    /// Target is invalid (already dead, etc.).
    #[error("invalid target: {reason}")]
    InvalidTarget {
        reason: &'static str,
        #[cfg_attr(feature = "serde", serde(skip))]
        context: ErrorContext,
    },
}

impl AttackError {
    /// Creates an ActorNotFound error with context.
    pub fn actor_not_found(actor: EntityId, nonce: u64) -> Self {
        Self::ActorNotFound {
            actor,
            context: ErrorContext::new(nonce).with_actor(actor),
        }
    }

    /// Creates a TargetNotFound error with context.
    pub fn target_not_found(target: EntityId, actor: EntityId, nonce: u64) -> Self {
        Self::TargetNotFound {
            target,
            context: ErrorContext::new(nonce).with_actor(actor),
        }
    }

    /// Creates an OutOfRange error with context.
    pub fn out_of_range(
        target_pos: Position,
        max_range: u32,
        distance: u32,
        actor: EntityId,
        nonce: u64,
    ) -> Self {
        Self::OutOfRange {
            target_pos,
            max_range,
            distance,
            context: ErrorContext::new(nonce)
                .with_actor(actor)
                .with_position(target_pos),
        }
    }

    /// Creates an InvalidTarget error with context.
    pub fn invalid_target(reason: &'static str, actor: EntityId, nonce: u64) -> Self {
        Self::InvalidTarget {
            reason,
            context: ErrorContext::new(nonce).with_actor(actor),
        }
    }
}

impl GameError for AttackError {
    fn severity(&self) -> ErrorSeverity {
        match self {
            Self::Oracle(e) => e.severity(),
            Self::ActorNotFound { .. } | Self::TargetNotFound { .. } => ErrorSeverity::Validation,
            Self::OutOfRange { .. } => ErrorSeverity::Recoverable,
            Self::InvalidTarget { .. } => ErrorSeverity::Validation,
        }
    }

    fn context(&self) -> Option<&ErrorContext> {
        match self {
            Self::Oracle(_) => None,
            Self::ActorNotFound { context, .. }
            | Self::TargetNotFound { context, .. }
            | Self::OutOfRange { context, .. }
            | Self::InvalidTarget { context, .. } => Some(context),
        }
    }

    fn error_code(&self) -> &'static str {
        match self {
            Self::Oracle(_) => "ATTACK_ORACLE",
            Self::ActorNotFound { .. } => "ATTACK_ACTOR_NOT_FOUND",
            Self::TargetNotFound { .. } => "ATTACK_TARGET_NOT_FOUND",
            Self::OutOfRange { .. } => "ATTACK_OUT_OF_RANGE",
            Self::InvalidTarget { .. } => "ATTACK_INVALID_TARGET",
        }
    }
}

impl ActionTransition for AttackAction {
    type Error = AttackError;
    type Result = crate::combat::AttackResult;

    fn actor(&self) -> EntityId {
        self.actor
    }

    fn cost(&self, env: &GameEnv<'_>) -> Tick {
        env.tables().map(|t| t.action_costs().attack).unwrap_or(100)
    }

    fn pre_validate(&self, state: &GameState, env: &GameEnv<'_>) -> Result<(), Self::Error> {
        let nonce = state.turn.nonce;

        let attacker = state
            .entities
            .actor(self.actor)
            .ok_or_else(|| AttackError::actor_not_found(self.actor, nonce))?;

        let defender = state
            .entities
            .actor(self.target)
            .ok_or_else(|| AttackError::target_not_found(self.target, self.actor, nonce))?;

        // Get weapon info to determine attack type
        let weapon_kind = get_weapon_kind(attacker, env)?;
        let attack_type = weapon_kind.attack_type();

        // Range check based on attack type
        match attack_type {
            AttackType::Melee => {
                let range = weapon_kind.melee_range();
                if !is_within_range(attacker.position, defender.position, range) {
                    let distance = chebyshev_distance(attacker.position, defender.position);
                    return Err(AttackError::out_of_range(
                        defender.position,
                        range,
                        distance,
                        self.actor,
                        nonce,
                    ));
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
        let nonce = state.turn.nonce;

        let attacker = state
            .entities
            .actor(self.actor)
            .ok_or_else(|| AttackError::actor_not_found(self.actor, nonce))?;

        let defender = state
            .entities
            .actor(self.target)
            .ok_or_else(|| AttackError::target_not_found(self.target, self.actor, nonce))?;

        // Snapshot stats
        let attacker_stats = attacker.snapshot();
        let defender_stats = defender.snapshot();

        // Get weapon damage
        let weapon_damage = get_weapon_damage(attacker, env)?;

        // Generate deterministic seed for this attack roll
        let roll = if let Ok(rng_oracle) = env.rng() {
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

        let tables = env.tables()?;
        let result = combat::resolve_attack(
            &attacker_stats,
            &defender_stats,
            weapon_damage,
            roll,
            tables,
        );

        // === Pass 2: Apply damage (mutable) ===

        if let Some(damage) = result.damage {
            let defender = state
                .entities
                .actor_mut(self.target)
                .ok_or_else(|| AttackError::target_not_found(self.target, self.actor, nonce))?;

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
        // env.items() returns Result<&dyn ItemOracle, OracleError>
        if let Ok(items_oracle) = env.items()
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
        // env.items() returns Result<&dyn ItemOracle, OracleError>
        if let Ok(items_oracle) = env.items()
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

/// Calculate Chebyshev distance between two positions.
///
/// Chebyshev distance = max(|dx|, |dy|) (8-directional movement)
fn chebyshev_distance(from: Position, to: Position) -> u32 {
    let dx = (from.x - to.x).unsigned_abs();
    let dy = (from.y - to.y).unsigned_abs();
    dx.max(dy)
}

/// Check if target is within range using Chebyshev distance.
fn is_within_range(from: Position, to: Position, range: u32) -> bool {
    chebyshev_distance(from, to) <= range
}
