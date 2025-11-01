//! AI context for utility-based decision making.
//!
//! The [`AiContext`] serves as the "blackboard" for AI decision-making across
//! all three layers (Intent, Tactic, Action selection). It provides:
//!
//! - Read access to game state
//! - Cached available actions (computed once per turn)
//! - Helper methods for scoring functions
//! - Action storage mechanism

use game_content::traits::TraitProfile;
use game_core::{Action, CharacterActionKind, EntityId, GameEnv, GameState};

/// Context for AI decision-making across all three layers.
///
/// # Design
///
/// This struct provides all the information needed for utility scoring:
///
/// 1. **Game State**: Current world state (entity positions, HP, etc.)
/// 2. **Available Actions**: Pre-computed list of executable actions (cached)
/// 3. **Oracles**: Static game data (maps, items, NPC templates)
/// 4. **Action Storage**: Selected action to be executed
///
/// # Caching Strategy
///
/// `available_actions` is computed once per turn using `get_available_actions()`
/// and reused across all three decision layers. This avoids redundant computation
/// and ensures consistency.
///
/// # Lifetime
///
/// The `'a` lifetime ensures that the context doesn't outlive the `GameState`
/// it references. This is safe because AI evaluation happens synchronously
/// within a single turn.
pub struct AiContext<'a> {
    /// The entity making the decision.
    pub entity: EntityId,

    /// Read-only access to the current game state.
    pub state: &'a GameState,

    /// Read-only access to all game oracles.
    pub env: GameEnv<'a>,

    /// Cached list of available actions for this entity.
    ///
    /// Computed once per turn using `game_core::get_available_actions()`.
    /// All three decision layers use this same list, avoiding redundant computation.
    available_actions: Vec<CharacterActionKind>,

    /// The action selected for execution.
    ///
    /// This is `None` initially. Layer 3 (Action Selection) sets this
    /// via `set_action()`.
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
    /// A new context with no action set and empty available_actions.
    /// Use `with_available_actions()` to populate the action cache.
    pub fn new(entity: EntityId, state: &'a GameState, env: GameEnv<'a>) -> Self {
        Self {
            entity,
            state,
            env,
            available_actions: Vec::new(),
            action: None,
        }
    }

    /// Sets the available actions cache (builder pattern).
    ///
    /// This should be called immediately after `new()` to populate the
    /// action cache for use by all three decision layers.
    ///
    /// # Arguments
    ///
    /// * `actions` - List of available actions from `game_core::get_available_actions()`
    ///
    /// # Returns
    ///
    /// Self for method chaining.
    pub fn with_available_actions(mut self, actions: Vec<CharacterActionKind>) -> Self {
        self.available_actions = actions;
        self
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

    // ========================================================================
    // Utility Scoring Helper Methods
    // ========================================================================

    /// Gets HP ratio for the current entity (0-100).
    ///
    /// # Returns
    ///
    /// Current HP as percentage of max HP, or 100 if entity has no HP stat.
    ///
    /// # Note
    ///
    /// Currently uses a hardcoded max HP of 100. Future: Derive from core_stats.
    pub fn hp_ratio(&self) -> u32 {
        if let Some(actor) = self.state.entities.actor(self.entity) {
            let current_hp = actor.resources.hp;
            let max_hp = 100; // TODO: Derive from core_stats
            ((current_hp * 100) / max_hp).min(100)
        } else {
            100 // Non-actors are considered "full health"
        }
    }

    /// Gets distance to player in tiles.
    ///
    /// # Returns
    ///
    /// Manhattan distance to player, or `u32::MAX` if player position unknown.
    pub fn distance_to_player(&self) -> u32 {
        if let Some(actor) = self.state.entities.actor(self.entity) {
            let player_pos = self.state.entities.player().position;
            let dx = (actor.position.x - player_pos.x).abs();
            let dy = (actor.position.y - player_pos.y).abs();
            (dx + dy) as u32
        } else {
            u32::MAX
        }
    }

    /// Checks if player is visible to this entity.
    ///
    /// Uses Manhattan distance to determine if player is within sight range.
    ///
    /// # Returns
    ///
    /// True if player is within 10 tiles (Manhattan distance).
    ///
    /// # Future Improvements
    ///
    /// - Add line-of-sight checks (walls blocking vision)
    /// - Consider perception traits (some NPCs see further)
    /// - Consider light levels (darkness reduces vision)
    pub fn can_see_player(&self) -> bool {
        // Calculate distance to player
        let distance = self.distance_to_player();

        // If distance is MAX, entity is not an actor or other error
        if distance == u32::MAX {
            tracing::warn!(
                "can_see_player: entity {:?} has invalid distance to player",
                self.entity
            );
            return false;
        }

        // Sight range: 10 tiles (TODO: make configurable, trait-based)
        const SIGHT_RANGE: u32 = 10;

        let can_see = distance <= SIGHT_RANGE;

        tracing::debug!(
            "NPC {:?} checking vision to player: distance={}, can_see={}",
            self.entity,
            distance,
            can_see
        );

        can_see
    }

    /// Counts nearby allies within the specified range.
    ///
    /// # Arguments
    ///
    /// * `range` - Maximum distance in tiles (Manhattan distance)
    ///
    /// # Returns
    ///
    /// Number of allied entities within range.
    ///
    /// # TODO
    ///
    /// Currently returns 0 (no allies). Need to implement entity iteration
    /// in EntitiesState to enable proper ally counting.
    pub fn count_nearby_allies(&self, _range: u32) -> u32 {
        // TODO: Implement when EntitiesState has actors() iterator
        0
    }

    /// Counts visible enemies.
    ///
    /// Currently counts all actors with different template_id.
    /// Future: Add line-of-sight and faction system.
    ///
    /// # TODO
    ///
    /// Currently returns 1 (assumes player is always visible enemy).
    /// Need to implement entity iteration in EntitiesState.
    pub fn count_visible_enemies(&self) -> u32 {
        // TODO: Implement when EntitiesState has actors() iterator
        // For now, assume player is always visible as enemy
        1
    }

    /// Checks if there's a valid escape route.
    ///
    /// Currently simplified to always return true.
    /// Future: Add proper pathfinding and passability checks.
    ///
    /// # TODO
    ///
    /// Implement proper escape route checking when MapOracle exposes
    /// necessary methods (dimensions, passability).
    pub fn has_escape_route(&self) -> bool {
        // TODO: Check adjacent tiles for passability
        true
    }

    // ========================================================================
    // Position Helpers (for Layer 3 Action Selection)
    // ========================================================================

    /// Gets the current entity's position.
    ///
    /// # Returns
    ///
    /// The entity's position, or `Position::new(0, 0)` if not found.
    pub fn my_position(&self) -> game_core::Position {
        self.state
            .entities
            .actor(self.entity)
            .map(|a| a.position)
            .unwrap_or(game_core::Position::new(0, 0))
    }

    /// Gets the player's position.
    ///
    /// # Returns
    ///
    /// The player's current position.
    pub fn player_position(&self) -> game_core::Position {
        self.state.entities.player().position
    }

    /// Calculates the position after applying a cardinal direction move.
    ///
    /// # Arguments
    ///
    /// * `direction` - The cardinal direction to move
    ///
    /// # Returns
    ///
    /// The new position after moving in the specified direction.
    pub fn position_after_move(
        &self,
        direction: game_core::CardinalDirection,
    ) -> game_core::Position {
        let current = self.my_position();
        // Use the game's coordinate system (from CardinalDirection::delta)
        let (dx, dy) = direction.delta();
        game_core::Position::new(current.x + dx, current.y + dy)
    }

    /// Calculates the Manhattan distance from a given position to the player.
    ///
    /// # Arguments
    ///
    /// * `pos` - The position to calculate distance from
    ///
    /// # Returns
    ///
    /// Manhattan distance to the player.
    pub fn distance_from_to_player(&self, pos: game_core::Position) -> u32 {
        let player_pos = self.player_position();
        let dx = (pos.x - player_pos.x).abs();
        let dy = (pos.y - player_pos.y).abs();
        (dx + dy) as u32
    }

    // ========================================================================
    // Trait Profile Access
    // ========================================================================

    /// Gets the trait profile for the current entity.
    ///
    /// This method looks up the entity's template_id from the game state,
    /// then queries the NpcOracle for the corresponding trait profile.
    ///
    /// # Returns
    ///
    /// - `Some(&TraitProfile)` if the entity is an actor with a registered trait profile
    /// - `None` if:
    ///   - Entity is not an actor (e.g., prop, item)
    ///   - NPC oracle is not available
    ///   - Template has no trait profile registered
    pub fn trait_profile(&self) -> Option<&TraitProfile> {
        // TODO: Restore trait profile lookup after actor system migration
        // Previously: Retrieved template_id from ActorState and looked up trait profile
        // Next: Need to track def_id -> template_id mapping in runtime or store def_id in ActorState
        None
    }

    // ========================================================================
    // Available Actions Accessors
    // ========================================================================

    /// Gets the cached list of available actions.
    ///
    /// This list is computed once per turn and reused across all decision layers.
    ///
    /// # Returns
    ///
    /// Slice of all actions this entity can currently execute.
    pub fn available_actions(&self) -> &[CharacterActionKind] {
        &self.available_actions
    }

    /// Filters available actions to attack actions only.
    ///
    /// # Returns
    ///
    /// Iterator over attack actions (melee, ranged, special attacks).
    pub fn attack_actions(&self) -> impl Iterator<Item = &CharacterActionKind> {
        self.available_actions
            .iter()
            .filter(|action| matches!(action, CharacterActionKind::Attack(_)))
    }

    /// Filters available actions to movement actions only.
    ///
    /// # Returns
    ///
    /// Iterator over movement actions (Move in various directions).
    pub fn movement_actions(&self) -> impl Iterator<Item = &CharacterActionKind> {
        self.available_actions
            .iter()
            .filter(|action| matches!(action, CharacterActionKind::Move(_)))
    }

    /// Filters available actions to item usage actions only.
    ///
    /// # Returns
    ///
    /// Iterator over UseItem actions.
    pub fn item_actions(&self) -> impl Iterator<Item = &CharacterActionKind> {
        self.available_actions
            .iter()
            .filter(|action| matches!(action, CharacterActionKind::UseItem(_)))
    }

    /// Filters available actions to interaction actions only.
    ///
    /// # Returns
    ///
    /// Iterator over Interact actions (doors, chests, NPCs).
    pub fn interact_actions(&self) -> impl Iterator<Item = &CharacterActionKind> {
        self.available_actions
            .iter()
            .filter(|action| matches!(action, CharacterActionKind::Interact(_)))
    }

    /// Checks if Wait action is available.
    ///
    /// # Returns
    ///
    /// True if Wait is in the available actions list.
    pub fn can_wait(&self) -> bool {
        self.available_actions
            .iter()
            .any(|action| matches!(action, CharacterActionKind::Wait(_)))
    }

    /// Checks if any attack action is available.
    ///
    /// # Returns
    ///
    /// True if at least one attack action exists.
    pub fn has_attack_actions(&self) -> bool {
        let has_attacks = self.attack_actions().next().is_some();

        tracing::debug!(
            "NPC {:?} has_attack_actions: {} (total available: {})",
            self.entity,
            has_attacks,
            self.available_actions.len()
        );

        if !has_attacks && !self.available_actions.is_empty() {
            tracing::debug!("  Available actions: {:?}", self.available_actions);
        }

        has_attacks
    }

    /// Checks if any movement action is available.
    ///
    /// # Returns
    ///
    /// True if at least one movement action exists.
    pub fn has_movement_actions(&self) -> bool {
        self.movement_actions().next().is_some()
    }

    /// Checks if any item usage action is available.
    ///
    /// # Returns
    ///
    /// True if at least one item action exists.
    pub fn has_item_actions(&self) -> bool {
        self.item_actions().next().is_some()
    }

    /// Counts total number of available actions.
    ///
    /// # Returns
    ///
    /// Number of actions in the cache.
    pub fn action_count(&self) -> usize {
        self.available_actions.len()
    }
}
