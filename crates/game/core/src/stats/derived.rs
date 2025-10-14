//! Derived Stats - Layer 2 of the stat system.
//!
//! Combat and action resolution stats derived from CoreEffective.
//! These are NOT stored - always recomputed from core stats when needed.
//!
//! Components: Attack, Accuracy, Evasion, AC, PsiPower, FocusEff

use super::bonus::BonusStack;
use super::core::CoreEffective;

/// Derived combat statistics.
///
/// These are pure functions of CoreEffective + equipment/buff bonuses.
/// NOT stored - recomputed at action initiation.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DerivedStats {
    /// Physical attack power (damage scaling)
    pub attack: i32,

    /// Attack accuracy (hit chance)
    pub accuracy: i32,

    /// Evasion (dodge chance)
    pub evasion: i32,

    /// Armor Class (damage reduction)
    pub ac: i32,

    /// Psionic/spell power (magic damage scaling)
    pub psi_power: i32,

    /// Focus efficiency (spell cost reduction / effectiveness)
    pub focus_eff: i32,
}

impl DerivedStats {
    /// Compute derived stats from CoreEffective
    ///
    /// Base formulas (before bonuses):
    /// - Attack: STR × 1.5
    /// - Accuracy: DEX
    /// - Evasion: DEX × 0.5
    /// - AC: 10 + (DEX-10)/2
    /// - PsiPower: INT × 0.8 + EGO × 0.5
    /// - FocusEff: WIL × 1.2
    pub fn compute_base(core: &CoreEffective) -> Self {
        Self {
            attack: (core.str * 15) / 10,
            accuracy: core.dex,
            evasion: (core.dex * 5) / 10,
            ac: 10 + (core.dex - 10) / 2,
            psi_power: (core.int * 8) / 10 + (core.ego * 5) / 10,
            focus_eff: (core.wil * 12) / 10,
        }
    }

    /// Apply bonuses to derived stats
    pub fn apply_bonuses(&self, bonuses: &DerivedBonuses) -> Self {
        Self {
            attack: bonuses.attack.apply_unclamped(self.attack),
            accuracy: bonuses.accuracy.apply_unclamped(self.accuracy),
            evasion: bonuses.evasion.apply_unclamped(self.evasion),
            ac: bonuses.ac.apply_unclamped(self.ac),
            psi_power: bonuses.psi_power.apply_unclamped(self.psi_power),
            focus_eff: bonuses.focus_eff.apply_unclamped(self.focus_eff),
        }
    }

    /// Compute derived stats with bonuses
    pub fn compute(core: &CoreEffective, bonuses: &DerivedBonuses) -> Self {
        let base = Self::compute_base(core);
        base.apply_bonuses(bonuses)
    }
}

/// Bonuses that apply to derived stats.
///
/// Sources: equipment, buffs, environmental effects, etc.
#[derive(Clone, Debug, Default)]
pub struct DerivedBonuses {
    pub attack: BonusStack,
    pub accuracy: BonusStack,
    pub evasion: BonusStack,
    pub ac: BonusStack,
    pub psi_power: BonusStack,
    pub focus_eff: BonusStack,
}

impl DerivedBonuses {
    /// Create new empty derived bonuses
    pub fn new() -> Self {
        Self::default()
    }
}
