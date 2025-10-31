//! Speed and Cost - Layer 3 of the stat system.
//!
//! Speed values determine how quickly actions execute on the timeline.
//! Action costs are computed from base costs and speed values.
//!
//! Formulas:
//! - SpeedKind = 20 + weighted(CoreEffective)
//! - final_cost = base_cost × 100 / clamp(SpeedKind, 1, 10000)

use super::bonus::{BonusStack, StatBounds, StatLayer};
use super::core::CoreEffective;

/// Speed statistics for different action types.
///
/// Higher speed = faster action execution = lower action cost.
/// Base speed is 100, modified by core stats and conditions.
#[derive(Clone, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct SpeedStats {
    /// Physical action speed (movement, melee, physical skills)
    pub physical: i32,

    /// Cognitive action speed (problem-solving, analysis, mental skills)
    pub cognitive: i32,

    /// Ritual action speed (spellcasting, rituals, channeling)
    pub ritual: i32,
}

impl SpeedStats {
    /// Compute base speed stats from CoreEffective (internal helper)
    ///
    /// Formulas:
    /// - Physical: 20 + DEX × 4 + STR × 1
    /// - Cognitive: 20 + INT × 3 + WIL × 2
    /// - Ritual: 20 + WIL × 2.5 + EGO × 2.5
    fn compute_base(core: &CoreEffective) -> Self {
        let physical = 20 + (core.dex * 4) + core.str;
        let cognitive = 20 + (core.int * 3) + (core.wil * 2);
        let ritual = 20 + (core.wil * 5 / 2) + (core.ego * 5 / 2);

        Self {
            physical,
            cognitive,
            ritual,
        }
    }
}

/// Bonus modifiers for speed values.
///
/// Represents effects like Haste, Slow, Stun, etc.
/// Applied as final multipliers to base speed values.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct SpeedBonuses {
    pub physical: BonusStack,
    pub cognitive: BonusStack,
    pub ritual: BonusStack,
}

impl SpeedBonuses {
    /// Create new empty bonus set
    pub fn new() -> Self {
        Self::default()
    }
}

/// Layer 3: Speed Stats Layer
///
/// Base: CoreEffective (output from Layer 1)
/// Bonuses: SpeedBonuses (from buffs, debuffs, conditions)
/// Final: SpeedStats (action speed values)
impl StatLayer for SpeedStats {
    type Base = CoreEffective;
    type Bonuses = SpeedBonuses;
    type Final = Self;

    fn compute(base: &Self::Base, bonuses: &Self::Bonuses) -> Self::Final {
        let bounds = StatBounds::SPEED_STATS;
        let base_speed = Self::compute_base(base);

        Self {
            physical: bonuses
                .physical
                .apply(base_speed.physical, bounds.min, bounds.max),
            cognitive: bonuses
                .cognitive
                .apply(base_speed.cognitive, bounds.min, bounds.max),
            ritual: bonuses
                .ritual
                .apply(base_speed.ritual, bounds.min, bounds.max),
        }
    }

    fn empty_bonuses() -> Self::Bonuses {
        SpeedBonuses::new()
    }

    fn bounds() -> Option<StatBounds> {
        Some(StatBounds::SPEED_STATS)
    }
}

/// The kind of speed to use for an action
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum SpeedKind {
    Physical,
    Cognitive,
    Ritual,
}

impl SpeedKind {
    /// Get the speed value from SpeedStats for this kind
    pub fn get_speed(&self, stats: &SpeedStats) -> i32 {
        match self {
            SpeedKind::Physical => stats.physical,
            SpeedKind::Cognitive => stats.cognitive,
            SpeedKind::Ritual => stats.ritual,
        }
    }
}

/// Calculate the final action cost from base cost and speed.
///
/// Formula: final_cost = base_cost × 100 / clamp(speed, 50, 200)
///
/// Speed is already computed from core stats + bonuses (including status effects),
/// so this function only handles the base_cost → final_cost transformation.
///
/// # Arguments
/// * `base_cost` - The base cost of the action (e.g., 10 for movement)
/// * `speed` - The speed value from snapshot (already includes bonuses/effects)
///
/// # Returns
/// The final cost in timeline ticks
pub fn calculate_action_cost(base_cost: u64, speed: i32) -> u64 {
    const MIN_SPEED: i32 = 1;
    const MAX_SPEED: i32 = 10000;

    let clamped_speed = speed.clamp(MIN_SPEED, MAX_SPEED).max(1) as u64;
    (base_cost * 100) / clamped_speed
}
