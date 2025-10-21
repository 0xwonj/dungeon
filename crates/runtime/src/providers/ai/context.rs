//! AI context for behavior tree evaluation.
//!
//! The [`AiContext`] serves as the "blackboard" in behavior tree terminology.
//! It provides read access to game state and a mechanism to store the generated
//! action that will be returned to the runtime.

use game_core::{Action, EntityId, GameEnv, GameState};

/// Context passed to behavior tree nodes during AI evaluation.
///
/// # Design
///
/// This struct bridges the generic `behavior-tree` library with game-specific
/// types from `game-core`. It follows the pattern:
///
/// 1. Behavior nodes read `state` to make decisions
/// 2. Action nodes call `set_action()` to specify what to do
/// 3. The provider extracts the action with `take_action()`
///
/// # Lifetime
///
/// The `'a` lifetime ensures that the context doesn't outlive the `GameState`
/// it references. This is safe because AI evaluation happens synchronously
/// within a single turn.
///
/// # Example
///
/// ```rust,ignore
/// use behavior_tree::{Behavior, Status};
/// use runtime::providers::ai::AiContext;
/// use game_core::{Action, CharacterActionKind};
///
/// struct MoveNorth;
///
/// impl Behavior<AiContext<'_>> for MoveNorth {
///     fn tick(&self, ctx: &mut AiContext) -> Status {
///         let action = Action::character(
///             ctx.entity,
///             CharacterActionKind::Move(/* ... */),
///         );
///
///         ctx.set_action(action);
///         Status::Success
///     }
/// }
/// ```
pub struct AiContext<'a> {
    /// The entity making the decision.
    ///
    /// This is typically an NPC, but could be any entity with an action provider.
    pub entity: EntityId,

    /// Read-only access to the current game state.
    ///
    /// Nodes can query this to make decisions:
    /// - `state.entities.player.position` - where is the player?
    /// - `state.entities.actor(entity)` - get this NPC's state
    /// - `state.world.tile_map` - check tile occupancy
    pub state: &'a GameState,

    /// Read-only access to all game oracles.
    ///
    /// Provides access to static game data:
    /// - `env.map()` - terrain, passability, dimensions
    /// - `env.items()` - item definitions and properties
    /// - `env.tables()` - combat tables, movement rules
    /// - `env.npcs()` - NPC templates and stats
    /// - `env.config()` - game configuration
    pub env: GameEnv<'a>,

    /// The action to be executed (set by action nodes).
    ///
    /// This is `None` initially. Action nodes should call `set_action()`
    /// to specify what the entity should do.
    action: Option<Action>,
}

impl<'a> AiContext<'a> {
    /// Creates a new AI context for the given entity and game state.
    ///
    /// # Arguments
    ///
    /// * `entity` - The entity ID making the decision
    /// * `state` - Reference to the current game state
    /// * `env` - Reference to all game oracles
    ///
    /// # Returns
    ///
    /// A new context with no action set.
    pub fn new(entity: EntityId, state: &'a GameState, env: GameEnv<'a>) -> Self {
        Self {
            entity,
            state,
            env,
            action: None,
        }
    }

    /// Sets the action that this entity should execute.
    ///
    /// # Arguments
    ///
    /// * `action` - The action to execute
    ///
    /// # Panics
    ///
    /// Panics if an action was already set. This indicates a bug in the
    /// behavior tree (multiple action nodes succeeded in a single evaluation).
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// ctx.set_action(Action::character(
    ///     ctx.entity,
    ///     CharacterActionKind::Wait,
    /// ));
    /// ```
    pub fn set_action(&mut self, action: Action) {
        if self.action.is_some() {
            panic!(
                "Action already set for entity {:?}. \
                 Multiple action nodes succeeded in the same evaluation.",
                self.entity
            );
        }
        self.action = Some(action);
    }

    /// Extracts the action from this context, if one was set.
    ///
    /// This consumes the context and returns the action. If no action was set,
    /// returns `None`.
    ///
    /// # Returns
    ///
    /// The action that was set, or `None` if no action node succeeded.
    pub fn take_action(self) -> Option<Action> {
        self.action
    }

    /// Checks if an action has been set.
    ///
    /// This is useful for debugging or conditional logic.
    pub fn has_action(&self) -> bool {
        self.action.is_some()
    }
}
