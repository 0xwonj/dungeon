//! Action domain definitions.
//!
//! Provides typed representations for player intent, including parsing helpers
//! (`ActionCommand`) and concrete action kinds executed by the engine.
pub mod command;
pub mod kinds;
pub mod transition;

use crate::state::{EntityId, Tick};

pub use command::{ActionCommand, CommandContext};
pub use kinds::{
    AttackAction, AttackCommand, AttackStyle, CardinalDirection, InteractAction, InteractCommand,
    InventorySlot, ItemTarget, MoveAction, MoveCommand, MoveError, UseItemAction, UseItemCommand,
};
pub use transition::ActionTransition;

/// Describes a single intent issued by an entity for the current turn.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Action {
    pub actor: EntityId,
    pub kind: ActionKind,
}

impl Action {
    pub fn new(actor: EntityId, kind: ActionKind) -> Self {
        debug_assert!(match &kind {
            ActionKind::Move(move_action) => move_action.actor == actor,
            ActionKind::Attack(attack_action) => attack_action.actor == actor,
            ActionKind::UseItem(use_item_action) => use_item_action.actor == actor,
            ActionKind::Interact(interact_action) => interact_action.actor == actor,
            _ => true,
        });
        Self { actor, kind }
    }

    pub fn from_command<C>(
        actor: EntityId,
        command: C,
        ctx: CommandContext<'_>,
    ) -> Result<Self, C::Error>
    where
        C: ActionCommand,
    {
        command.into_action(actor, ctx)
    }

    /// Returns the time cost (in ticks) for this action.
    /// This determines how much the entity's ready_at advances after execution.
    /// Cost is scaled by the actor's speed stat.
    pub fn cost(&self, stats: &crate::state::ActorStats) -> Tick {
        use crate::action::ActionTransition;

        // Get base cost
        let base_cost = match &self.kind {
            ActionKind::Move(action) => action.cost().0,
            ActionKind::Attack(action) => action.cost().0,
            ActionKind::UseItem(action) => action.cost().0,
            ActionKind::Interact(action) => action.cost().0,
            ActionKind::Wait => 100,
        };

        // Scale by speed (100 = baseline)
        let speed = stats.speed.max(1) as u64;
        Tick(base_cost * 100 / speed)
    }

    /// Calculates the delay for a given action kind and stats.
    /// Used by activate() to initialize ready_at.
    pub fn calculate_delay(kind: &ActionKind, stats: &crate::state::ActorStats) -> Tick {
        let base_cost = match kind {
            ActionKind::Wait => 100,
            _ => 100, // Default for other actions
        };

        let speed = stats.speed.max(1) as u64;
        Tick(base_cost * 100 / speed)
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

impl From<MoveAction> for ActionKind {
    fn from(action: MoveAction) -> Self {
        Self::Move(action)
    }
}

impl From<AttackAction> for ActionKind {
    fn from(action: AttackAction) -> Self {
        Self::Attack(action)
    }
}

impl From<UseItemAction> for ActionKind {
    fn from(action: UseItemAction) -> Self {
        Self::UseItem(action)
    }
}

impl From<InteractAction> for ActionKind {
    fn from(action: InteractAction) -> Self {
        Self::Interact(action)
    }
}
