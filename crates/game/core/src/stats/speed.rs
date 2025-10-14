//! Speed and Cost - Layer 3 of the stat system.
//!
//! Speed values determine how quickly actions execute on the timeline.
//! Action costs are computed from base costs and speed values.
//!
//! Formulas:
//! - SpeedKind = base + weighted(CoreEffective) - penalties
//! - final_cost = base_cost × Conditions × 100 / clamp(SpeedKind, 50, 200)

use super::bonus::{BonusStack, StatBounds, StatLayer};
use super::core::CoreEffective;

/// Speed statistics for different action types.
///
/// Higher speed = faster action execution = lower action cost.
/// Base speed is 100, modified by core stats and conditions.
#[derive(Clone, Debug, PartialEq, Eq)]
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
    /// - Physical: 100 + DEX × 0.8 + STR × 0.2 - ArmorPenalty
    /// - Cognitive: 100 + INT × 0.6 + WIL × 0.4
    /// - Ritual: 100 + WIL × 0.5 + EGO × 0.5
    fn compute_base(core: &CoreEffective) -> Self {
        let physical = 100 + (core.dex * 8 / 10) + (core.str * 2 / 10);
        let cognitive = 100 + (core.int * 6 / 10) + (core.wil * 4 / 10);
        let ritual = 100 + (core.wil * 5 / 10) + (core.ego * 5 / 10);

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
/// Formula: final_cost = base_cost × condition_multiplier × 100 / clamp(speed, 50, 200)
///
/// # Arguments
/// * `base_cost` - The base cost of the action (e.g., 100 for normal action)
/// * `speed` - The speed value (already includes conditions, clamped to [50, 200])
/// * `condition_multiplier` - Additional cost multiplier from conditions (100 = no change)
///
/// # Returns
/// The final cost in timeline units
///
/// # Examples
/// - Normal action (cost 100, speed 100): 100 × 100 / 100 = 100
/// - Fast action (cost 100, speed 200): 100 × 100 / 200 = 50
/// - Slow action (cost 100, speed 50): 100 × 100 / 50 = 200
pub fn calculate_action_cost(base_cost: i32, speed: i32, condition_multiplier: i32) -> i32 {
    const MIN_SPEED: i32 = 50;
    const MAX_SPEED: i32 = 200;

    let clamped_speed = speed.clamp(MIN_SPEED, MAX_SPEED);
    (base_cost * condition_multiplier * 100) / (clamped_speed * 100)
}
