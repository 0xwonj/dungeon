use crate::state::EntityId;

/// Performs an interaction with a nearby prop or entity.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct InteractAction {
    pub target: EntityId,
}

impl InteractAction {
    pub fn new(target: EntityId) -> Self {
        Self { target }
    }
}
