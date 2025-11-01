//! Derived Stats - Layer 2 of the stat system.
//!
//! Combat and action resolution stats derived from CoreEffective.
//! These are NOT stored - always recomputed from core stats when needed.
//!
//! Components: Attack, Accuracy, Evasion, AC, PsiPower, FocusEff

use super::bonus::{BonusStack, StatBounds, StatLayer};
use super::core::CoreEffective;

/// Derived combat statistics.
///
/// These are pure functions of CoreEffective + equipment/buff bonuses.
/// NOT stored - recomputed at action initiation.
#[derive(Clone, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
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
    /// Compute base derived stats from CoreEffective (internal helper)
    ///
    /// Base formulas (before bonuses):
    /// - Attack: STR × 1.5
    /// - Accuracy: DEX
    /// - Evasion: DEX × 0.5
    /// - AC: 10 + (DEX-10)/2
    /// - PsiPower: INT × 0.8 + EGO × 0.5
    /// - FocusEff: WIL × 1.2
    fn compute_base(core: &CoreEffective) -> Self {
        Self {
            attack: (core.str * 15) / 10,
            accuracy: core.dex,
            evasion: (core.dex * 5) / 10,
            ac: 10 + (core.dex - 10) / 2,
            psi_power: (core.int * 8) / 10 + (core.ego * 5) / 10,
            focus_eff: (core.wil * 12) / 10,
        }
    }
}

/// Bonuses that apply to derived stats.
///
/// Sources: equipment, buffs, environmental effects, etc.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
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

/// Layer 2: Derived Stats Layer
///
/// Base: CoreEffective (output from Layer 1)
/// Bonuses: DerivedBonuses (from equipment, skills, etc.)
/// Final: DerivedStats (combat stats)
impl StatLayer for DerivedStats {
    type Base = CoreEffective;
    type Bonuses = DerivedBonuses;
    type Final = Self;

    fn compute(base: &Self::Base, bonuses: &Self::Bonuses) -> Self::Final {
        const BOUNDS: StatBounds = StatBounds::DERIVED;

        let base_stats = Self::compute_base(base);

        Self {
            attack: bonuses
                .attack
                .apply(base_stats.attack, BOUNDS.min, BOUNDS.max),
            accuracy: bonuses
                .accuracy
                .apply(base_stats.accuracy, BOUNDS.min, BOUNDS.max),
            evasion: bonuses
                .evasion
                .apply(base_stats.evasion, BOUNDS.min, BOUNDS.max),
            ac: bonuses.ac.apply(base_stats.ac, BOUNDS.min, BOUNDS.max),
            psi_power: bonuses
                .psi_power
                .apply(base_stats.psi_power, BOUNDS.min, BOUNDS.max),
            focus_eff: bonuses
                .focus_eff
                .apply(base_stats.focus_eff, BOUNDS.min, BOUNDS.max),
        }
    }

    fn empty_bonuses() -> Self::Bonuses {
        DerivedBonuses::new()
    }
}
