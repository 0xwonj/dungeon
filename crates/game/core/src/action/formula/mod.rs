//! Formula system for dynamic value calculation.
//!
//! Formulas allow action effects to scale based on:
//! - Character stats (STR, INT, etc.)
//! - Current/max resources (HP, MP, Lucidity)
//! - Previous effects in the same action (damage chains)
//! - Weapon damage
//! - Arithmetic combinations (sum, product, min, max)
//!
//! ## Examples
//!
//! ```ignore
//! // 150% weapon damage
//! Formula::WeaponDamage { percent: 150 }
//!
//! // 50% caster STR + 10 flat
//! Formula::Sum(vec![
//!     Formula::CasterStat { stat: CoreStatKind::Str, percent: 50 },
//!     Formula::Constant(10),
//! ])
//!
//! // 30% of previous damage (damage chain)
//! Formula::FromPreviousDamage { percent: 30 }
//! ```

pub mod evaluate;

pub use evaluate::evaluate;

use crate::stats::{CoreStatKind, ResourceKind};

// ============================================================================
// Formula Definition
// ============================================================================

/// Formula for calculating numeric values.
#[derive(Clone, Debug, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum Formula {
    /// Fixed constant value.
    Constant(u32),

    /// Percentage of caster's stat.
    CasterStat { stat: CoreStatKind, percent: u32 },

    /// Percentage of target's stat.
    TargetStat { stat: CoreStatKind, percent: u32 },

    /// Percentage of caster's current resource.
    CasterResource {
        resource: ResourceKind,
        percent: u32,
    },

    /// Percentage of target's current resource.
    TargetResource {
        resource: ResourceKind,
        percent: u32,
    },

    /// Percentage of target's missing resource (max - current).
    TargetMissingResource {
        resource: ResourceKind,
        percent: u32,
    },

    /// Percentage of target's max resource.
    TargetMaxResource {
        resource: ResourceKind,
        percent: u32,
    },

    /// Percentage of weapon damage.
    WeaponDamage { percent: u32 },

    /// Percentage of damage dealt in previous effects (this action).
    FromPreviousDamage { percent: u32 },

    /// Percentage of healing done in previous effects (this action).
    FromPreviousHealing { percent: u32 },

    /// Sum of multiple formulas.
    Sum(Vec<Formula>),

    /// Product of multiple formulas.
    Product(Vec<Formula>),

    /// Minimum of multiple formulas.
    Min(Vec<Formula>),

    /// Maximum of multiple formulas.
    Max(Vec<Formula>),
}
