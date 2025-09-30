pub mod combat;
pub mod interact;
pub mod inventory;
pub mod movement;

use crate::state::EntityId;

pub use combat::{AttackAction, AttackStyle};
pub use interact::InteractAction;
pub use inventory::{InventorySlot, ItemTarget, UseItemAction};
pub use movement::{CardinalDirection, MoveAction};

/// Describes a single intent issued by an entity for the current turn.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Action {
    pub actor: EntityId,
    pub kind: ActionKind,
}

impl Action {
    pub fn new(actor: EntityId, kind: ActionKind) -> Self {
        Self { actor, kind }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ActionKind {
    Move(MoveAction),
    Attack(AttackAction),
    UseItem(UseItemAction),
    Interact(InteractAction),
    Wait,
}
