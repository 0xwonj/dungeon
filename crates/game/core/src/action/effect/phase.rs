//! Execution phases for effect ordering.

/// Execution phase for effect ordering.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum ExecutionPhase {
    /// Before main effects (buffs, debuffs setup).
    PreEffect = 0,

    /// Main damage/healing phase.
    #[default]
    Primary = 1,

    /// After main effects (lifesteal, on-hit effects).
    PostEffect = 2,

    /// Final effects (stacks, cooldowns, cleanup).
    Finalize = 3,
}
