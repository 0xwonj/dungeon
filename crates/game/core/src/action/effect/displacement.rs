//! Displacement modes for movement effects.

/// How to determine displacement for movement effects.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum Displacement {
    /// Move in direction specified by ActionInput::Direction.
    FromInput { distance: u32 },

    /// Move toward target entity.
    TowardTarget { distance: u32 },

    /// Move away from target entity.
    AwayFromTarget { distance: u32 },

    /// Move away from caster (knockback).
    AwayFromCaster { distance: u32 },

    /// Teleport to position specified by ActionInput::Position.
    ToInputPosition,

    /// Teleport to random valid position within range.
    RandomInRange { range: u32 },
}
