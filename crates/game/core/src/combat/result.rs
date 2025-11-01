//! Combat result types and attack resolution.

use crate::env::TablesOracle;
use crate::stats::StatsSnapshot;

use super::damage::calculate_damage;
use super::hit::check_hit;

/// Outcome of an attack attempt.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum AttackOutcome {
    /// Attack missed the target.
    Miss,
    /// Attack hit the target.
    Hit,
    /// Critical hit (not yet implemented).
    Critical,
}

/// Result of a combat resolution.
#[derive(Clone, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct AttackResult {
    /// Whether the attack hit, missed, or was critical.
    pub outcome: AttackOutcome,

    /// Damage dealt (None if miss).
    pub damage: Option<u32>,
}

/// Resolve a complete attack sequence (hit check + damage calculation).
///
/// This is the main entry point for basic combat resolution.
///
/// # Arguments
///
/// * `attacker_stats` - Snapshot of attacker's stats
/// * `defender_stats` - Snapshot of defender's stats
/// * `weapon_damage` - Base weapon damage
/// * `roll` - Random roll for hit check (0-100)
/// * `tables` - Balance parameters oracle
///
/// # Returns
///
/// Complete attack result with outcome and damage.
pub fn resolve_attack(
    attacker_stats: &StatsSnapshot,
    defender_stats: &StatsSnapshot,
    weapon_damage: u32,
    roll: u32,
    tables: &(impl TablesOracle + ?Sized),
) -> AttackResult {
    // 1. Check if attack hits
    let hit = check_hit(
        attacker_stats.derived.accuracy,
        defender_stats.derived.evasion,
        roll,
        tables,
    );

    if !hit {
        return AttackResult {
            outcome: AttackOutcome::Miss,
            damage: None,
        };
    }

    // 2. Calculate damage
    let damage = calculate_damage(attacker_stats, defender_stats, weapon_damage, false, tables);

    AttackResult {
        outcome: AttackOutcome::Hit,
        damage: Some(damage),
    }
}
