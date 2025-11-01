use crate::action::ActionTransition;
use crate::env::GameEnv;
use crate::error::NeverError;
use crate::state::{EntityId, GameState, Position, Tick};

/// Consumes or activates an item from the actor's inventory.
#[derive(Clone, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct UseItemAction {
    pub actor: EntityId,
    pub slot: InventoryIndex,
    pub target: Option<ItemTarget>,
}

impl UseItemAction {
    pub fn new(actor: EntityId, slot: InventoryIndex, target: Option<ItemTarget>) -> Self {
        Self {
            actor,
            slot,
            target,
        }
    }
}

/// Index into an actor's inventory.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct InventoryIndex {
    pub index: u8,
}

impl InventoryIndex {
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
    type Error = NeverError;
    type Result = ();

    fn actor(&self) -> EntityId {
        self.actor
    }

    fn cost(&self, env: &GameEnv<'_>) -> Tick {
        env.tables()
            .map(|t| t.action_costs().use_item)
            .unwrap_or(100)
    }

    fn apply(&self, _state: &mut GameState, _env: &GameEnv<'_>) -> Result<(), Self::Error> {
        Ok(())
    }
}
