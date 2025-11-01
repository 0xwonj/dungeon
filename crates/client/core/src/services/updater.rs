//! ViewModelUpdater service layer for delta-based incremental updates.
//!
//! This service interprets Events from the runtime and applies incremental
//! updates to the ViewModel, avoiding full state regeneration.
//!
//! # Architecture
//!
//! - `UpdateScope`: Bitflags tracking which parts of ViewModel changed (for selective rendering)
//! - `ViewModelUpdater`: Stateless service for applying runtime Events to ViewModel

use bitflags::bitflags;
use game_core::{GameState, StateDelta, env::MapOracle};
use runtime::{Event, GameStateEvent};

use crate::view_model::{
    ViewModel,
    entities::{collect_actors, collect_items, collect_props},
};

// ============================================================================
// UpdateScope - Fine-grained change tracking
// ============================================================================

bitflags! {
    /// Tracks which parts of ViewModel have been updated.
    ///
    /// This enables widgets to skip rendering unchanged areas for better performance.
    ///
    /// # Design
    ///
    /// - Each flag represents a logical section of ViewModel
    /// - Flags can be combined (e.g., `ACTORS | ITEMS`)
    /// - Composite flags like `ENTITIES` and `ALL` provide convenient shortcuts
    #[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
    pub struct UpdateScope: u32 {
        /// Turn metadata changed (clock, current actor, active actors).
        const TURN        = 0b00000001;

        /// Map terrain or static features changed.
        const MAP         = 0b00000010;

        /// World summary statistics changed.
        const WORLD       = 0b00000100;

        /// Actor entities changed (health, position, stats).
        const ACTORS      = 0b00001000;

        /// Prop entities changed (state, position).
        const PROPS       = 0b00010000;

        /// Item entities changed (position, ownership).
        const ITEMS       = 0b00100000;

        /// Only player stats changed (optimization for common case).
        const PLAYER_ONLY = 0b01000000;

        /// Map occupancy grid changed (entities moved).
        const OCCUPANCY   = 0b10000000;

        /// All entity types changed.
        const ENTITIES = Self::ACTORS.bits() | Self::PROPS.bits() | Self::ITEMS.bits();

        /// Everything changed (full rebuild).
        const ALL = Self::TURN.bits()
                  | Self::MAP.bits()
                  | Self::WORLD.bits()
                  | Self::ACTORS.bits()
                  | Self::PROPS.bits()
                  | Self::ITEMS.bits()
                  | Self::OCCUPANCY.bits();
    }
}

impl UpdateScope {
    /// Returns true if any entity-related scope is set.
    pub fn has_entity_changes(&self) -> bool {
        self.intersects(Self::ENTITIES)
    }
}

impl Default for UpdateScope {
    fn default() -> Self {
        Self::empty()
    }
}

// ============================================================================
// ViewModelUpdater - Event-driven ViewModel updates
// ============================================================================

/// Service layer for interpreting runtime Events and updating ViewModel incrementally.
///
/// # Design Principles
///
/// - **Stateless**: Pure functions that transform ViewModel based on Events
/// - **Event-driven**: Consumes runtime Events, applies delta-based updates
/// - **Selective updates**: Returns UpdateScope for rendering optimization
///
/// # Pattern
///
/// Similar to Redux reducers or ECS change detection systems - interprets
/// state changes and updates view layer incrementally.
///
/// # Delta Semantics
///
/// **IMPORTANT**: `StateDelta::is_empty() == true` means "delta info not provided",
/// NOT "no changes occurred". This happens when:
/// - ZK feature is enabled (delta calculation is expensive in zkVM)
/// - Runtime skips delta computation for performance
///
/// Therefore:
/// - Empty delta → Full rebuild (conservative, ensures correctness)
/// - Non-empty delta → Selective update (performance optimization)
pub struct ViewModelUpdater;

impl ViewModelUpdater {
    /// Update ViewModel based on runtime Event.
    ///
    /// Returns UpdateScope indicating which parts of ViewModel changed,
    /// enabling selective rendering optimizations.
    ///
    /// # Arguments
    ///
    /// * `view_model` - The ViewModel to update (mutated in place)
    /// * `event` - The Event from runtime
    /// * `state` - The current GameState (fallback for full rebuild)
    /// * `map_oracle` - Map oracle for terrain data
    ///
    /// # Returns
    ///
    /// UpdateScope flags indicating which ViewModel fields were updated.
    pub fn update<M: MapOracle + ?Sized>(
        view_model: &mut ViewModel,
        event: &Event,
        state: &GameState,
        map_oracle: &M,
    ) -> UpdateScope {
        match event {
            Event::GameState(game_event) => {
                Self::update_from_game_event(view_model, game_event, state, map_oracle)
            }
            Event::Proof(_) => {
                // Proof events don't affect ViewModel
                UpdateScope::empty()
            }
            Event::ActionRef(_) => {
                // ActionRef is just a log reference, doesn't carry state changes
                UpdateScope::empty()
            }
        }
    }

