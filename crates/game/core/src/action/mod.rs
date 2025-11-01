//! Action domain definitions.
//!
//! Provides typed representations for player intent and concrete action kinds executed by the engine.
pub mod available;
pub mod combat;
pub mod interact;
pub mod inventory;
pub mod movement;
pub mod system;
pub mod wait;

use crate::env::GameEnv;
use crate::state::{EntityId, GameState, Tick};

pub use available::get_available_actions;
pub use combat::AttackAction;
pub use interact::InteractAction;
pub use inventory::{InventoryIndex, ItemTarget, UseItemAction};
pub use movement::{CardinalDirection, MoveAction, MoveError};
pub use system::{ActionCostAction, ActivationAction, PrepareTurnAction, TurnError};
pub use wait::WaitAction;

/// Defines how a concrete action variant mutates game state while mirroring
/// the constraint checks enforced inside zk circuits.
///
/// Implementors can override the validation hooks to surface pre- and
/// post-conditions that must hold around the state mutation. All hooks receive
/// read-only access to deterministic environment facts via `Env` and must stay
/// side-effect free.
///
/// # Associated Types
///
/// - `Error`: Error type for validation and execution failures
/// - `Result`: Execution result metadata (e.g., combat outcome, movement penalties)
///   - Use `()` for actions with no meaningful result
///   - Use specific result types (e.g., `AttackResult`) for actions with outcomes
pub trait ActionTransition {
    type Error;
    type Result;

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
    ///
    /// Returns action-specific execution metadata (e.g., combat results, item effects).
    fn apply(&self, state: &mut GameState, env: &GameEnv<'_>) -> Result<Self::Result, Self::Error>;

    /// Validates post-conditions using the state **after** mutation.
    fn post_validate(&self, _state: &GameState, _env: &GameEnv<'_>) -> Result<(), Self::Error> {
        Ok(())
    }
}

/// Action variants for characters (player/NPC).
#[derive(Clone, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum CharacterActionKind {
    Move(MoveAction),
    Attack(AttackAction),
    UseItem(UseItemAction),
    Interact(InteractAction),
    Wait(WaitAction),
}

/// Action variants for system operations.
#[derive(Clone, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum SystemActionKind {
    PrepareTurn(PrepareTurnAction),
    ActionCost(ActionCostAction),
    Activation(ActivationAction),
}

/// Describes a single action executed during gameplay.
///
/// Type-level separation ensures characters and system are distinct:
/// - Character actions: executed by player/NPC characters
/// - System actions: executed by the game engine (not a character)
#[derive(Clone, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum Action {
    Character {
        actor: EntityId,
        kind: CharacterActionKind,
    },
    System {
        kind: SystemActionKind,
    },
}

impl Action {
    /// Creates a new character action.
    pub fn character(actor: EntityId, kind: CharacterActionKind) -> Self {
        debug_assert!(match &kind {
            CharacterActionKind::Move(move_action) => move_action.actor == actor,
            CharacterActionKind::Attack(attack_action) => attack_action.actor == actor,
            CharacterActionKind::UseItem(use_item_action) => use_item_action.actor == actor,
            CharacterActionKind::Interact(interact_action) => interact_action.actor == actor,
            CharacterActionKind::Wait(wait_action) => wait_action.actor == actor,
        });
        Self::Character { actor, kind }
    }

    /// Creates a new system action.
    pub fn system(kind: SystemActionKind) -> Self {
        Self::System { kind }
    }

    /// Returns the entity ID performing this action.
    /// For system actions, returns EntityId::SYSTEM.
    pub fn actor(&self) -> EntityId {
        match self {
            Action::Character { actor, .. } => *actor,
            Action::System { .. } => EntityId::SYSTEM,
        }
    }

    /// Returns the time cost (in ticks) for this action.
    /// Cost is scaled by the actor's speed stat (from snapshot).
    pub fn cost(&self, snapshot: &crate::stats::StatsSnapshot) -> Tick {
        use crate::action::ActionTransition;
        use crate::stats::calculate_action_cost;

        let base_cost = match self {
            Action::Character { kind, .. } => match kind {
                CharacterActionKind::Move(action) => action.cost(),
                CharacterActionKind::Attack(action) => action.cost(),
                CharacterActionKind::UseItem(action) => action.cost(),
                CharacterActionKind::Interact(action) => action.cost(),
                CharacterActionKind::Wait(action) => action.cost(),
            },
            Action::System { kind } => match kind {
                SystemActionKind::PrepareTurn(action) => action.cost(),
                SystemActionKind::ActionCost(action) => action.cost(),
                SystemActionKind::Activation(action) => action.cost(),
            },
        };

        calculate_action_cost(base_cost, snapshot.speed.physical)
    }

    /// Returns the snake_case string representation of the action.
    pub fn as_snake_case(&self) -> &'static str {
        match self {
            Action::Character { kind, .. } => match kind {
                CharacterActionKind::Move(_) => "move",
                CharacterActionKind::Attack(_) => "attack",
                CharacterActionKind::UseItem(_) => "use_item",
                CharacterActionKind::Interact(_) => "interact",
                CharacterActionKind::Wait(_) => "wait",
            },
            Action::System { kind } => match kind {
                SystemActionKind::PrepareTurn(_) => "prepare_turn",
                SystemActionKind::ActionCost(_) => "action_cost",
                SystemActionKind::Activation(_) => "activation",
            },
        }
    }
}

impl From<MoveAction> for CharacterActionKind {
    fn from(action: MoveAction) -> Self {
        Self::Move(action)
    }
}

impl From<AttackAction> for CharacterActionKind {
    fn from(action: AttackAction) -> Self {
        Self::Attack(action)
    }
}

impl From<UseItemAction> for CharacterActionKind {
    fn from(action: UseItemAction) -> Self {
        Self::UseItem(action)
    }
}

impl From<InteractAction> for CharacterActionKind {
    fn from(action: InteractAction) -> Self {
        Self::Interact(action)
    }
}

impl From<WaitAction> for CharacterActionKind {
    fn from(action: WaitAction) -> Self {
        Self::Wait(action)
    }
}

impl From<PrepareTurnAction> for SystemActionKind {
    fn from(action: PrepareTurnAction) -> Self {
        Self::PrepareTurn(action)
    }
}

impl From<ActionCostAction> for SystemActionKind {
    fn from(action: ActionCostAction) -> Self {
        Self::ActionCost(action)
    }
}

impl From<ActivationAction> for SystemActionKind {
    fn from(action: ActivationAction) -> Self {
        Self::Activation(action)
    }
}
