//! Hit chance and accuracy calculations.

/// Calculate hit chance based on accuracy vs evasion.
///
/// # Formula
///
/// ```text
/// base_chance = 75
/// hit_chance = base_chance + (accuracy - evasion)
/// clamped to [5, 95]
/// ```
///
/// This ensures:
/// - Minimum 5% hit chance (even against high evasion)
/// - Maximum 95% hit chance (never guaranteed)
/// - Linear scaling with stat difference
///
/// # Arguments
///
/// * `accuracy` - Attacker's accuracy stat (from derived stats)
/// * `evasion` - Defender's evasion stat (from derived stats)
///
/// # Returns
///
/// Hit chance as percentage (5-95)
pub fn calculate_hit_chance(accuracy: i32, evasion: i32) -> u32 {
    const BASE_CHANCE: i32 = 75;
    const MIN_CHANCE: u32 = 5;
    const MAX_CHANCE: u32 = 95;

    let stat_diff = accuracy - evasion;
    let hit_chance = BASE_CHANCE + stat_diff;

    hit_chance.clamp(MIN_CHANCE as i32, MAX_CHANCE as i32) as u32
}

/// Check if an attack hits based on accuracy, evasion, and random roll.
///
/// # Arguments
///
/// * `accuracy` - Attacker's accuracy stat
/// * `evasion` - Defender's evasion stat
/// * `roll` - Random roll (0-100)
///
/// # Returns
///
/// `true` if attack hits, `false` if it misses.
pub fn check_hit(accuracy: i32, evasion: i32, roll: u32) -> bool {
    let hit_chance = calculate_hit_chance(accuracy, evasion);
    roll <= hit_chance
}
