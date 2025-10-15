use std::fmt;

/// Unique identifier for any entity tracked in the state.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct EntityId(pub u32);

impl EntityId {
    /// Reserved identifier for the controllable player character.
    pub const PLAYER: Self = Self(0);

    /// Reserved identifier for system-level actions (turn scheduling, hooks, etc.).
    ///
    /// System actions are deterministic state transitions that maintain game invariants
    /// but are not initiated by any in-game entity. Examples include turn preparation,
    /// action cost application, and entity activation updates.
    pub const SYSTEM: Self = Self(u32::MAX);

    /// Returns true if this entity represents a system actor.
    #[inline]
    pub const fn is_system(self) -> bool {
        self.0 == Self::SYSTEM.0
    }

    /// Returns true if this entity represents the player.
    #[inline]
    pub const fn is_player(self) -> bool {
        self.0 == Self::PLAYER.0
    }
}

impl Default for EntityId {
    fn default() -> Self {
        Self::PLAYER
    }
}

impl fmt::Display for EntityId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "#{}", self.0)
    }
}

/// Discrete grid position expressed in tile coordinates.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Position {
    pub x: i32,
    pub y: i32,
}

impl Position {
    pub const ORIGIN: Self = Self { x: 0, y: 0 };

    pub fn new(x: i32, y: i32) -> Self {
        Self { x, y }
    }
}

impl Default for Position {
    fn default() -> Self {
        Self::ORIGIN
    }
}

/// Discrete time unit in the timeline-based scheduling system.
pub type Tick = u64;
