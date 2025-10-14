use bitflags::bitflags;

bitflags! {
    /// Tracks which fields of an [`ActorState`] changed during a state transition.
    ///
    /// Each bit represents a single field in the actor structure. Using bitflags
    /// provides O(1) set/check operations and minimal memory footprint (~1 byte).
    #[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
    pub struct ActorFields: u8 {
        const POSITION    = 1 << 0;
        const CORE_STATS  = 1 << 1;
        const RESOURCES   = 1 << 2;
        const BONUSES     = 1 << 3;
        const INVENTORY   = 1 << 4;
        const READY_AT    = 1 << 5;
    }
}

bitflags! {
    /// Tracks which fields of a [`PropState`] changed during a state transition.
    #[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
    pub struct PropFields: u8 {
        const POSITION  = 1 << 0;
        const IS_ACTIVE = 1 << 1;
    }
}

bitflags! {
    /// Tracks which fields of an [`ItemState`] changed during a state transition.
    #[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
    pub struct ItemFields: u8 {
        const POSITION = 1 << 0;
    }
}

bitflags! {
    /// Tracks which fields of [`TurnState`] changed during a state transition.
    #[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
    pub struct TurnFields: u8 {
        const CLOCK         = 1 << 0;
        const CURRENT_ACTOR = 1 << 1;
    }
}
