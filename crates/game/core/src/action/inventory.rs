use core::convert::Infallible;

use crate::action::ActionTransition;
use crate::env::GameEnv;
use crate::state::{EntityId, GameState, Position, Tick};

/// Consumes or activates an item from the actor's inventory.
#[derive(Clone, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
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

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct InventorySlot {
    pub index: u8,
}

impl InventorySlot {
    pub fn new(index: u8) -> Self {
        Self { index }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum ItemTarget {
    Entity(EntityId),
    Position(Position),
}

impl ActionTransition for UseItemAction {
    type Error = Infallible;

    fn actor(&self) -> EntityId {
        self.actor
    }

    fn cost(&self) -> Tick {
        8
    }

    fn apply(&self, _state: &mut GameState, _env: &GameEnv<'_>) -> Result<(), Self::Error> {
        Ok(())
    }
}
