use std::fmt;

/// Unique identifier for any entity tracked in the state.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct EntityId(pub u32);

impl EntityId {
    /// Reserved identifier for the controllable player character.
    pub const PLAYER: Self = Self(0);
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
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
pub struct Tick(pub u64);

impl Tick {
    pub const ZERO: Self = Self(0);

    pub fn new(value: u64) -> Self {
        Self(value)
    }
}

impl std::ops::Add<u64> for Tick {
    type Output = Tick;
    fn add(self, rhs: u64) -> Tick {
        Tick(self.0 + rhs)
    }
}

impl std::ops::Sub<u64> for Tick {
    type Output = Tick;
    fn sub(self, rhs: u64) -> Tick {
        Tick(self.0 - rhs)
    }
}

impl fmt::Display for Tick {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// Integer resource meter (e.g., health, stamina) tracked per actor.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub struct ResourceMeter {
    pub current: u32,
    pub maximum: u32,
}

impl ResourceMeter {
    pub fn new(current: u32, maximum: u32) -> Self {
        Self { current, maximum }
    }
}
