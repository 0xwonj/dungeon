//! Core Stats - Layer 1 of the stat system.
//!
//! Core stats (STR, DEX, CON, INT, WIL, EGO, Level) are the Single Source of Truth (SSOT)
//! and the only stats that are permanently stored. All other stats are derived from these.
//!
//! CoreEffective = (Base + Flat) × (1 + %Inc) × More × Less × Clamp

use super::bonus::{Bonus, BonusStack};

/// The six core attributes that define a character.
///
/// These are permanently stored and form the foundation for all calculations.
/// - **STR** (Strength): Physical power, melee damage, carrying capacity
/// - **CON** (Constitution): Health, stamina, physical resilience
/// - **DEX** (Dexterity): Physical speed, evasion, accuracy
/// - **INT** (Intelligence): Cognitive speed, learning, problem-solving
/// - **WIL** (Willpower): Mental fortitude, spellcasting, focus
/// - **EGO** (Ego): Force of personality, ritual power, critical strikes
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CoreStats {
    pub str: i32,
    pub con: i32,
    pub dex: i32,
    pub int: i32,
    pub wil: i32,
    pub ego: i32,
    pub level: i32,
}

impl CoreStats {
    /// Create new core stats with specified values
    pub fn new(str: i32, con: i32, dex: i32, int: i32, wil: i32, ego: i32, level: i32) -> Self {
        Self {
            str,
            con,
            dex,
            int,
            wil,
            ego,
            level,
        }
    }
}

impl Default for CoreStats {
    /// Default stats: all 10 (average human), level 1
    fn default() -> Self {
        Self {
            str: 10,
            con: 10,
            dex: 10,
            int: 10,
            wil: 10,
            ego: 10,
            level: 1,
        }
    }
}

/// Bonuses that apply to core stats from equipment, buffs, conditions.
///
/// These are NOT stored - they are computed from the game state
/// (equipped items, active buffs, environmental effects, etc.)
#[derive(Clone, Debug, Default)]
pub struct CoreStatBonuses {
    pub str_bonuses: BonusStack,
    pub con_bonuses: BonusStack,
    pub dex_bonuses: BonusStack,
    pub int_bonuses: BonusStack,
    pub wil_bonuses: BonusStack,
    pub ego_bonuses: BonusStack,
}

impl CoreStatBonuses {
    /// Create new empty bonus collection
    pub fn new() -> Self {
        Self::default()
    }

    /// Add a flat bonus to STR
    pub fn add_str(&mut self, bonus: Bonus) {
        self.str_bonuses.add(bonus);
    }

    /// Add a flat bonus to CON
    pub fn add_con(&mut self, bonus: Bonus) {
        self.con_bonuses.add(bonus);
    }

    /// Add a flat bonus to DEX
    pub fn add_dex(&mut self, bonus: Bonus) {
        self.dex_bonuses.add(bonus);
    }

    /// Add a flat bonus to INT
    pub fn add_int(&mut self, bonus: Bonus) {
        self.int_bonuses.add(bonus);
    }

    /// Add a flat bonus to WIL
    pub fn add_wil(&mut self, bonus: Bonus) {
        self.wil_bonuses.add(bonus);
    }

    /// Add a flat bonus to EGO
    pub fn add_ego(&mut self, bonus: Bonus) {
        self.ego_bonuses.add(bonus);
    }
}

/// CoreEffective - the result of applying bonuses to base stats.
///
/// This is Layer 1's output and serves as input for all other layers.
/// It is NEVER stored - always recomputed when needed.
///
/// Formula: CoreEffective = (Base + Flat) × (1 + %Inc) × More × Less × Clamp
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CoreEffective {
    pub str: i32,
    pub con: i32,
    pub dex: i32,
    pub int: i32,
    pub wil: i32,
    pub ego: i32,
    pub level: i32,
}

impl CoreEffective {
    /// Compute CoreEffective from base stats and bonuses
    ///
    /// Stats are clamped to [1, 99] range to prevent edge cases.
    pub fn compute(base: &CoreStats, bonuses: &CoreStatBonuses) -> Self {
        const MIN_STAT: i32 = 1;
        const MAX_STAT: i32 = 99;

        Self {
            str: bonuses.str_bonuses.apply(base.str, MIN_STAT, MAX_STAT),
            con: bonuses.con_bonuses.apply(base.con, MIN_STAT, MAX_STAT),
            dex: bonuses.dex_bonuses.apply(base.dex, MIN_STAT, MAX_STAT),
            int: bonuses.int_bonuses.apply(base.int, MIN_STAT, MAX_STAT),
            wil: bonuses.wil_bonuses.apply(base.wil, MIN_STAT, MAX_STAT),
            ego: bonuses.ego_bonuses.apply(base.ego, MIN_STAT, MAX_STAT),
            level: base.level, // Level is not affected by bonuses
        }
    }

    /// Compute with no bonuses (base stats only)
    pub fn from_base(base: &CoreStats) -> Self {
        Self::compute(base, &CoreStatBonuses::new())
    }
}
