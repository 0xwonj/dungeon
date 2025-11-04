//! Damage calculation and application.

use crate::env::TablesOracle;
use crate::stats::StatsSnapshot;

// ============================================================================
// Damage Type
// ============================================================================

/// Damage type for resistances and damage calculation.
///
/// Different damage types may have different resistance values on actors.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum DamageType {
    /// Physical damage (melee, projectiles).
    Physical,
    /// Fire damage (burns, explosions).
    Fire,
    /// Cold damage (ice, frost).
    Cold,
    /// Lightning damage (electricity, storms).
    Lightning,
    /// Poison damage (toxins, venom).
    Poison,
    /// Arcane damage (pure magic).
    Arcane,
    /// True damage (ignores all resistances).
    True,
}

// ============================================================================
// Damage Calculation
// ============================================================================

/// Calculate damage from an attack.
///
/// # Formula
///
/// ```text
/// base_damage = weapon_damage + attack_stat
/// reduced_damage = base_damage - (ac / ac_divisor)
/// final_damage = max(reduced_damage, minimum)
///
/// if critical:
///     final_damage *= crit_multiplier
/// ```
///
/// Balance parameters are provided by TablesOracle:
/// - AC divisor (default: 2)
/// - Critical multiplier (default: 2)
/// - Minimum damage (default: 0)
///
/// # Arguments
///
/// * `attacker_stats` - Attacker's stats snapshot
/// * `defender_stats` - Defender's stats snapshot
/// * `weapon_damage` - Base weapon damage
/// * `is_critical` - Whether this is a critical hit
/// * `tables` - Balance parameters oracle
///
/// # Returns
///
/// Final damage value
pub fn calculate_damage(
    attacker_stats: &StatsSnapshot,
    defender_stats: &StatsSnapshot,
    weapon_damage: u32,
    is_critical: bool,
    tables: &(impl TablesOracle + ?Sized),
) -> u32 {
    let params = tables.combat().damage;

    // Base damage: weapon + attack stat
    let base_damage = weapon_damage + attacker_stats.derived.attack.max(0) as u32;

    // Defense reduction: AC / divisor
    let ac_reduction = (defender_stats.derived.ac.max(0) / params.ac_divisor as i32) as u32;
    let reduced_damage = base_damage.saturating_sub(ac_reduction);

    // Apply minimum damage
    let mut final_damage = reduced_damage.max(params.minimum);

    // Critical hit multiplies damage
    if is_critical {
        final_damage *= params.crit_multiplier;
    }

    final_damage
}

/// Apply damage to current HP.
///
/// # Arguments
///
/// * `current_hp` - Current HP value
/// * `damage` - Damage to apply
///
/// # Returns
///
/// New HP value (clamped to 0)
pub fn apply_damage(current_hp: u32, damage: u32) -> u32 {
    current_hp.saturating_sub(damage)
}
