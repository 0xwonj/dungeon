//! Modifiers - Layer 4 of the stat system.
//!
//! Roll modifiers for skill checks and attribute tests, derived from CoreEffective.
//! These are the bonuses added to d20-style rolls for success/failure determination.
//!
//! Formula: modifier = floor((CoreEffective - 10) / 2) + Flat + %Inc

use super::bonus::{Bonus, BonusStack, StatBounds, StatLayer};
use super::core::CoreEffective;

/// Additional bonuses that can be applied to modifiers.
///
/// These represent situational bonuses (skill ranks, equipment, etc.)
/// that don't come from core stats.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct ModifierBonuses {
    pub str_bonuses: BonusStack,
    pub con_bonuses: BonusStack,
    pub dex_bonuses: BonusStack,
    pub int_bonuses: BonusStack,
    pub wil_bonuses: BonusStack,
    pub ego_bonuses: BonusStack,
}

impl ModifierBonuses {
    /// Create new empty modifier bonuses
    pub fn new() -> Self {
        Self::default()
    }

    /// Add a bonus to STR modifier
    pub fn add_str(&mut self, bonus: Bonus) {
        self.str_bonuses.add(bonus);
    }

    /// Add a bonus to CON modifier
    pub fn add_con(&mut self, bonus: Bonus) {
        self.con_bonuses.add(bonus);
    }

    /// Add a bonus to DEX modifier
    pub fn add_dex(&mut self, bonus: Bonus) {
        self.dex_bonuses.add(bonus);
    }

    /// Add a bonus to INT modifier
    pub fn add_int(&mut self, bonus: Bonus) {
        self.int_bonuses.add(bonus);
    }

    /// Add a bonus to WIL modifier
    pub fn add_wil(&mut self, bonus: Bonus) {
        self.wil_bonuses.add(bonus);
    }

    /// Add a bonus to EGO modifier
    pub fn add_ego(&mut self, bonus: Bonus) {
        self.ego_bonuses.add(bonus);
    }
}

/// Roll modifiers after applying bonuses.
///
/// These are the actual values added to rolls.
/// Used in d20-style rolls: `d20 + modifier vs DC`
#[derive(Clone, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct StatModifiers {
    pub str_mod: i32,
    pub con_mod: i32,
    pub dex_mod: i32,
    pub int_mod: i32,
    pub wil_mod: i32,
    pub ego_mod: i32,
}

impl StatModifiers {
    fn compute_base(core: &CoreEffective) -> Self {
        Self {
            str_mod: (Self::base_modifier(core.str)),
            con_mod: (Self::base_modifier(core.con)),
            dex_mod: (Self::base_modifier(core.dex)),
            int_mod: (Self::base_modifier(core.int)),
            wil_mod: (Self::base_modifier(core.wil)),
            ego_mod: (Self::base_modifier(core.ego)),
        }
    }

    /// Calculate base modifier from a stat value (D&D formula)
    ///
    /// Formula: floor((stat - 10) / 2)
    ///
    /// Examples:
    /// - 10-11 → +0
    /// - 12-13 → +1
    /// - 8-9 → -1
    /// - 20 → +5
    /// - 1 → -4
    fn base_modifier(stat: i32) -> i32 {
        (stat - 10) / 2
    }
}

/// Layer 4: Modifiers Layer
///
/// Base: CoreEffective (output from Layer 1)
/// Bonuses: ModifierBonuses (from skills, situational effects)
/// Final: StatModifiers (d20 roll modifiers)
impl StatLayer for StatModifiers {
    type Base = CoreEffective;
    type Bonuses = ModifierBonuses;
    type Final = Self;

    fn compute(base: &Self::Base, bonuses: &Self::Bonuses) -> Self::Final {
        const BOUNDS: StatBounds = StatBounds::MODIFIER;

        let base_stats = Self::compute_base(base);

        Self {
            str_mod: bonuses
                .str_bonuses
                .apply(base_stats.str_mod, BOUNDS.min, BOUNDS.max),
            con_mod: bonuses
                .con_bonuses
                .apply(base_stats.con_mod, BOUNDS.min, BOUNDS.max),
            dex_mod: bonuses
                .dex_bonuses
                .apply(base_stats.dex_mod, BOUNDS.min, BOUNDS.max),
            int_mod: bonuses
                .int_bonuses
                .apply(base_stats.int_mod, BOUNDS.min, BOUNDS.max),
            wil_mod: bonuses
                .wil_bonuses
                .apply(base_stats.wil_mod, BOUNDS.min, BOUNDS.max),
            ego_mod: bonuses
                .ego_bonuses
                .apply(base_stats.ego_mod, BOUNDS.min, BOUNDS.max),
        }
    }

    fn empty_bonuses() -> Self::Bonuses {
        ModifierBonuses::new()
    }
}
