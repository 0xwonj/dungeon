//! Damage calculation and application.

use crate::stats::StatsSnapshot;

/// Calculate damage from an attack.
///
/// # Formula
///
/// ```text
/// base_damage = weapon_damage + attack_stat
/// reduced_damage = base_damage - (ac / 2)
/// final_damage = max(reduced_damage, 1)
///
/// if critical:
///     final_damage *= 2
/// ```
///
/// # Arguments
///
/// * `attacker_stats` - Attacker's stats snapshot
/// * `defender_stats` - Defender's stats snapshot
/// * `weapon_damage` - Base weapon damage
/// * `is_critical` - Whether this is a critical hit
///
/// # Returns
///
/// Final damage value
pub fn calculate_damage(
    attacker_stats: &StatsSnapshot,
    defender_stats: &StatsSnapshot,
    weapon_damage: u32,
    is_critical: bool,
) -> u32 {
    // Base damage: weapon + attack stat
    let base_damage = weapon_damage + attacker_stats.derived.attack.max(0) as u32;

    // Defense reduction: AC / 2
    let ac_reduction = (defender_stats.derived.ac.max(0) / 2) as u32;
    let reduced_damage = base_damage.saturating_sub(ac_reduction);

    // Minimum damage is 0
    let mut final_damage = reduced_damage;

    // Critical hit doubles damage
    if is_critical {
        final_damage *= 2;
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
