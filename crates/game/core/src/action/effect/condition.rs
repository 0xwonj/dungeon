//! Conditions for conditional effects.

use crate::state::types::status::StatusEffectKind;
use crate::stats::ResourceKind;

/// Condition for conditional effects.
#[derive(Clone, Debug, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum Condition {
    /// Target's resource is below threshold (percentage).
    TargetResourceBelow {
        resource: ResourceKind,
        percent: u32,
    },

    /// Target's resource is above threshold (percentage).
    TargetResourceAbove {
        resource: ResourceKind,
        percent: u32,
    },

    /// Caster's resource is below threshold (percentage).
    CasterResourceBelow {
        resource: ResourceKind,
        percent: u32,
    },

    /// Caster's resource is above threshold (percentage).
    CasterResourceAbove {
        resource: ResourceKind,
        percent: u32,
    },

    /// Target has status effect.
    TargetHasStatus(StatusEffectKind),

    /// Caster has status effect.
    CasterHasStatus(StatusEffectKind),

    /// Target is behind caster.
    TargetBehind,

    /// Random chance (percentage, 0-100).
    RandomChance(u32),

    /// Previous effect was critical.
    WasCritical,

    /// All conditions must be true.
    And(Vec<Condition>),

    /// Any condition must be true.
    Or(Vec<Condition>),

    /// Condition must be false.
    Not(Box<Condition>),
}
