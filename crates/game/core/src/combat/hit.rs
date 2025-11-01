//! Hit chance and accuracy calculations.

use crate::env::TablesOracle;

/// Calculate hit chance based on accuracy vs evasion.
///
/// # Formula
///
/// ```text
/// hit_chance = base + (accuracy - evasion)
/// clamped to [min, max]
/// ```
///
/// # Arguments
///
/// * `accuracy` - Attacker's accuracy stat (from derived stats)
/// * `evasion` - Defender's evasion stat (from derived stats)
/// * `tables` - Balance parameters oracle
///
/// # Returns
///
/// Hit chance as percentage
pub fn calculate_hit_chance(
    accuracy: i32,
    evasion: i32,
    tables: &(impl TablesOracle + ?Sized),
) -> u32 {
    let params = tables.combat().hit_chance;

    let stat_diff = accuracy - evasion;
    let hit_chance = params.base + stat_diff;

    hit_chance.clamp(params.min as i32, params.max as i32) as u32
}

/// Check if an attack hits based on accuracy, evasion, and random roll.
///
/// # Arguments
///
/// * `accuracy` - Attacker's accuracy stat
/// * `evasion` - Defender's evasion stat
/// * `roll` - Random roll (0-100)
/// * `tables` - Balance parameters oracle
///
/// # Returns
///
/// `true` if attack hits, `false` if it misses.
pub fn check_hit(
    accuracy: i32,
    evasion: i32,
    roll: u32,
    tables: &(impl TablesOracle + ?Sized),
) -> bool {
    let hit_chance = calculate_hit_chance(accuracy, evasion, tables);
    roll <= hit_chance
}
