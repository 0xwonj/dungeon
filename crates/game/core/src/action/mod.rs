//! Action domain definitions.
//!
//! Provides typed representations for player intent and concrete action kinds executed by the engine.
pub mod combat;
pub mod interact;
pub mod inventory;
pub mod movement;
pub mod system;

use crate::env::GameEnv;
use crate::state::{EntityId, GameState, Tick};

pub use combat::{AttackAction, AttackStyle};
pub use interact::InteractAction;
pub use inventory::{InventorySlot, ItemTarget, UseItemAction};
pub use movement::{CardinalDirection, MoveAction, MoveError};
pub use system::{ActionCostAction, ActivationAction, PrepareTurnAction, TurnError};

/// Defines how a concrete action variant mutates game state while mirroring
/// the constraint checks enforced inside zk circuits.
///
/// Implementors can override the validation hooks to surface pre- and
/// post-conditions that must hold around the state mutation. All hooks receive
/// read-only access to deterministic environment facts via `Env` and must stay
/// side-effect free.
pub trait ActionTransition {
    type Error;

    /// Returns the entity performing this action.
    /// For system actions, this should return `EntityId::SYSTEM`.
    fn actor(&self) -> EntityId;

    /// Returns the time cost of this action in ticks.
    /// This cost is used to advance the actor's ready_at value.
    fn cost(&self) -> Tick;

    /// Validates pre-conditions using the state **before** mutation.
    fn pre_validate(&self, _state: &GameState, _env: &GameEnv<'_>) -> Result<(), Self::Error> {
        Ok(())
    }

    /// Applies the action by mutating the game state directly. Implementations should
    /// assume that `pre_validate` has already run successfully.
    fn apply(&self, state: &mut GameState, env: &GameEnv<'_>) -> Result<(), Self::Error>;

    /// Validates post-conditions using the state **after** mutation.
    fn post_validate(&self, _state: &GameState, _env: &GameEnv<'_>) -> Result<(), Self::Error> {
        Ok(())
    }
}

/// Describes a single intent issued by an entity for the current turn.
#[derive(Clone, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
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
            // System actions must be executed by SYSTEM actor
            ActionKind::PrepareTurn(_) | ActionKind::ActionCost(_) | ActionKind::Activation(_) => {
                actor.is_system()
            }
            _ => true,
        });
        Self { actor, kind }
    }

    /// Returns the time cost (in ticks) for this action.
    /// This determines how much the entity's ready_at advances after execution.
    /// Cost is scaled by the actor's speed stat (from snapshot).
    pub fn cost(&self, snapshot: &crate::stats::StatsSnapshot) -> Tick {
        use crate::action::ActionTransition;

        // Get base cost
        let base_cost = match &self.kind {
            ActionKind::Move(action) => action.cost(),
            ActionKind::Attack(action) => action.cost(),
            ActionKind::UseItem(action) => action.cost(),
            ActionKind::Interact(action) => action.cost(),
            ActionKind::Wait => 100,
            // System actions have no time cost
            ActionKind::PrepareTurn(action) => action.cost(),
            ActionKind::ActionCost(action) => action.cost(),
            ActionKind::Activation(action) => action.cost(),
        };

        // Scale by speed (100 = baseline)
        // Use physical speed for all actions (for now)
        let speed = snapshot.speed.physical.max(1) as u64;
        base_cost * 100 / speed
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum ActionKind {
    // Player/NPC actions
    Move(MoveAction),
    Attack(AttackAction),
    UseItem(UseItemAction),
    Interact(InteractAction),
    Wait,

    // System actions (executed by EntityId::SYSTEM)
    PrepareTurn(PrepareTurnAction),
    ActionCost(ActionCostAction),
    Activation(ActivationAction),
}

impl ActionKind {
    /// Returns the snake_case string representation of the action variant.
    /// Useful for logging, metrics, and file naming.
    pub fn as_snake_case(&self) -> &'static str {
        match self {
            ActionKind::Move(_) => "move",
            ActionKind::Attack(_) => "attack",
            ActionKind::UseItem(_) => "use_item",
            ActionKind::Interact(_) => "interact",
            ActionKind::Wait => "wait",
            ActionKind::PrepareTurn(_) => "prepare_turn",
            ActionKind::ActionCost(_) => "action_cost",
            ActionKind::Activation(_) => "activation",
        }
    }
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

impl From<PrepareTurnAction> for ActionKind {
    fn from(action: PrepareTurnAction) -> Self {
        Self::PrepareTurn(action)
    }
}

impl From<ActionCostAction> for ActionKind {
    fn from(action: ActionCostAction) -> Self {
        Self::ActionCost(action)
    }
}

impl From<ActivationAction> for ActionKind {
    fn from(action: ActivationAction) -> Self {
        Self::Activation(action)
    }
}
