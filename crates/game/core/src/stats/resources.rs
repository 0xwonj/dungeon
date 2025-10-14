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

use super::core::CoreEffective;

/// Resource pool with current and maximum values.
///
/// Maximum is computed from stats, current is part of game state.
#[derive(Clone, Debug, PartialEq, Eq, Default)]
pub struct ResourceMeter {
    pub current: u32,
    pub maximum: u32,
}

impl ResourceMeter {
    /// Create a new resource meter
    pub const fn new(current: u32, maximum: u32) -> Self {
        Self { current, maximum }
    }

    /// Create a full resource meter
    pub const fn full(maximum: u32) -> Self {
        Self {
            current: maximum,
            maximum,
        }
    }

    /// Check if depleted (current = 0)
    pub const fn is_depleted(&self) -> bool {
        self.current == 0
    }

    /// Check if full (current = maximum)
    pub const fn is_full(&self) -> bool {
        self.current >= self.maximum
    }

    /// Get percentage (0-100)
    pub fn percent(&self) -> u32 {
        if self.maximum == 0 {
            return 0;
        }
        (self.current * 100) / self.maximum
    }
}

/// Maximum resource values computed from stats.
///
/// These are NOT stored - always recomputed from CoreEffective.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ResourceMaximums {
    pub hp_max: u32,
    pub mp_max: u32,
    pub lucidity_max: u32,
}

impl ResourceMaximums {
    /// Compute maximum resource values from CoreEffective
    ///
    /// Formulas:
    /// - HP_max = (CON × 10) + (Level × CON / 2)
    /// - MP_max = (WIL + INT) × 5 + (EGO × 2) + (Level × √WIL)
    /// - Lucidity_max = √Level × ((STR+DEX+CON)/2 + (INT+WIL+EGO)×2)
    pub fn compute(core: &CoreEffective) -> Self {
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

/// Current resource values (game state, must be stored).
///
/// This is the only part of the resource system that is persisted.
#[derive(Clone, Debug, PartialEq, Eq)]
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

    /// Combine current values with maximums to create meters
    pub fn to_meters(&self, max: &ResourceMaximums) -> ResourceMeters {
        ResourceMeters {
            hp: ResourceMeter::new(self.hp, max.hp_max),
            mp: ResourceMeter::new(self.mp, max.mp_max),
            lucidity: ResourceMeter::new(self.lucidity, max.lucidity_max),
        }
    }
}

/// Complete resource pools (current + maximum).
///
/// This combines current (stored) and maximum (computed) values.
#[derive(Clone, Debug, PartialEq, Eq, Default)]
pub struct ResourceMeters {
    pub hp: ResourceMeter,
    pub mp: ResourceMeter,
    pub lucidity: ResourceMeter,
}

impl ResourceMeters {
    /// Create from current and maximum values
    pub fn new(current: &ResourceCurrent, max: &ResourceMaximums) -> Self {
        current.to_meters(max)
    }

    /// Check if HP is depleted (dead/unconscious)
    pub fn is_hp_depleted(&self) -> bool {
        self.hp.is_depleted()
    }

    /// Check if MP is depleted
    pub fn is_mp_depleted(&self) -> bool {
        self.mp.is_depleted()
    }

    /// Check if Lucidity is depleted
    pub fn is_lucidity_depleted(&self) -> bool {
        self.lucidity.is_depleted()
    }
}
