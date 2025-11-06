//! Interaction types for world objects.

/// Type of interaction with world objects.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum InteractionType {
    Open,
    Close,
    PickUp,
    Use,
    Talk,
}
