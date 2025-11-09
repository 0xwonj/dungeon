//! Resources - Layer 5 of the stat system.
//!
//! Resource pools (HP, MP, Lucidity) are partially stored:
//! - Maximum values: Computed from CoreEffective (NOT stored)
//! - Current values: Game state (MUST be stored)
//!
//! Formulas:
//! - HP_max = (CON × 10) + (Level × CON / 2)
//! - MP_max = (WIL + INT) × 5 + (EGO × 2) + (Level × √WIL)
//! - Lucidity_max = √Level × ((STR+DEX+CON)/2 + (INT+WIL+EGO)×2)

use super::bonus::{BonusStack, StatBounds, StatLayer};
use super::core::CoreEffective;

// ============================================================================
// Resource Kind (for formulas and references)
// ============================================================================

/// Enum representing individual resource types.
///
/// Used in formulas, effect systems, and resource costs to reference specific resources.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum ResourceKind {
    /// Health points.
    Hp,
    /// Magic points (mana).
    Mp,
    /// Action resource (lucidity) - consumed by actions.
    Lucidity,
}

/// Maximum resource values computed from stats.
///
/// These are NOT stored - always recomputed from CoreEffective.
#[derive(Clone, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct ResourceMaximums {
    pub hp_max: u32,
    pub mp_max: u32,
    pub lucidity_max: u32,
}

impl ResourceMaximums {
    /// Get the max value for a specific resource.
    pub fn get(&self, resource: ResourceKind) -> u32 {
        match resource {
            ResourceKind::Hp => self.hp_max,
            ResourceKind::Mp => self.mp_max,
            ResourceKind::Lucidity => self.lucidity_max,
        }
    }

    /// Compute maximum resource values from CoreEffective (internal helper)
    ///
    /// Formulas:
    /// - HP_max = (CON × 10) + (Level × CON / 2)
    /// - MP_max = (WIL + INT) × 5 + (EGO × 2) + (Level × √WIL)
    /// - Lucidity_max = √Level × ((STR+DEX+CON)/2 + (INT+WIL+EGO)×2)
    fn compute_base(core: &CoreEffective) -> Self {
        let hp_max = Self::compute_hp(core);
        let mp_max = Self::compute_mp(core);
        let lucidity_max = Self::compute_lucidity(core);

        Self {
            hp_max,
            mp_max,
            lucidity_max,
        }
    }

    /// Compute HP maximum
    ///
    /// Formula: (CON × 10) + (Level × CON / 2)
    pub fn compute_hp(core: &CoreEffective) -> u32 {
        let base = core.con * 10;
        let per_level = (core.level * core.con) / 2;
        (base + per_level).max(1) as u32
    }

    /// Compute MP maximum
    ///
    /// Formula: (WIL + INT) × 5 + (EGO × 2) + (Level × √WIL)
    pub fn compute_mp(core: &CoreEffective) -> u32 {
        let base = (core.wil + core.int) * 5;
        let ego_bonus = core.ego * 2;
        let level_bonus = core.level * Self::integer_sqrt(core.wil as u32) as i32;
        (base + ego_bonus + level_bonus).max(0) as u32
    }

    /// Compute Lucidity maximum
    ///
    /// Formula: √Level × ((STR+DEX+CON)/2 + (INT+WIL+EGO)×2)
    pub fn compute_lucidity(core: &CoreEffective) -> u32 {
        let level_sqrt = Self::integer_sqrt(core.level as u32) as i32;
        let physical = (core.str + core.dex + core.con) / 2;
        let mental = (core.int + core.wil + core.ego) * 2;
        (level_sqrt * (physical + mental)).max(1) as u32
    }

    /// Integer square root (for determinism, no floating point)
    fn integer_sqrt(n: u32) -> u32 {
        if n == 0 {
            return 0;
        }
        let mut x = n;
        let mut y = x.div_ceil(2);
        while y < x {
            x = y;
            y = (x + n / x).div_ceil(2);
        }
        x
    }
}

/// Bonuses that apply to resource maximums.
///
/// Sources: equipment (+HP armor), buffs (+30% Max MP), class features, etc.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct ResourceBonuses {
    pub hp_max: BonusStack,
    pub mp_max: BonusStack,
    pub lucidity_max: BonusStack,
}

impl ResourceBonuses {
    /// Create new empty resource bonuses
    pub fn new() -> Self {
        Self::default()
    }
}

/// Layer 5: Resource Maximums Layer
///
/// Base: CoreEffective (output from Layer 1)
/// Bonuses: ResourceBonuses (from equipment, buffs, etc.)
/// Final: ResourceMaximums (max HP/MP/Lucidity)
impl StatLayer for ResourceMaximums {
    type Base = CoreEffective;
    type Bonuses = ResourceBonuses;
    type Final = Self;

    fn compute(base: &Self::Base, bonuses: &Self::Bonuses) -> Self::Final {
        const BOUNDS: StatBounds = StatBounds::RESOURCE_MAX;

        let base_resources = Self::compute_base(base);

        Self {
            hp_max: bonuses
                .hp_max
                .apply(base_resources.hp_max as i32, BOUNDS.min, BOUNDS.max)
                as u32,
            mp_max: bonuses
                .mp_max
                .apply(base_resources.mp_max as i32, BOUNDS.min, BOUNDS.max)
                as u32,
            lucidity_max: bonuses.lucidity_max.apply(
                base_resources.lucidity_max as i32,
                BOUNDS.min,
                BOUNDS.max,
            ) as u32,
        }
    }

    fn empty_bonuses() -> Self::Bonuses {
        ResourceBonuses::new()
    }
}

/// Current resource values (game state, must be stored).
///
/// This is the only part of the resource system that is persisted.
#[derive(Clone, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct ResourceCurrent {
    pub hp: u32,
    pub mp: u32,
    pub lucidity: u32,
}

impl ResourceCurrent {
    /// Create new current resource values
    pub const fn new(hp: u32, mp: u32, lucidity: u32) -> Self {
        Self { hp, mp, lucidity }
    }

    /// Create current resources at maximum (from maximums)
    pub const fn at_max(max: &ResourceMaximums) -> Self {
        Self {
            hp: max.hp_max,
            mp: max.mp_max,
            lucidity: max.lucidity_max,
        }
    }
}
