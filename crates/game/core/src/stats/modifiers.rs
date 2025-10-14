//! Modifiers - Layer 4 of the stat system.
//!
//! Roll modifiers for skill checks and attribute tests, derived from CoreEffective.
//! These are the bonuses added to d20-style rolls for success/failure determination.
//!
//! Formula: modifier = floor((CoreEffective - 10) / 2) + Flat + %Inc

use super::bonus::{Bonus, BonusStack};
use super::core::CoreEffective;

/// Roll modifiers derived from core stats.
///
/// These are NOT stored - computed on-demand from CoreEffective.
/// Used in d20-style rolls: `d20 + modifier vs DC`
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct StatModifiers {
    pub str_mod: i32,
    pub con_mod: i32,
    pub dex_mod: i32,
    pub int_mod: i32,
    pub wil_mod: i32,
    pub ego_mod: i32,
}

impl StatModifiers {
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
    pub fn base_modifier(stat: i32) -> i32 {
        (stat - 10) / 2
    }

    /// Compute modifiers from CoreEffective stats
    pub fn compute(core: &CoreEffective) -> Self {
        Self {
            str_mod: Self::base_modifier(core.str),
            con_mod: Self::base_modifier(core.con),
            dex_mod: Self::base_modifier(core.dex),
            int_mod: Self::base_modifier(core.int),
            wil_mod: Self::base_modifier(core.wil),
            ego_mod: Self::base_modifier(core.ego),
        }
    }
}

/// Additional bonuses that can be applied to modifiers.
///
/// These represent situational bonuses (skill ranks, equipment, etc.)
/// that don't come from core stats.
#[derive(Clone, Debug, Default)]
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

/// Final modifiers after applying additional bonuses.
///
/// These are the actual values added to rolls.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FinalModifiers {
    pub str_mod: i32,
    pub con_mod: i32,
    pub dex_mod: i32,
    pub int_mod: i32,
    pub wil_mod: i32,
    pub ego_mod: i32,
}

impl FinalModifiers {
    /// Compute final modifiers from core stats and additional bonuses
    ///
    /// Formula per stat: base_modifier + bonuses (unclamped)
    pub fn compute(core: &CoreEffective, bonuses: &ModifierBonuses) -> Self {
        let base = StatModifiers::compute(core);

        Self {
            str_mod: bonuses.str_bonuses.apply_unclamped(base.str_mod),
            con_mod: bonuses.con_bonuses.apply_unclamped(base.con_mod),
            dex_mod: bonuses.dex_bonuses.apply_unclamped(base.dex_mod),
            int_mod: bonuses.int_bonuses.apply_unclamped(base.int_mod),
            wil_mod: bonuses.wil_bonuses.apply_unclamped(base.wil_mod),
            ego_mod: bonuses.ego_bonuses.apply_unclamped(base.ego_mod),
        }
    }

    /// Compute with no additional bonuses
    pub fn from_core(core: &CoreEffective) -> Self {
        Self::compute(core, &ModifierBonuses::new())
    }
}
