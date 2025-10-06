use core::convert::Infallible;

use crate::action::ActionTransition;
use crate::env::GameEnv;
use crate::state::{EntityId, GameState, Position};

/// Consumes or activates an item from the actor's inventory.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct UseItemAction {
    pub actor: EntityId,
    pub slot: InventorySlot,
    pub target: Option<ItemTarget>,
}

impl UseItemAction {
    pub fn new(actor: EntityId, slot: InventorySlot, target: Option<ItemTarget>) -> Self {
        Self {
            actor,
            slot,
            target,
        }
    }
}

/// Command describing how an item should be used.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct UseItemCommand {
    pub slot: InventorySlot,
    pub target: Option<ItemTarget>,
}

impl UseItemCommand {
    pub fn new(slot: InventorySlot, target: Option<ItemTarget>) -> Self {
        Self { slot, target }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct InventorySlot {
    pub index: u8,
}

impl InventorySlot {
    pub fn new(index: u8) -> Self {
        Self { index }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ItemTarget {
    Entity(EntityId),
    Position(Position),
}

impl ActionTransition for UseItemAction {
    type Error = Infallible;

    fn cost(&self) -> crate::state::Tick {
        crate::state::Tick(8)
    }

    fn apply(&self, _state: &mut GameState, _env: &GameEnv<'_>) -> Result<(), Self::Error> {
        Ok(())
    }
}