    /// Handle GameStateEvent updates with delta-based optimization.
    fn update_from_game_event<M: MapOracle + ?Sized>(
        view_model: &mut ViewModel,
        event: &GameStateEvent,
        state: &GameState,
        map_oracle: &M,
    ) -> UpdateScope {
        match event {
            GameStateEvent::ActionExecuted { delta, .. } => {
                Self::apply_delta(view_model, delta, state, map_oracle)
            }

            GameStateEvent::ActionFailed { .. } => {
                // Action failed - usually no state change, but refresh turn info
                view_model.turn.update_from_state(state);
                UpdateScope::TURN
            }
        }
    }

    /// Apply StateDelta to ViewModel with selective updates.
    ///
    /// Returns UpdateScope indicating which fields changed, enabling
    /// selective rendering optimizations.
    ///
    /// # Delta Semantics
    ///
    /// **IMPORTANT**: `delta.is_empty() == true` means "delta info not provided",
    /// NOT "no changes occurred". This happens when:
    /// - ZK feature is enabled (delta calculation is expensive in zkVM)
    /// - Runtime skips delta computation for performance
    ///
    /// Therefore:
    /// - Empty delta → Full rebuild (conservative, ensures correctness)
    /// - Non-empty delta → Selective update (performance optimization)
    ///
    /// # Returns
    ///
    /// UpdateScope flags indicating which ViewModel fields were updated.
    fn apply_delta<M: MapOracle + ?Sized>(
        view_model: &mut ViewModel,
        delta: &StateDelta,
        state: &GameState,
        map_oracle: &M,
    ) -> UpdateScope {
        // IMPORTANT: Empty delta means "delta info not provided", not "no changes"
        // → Do full rebuild to ensure correctness
        if delta.is_empty() {
            view_model.rebuild_from_state(state, map_oracle);
            return UpdateScope::ALL;
        }

        // Delta is available → Selective updates for performance
        let mut scope = UpdateScope::empty();

        // Update turn state if changed
        if !delta.turn.is_empty() {
            view_model.turn.update_from_state(state);
            scope |= UpdateScope::TURN;
        }

        // Update entities if changed
        if !delta.entities.is_empty() {
            // Check if actors changed (player or NPCs)
            if !delta.entities.actors.is_empty() {
                view_model.actors = collect_actors(state);
                view_model.player = view_model
                    .actors
                    .first()
                    .expect("Player must exist")
                    .clone();
                scope |= UpdateScope::ACTORS;

                #[cfg(debug_assertions)]
                view_model.validate_invariants();
            }

            // Check if props changed
            if !delta.entities.props.is_empty() {
                view_model.props = collect_props(state);
                scope |= UpdateScope::PROPS;
            }

            // Check if items changed
            if !delta.entities.items.is_empty() {
                view_model.items = collect_items(state);
                scope |= UpdateScope::ITEMS;
            }
        }

        // Update world state if changed
        if !delta.world.is_empty() {
            // NOTE: Currently WorldChanges only tracks occupancy (entity movements).
            // Terrain is static and never changes during gameplay.
            // Map widget computes entity positions dynamically from ViewModel.actors/props,
            // so we don't need to rebuild MapView for occupancy changes.
            //
            // If terrain modifications are added in the future, WorldChanges should
            // add a separate `terrain: Vec<TerrainChanges>` field, and we would
            // rebuild the map only when terrain actually changes:
            //
            //   if !delta.world.terrain.is_empty() {
            //       view_model.map = MapView::from_state(map_oracle, state);
            //       scope |= UpdateScope::MAP;
            //   }
            //
            // For now, just flag occupancy change without rebuilding map:
            scope |= UpdateScope::OCCUPANCY;
        }

        // Update world summary only if entities changed
        if scope.has_entity_changes() {
            view_model.world.update_from_state(state);
            scope |= UpdateScope::WORLD;
        }

        // Update sync nonce
        view_model.last_sync_nonce = state.turn.nonce;

        scope
    }
}
