//! Core ViewModel structure.

use game_core::{EntityId, GameState, env::MapOracle};

use super::entities::{
    ActorView, ItemView, PropView, collect_actors, collect_items, collect_props,
};
use super::{MapView, TurnView, WorldSummary};

/// Stateful ViewModel owned by the event loop.
///
/// This structure maintains presentation-optimized state that is incrementally
/// updated as events arrive from the runtime, avoiding full state regeneration.
///
/// # Design
///
/// - `player`: Cached reference for O(1) access (UI frequently needs player data)
/// - `actors`: ALL actors including player (invariant: `actors[0]` is always player)
/// - This allows both fast player access AND convenient iteration over all actors
#[derive(Clone, Debug)]
pub struct ViewModel {
    /// Turn metadata (clock, current actor, active actors).
    pub turn: TurnView,

    /// 2D map grid with terrain info.
    pub map: MapView,

    /// Player actor cached for O(1) access.
    /// Invariant: Always equal to `actors[0]`.
    pub player: ActorView,

    /// ALL actors including player.
    /// Invariant: `actors[0].id == EntityId::PLAYER`.
    pub actors: Vec<ActorView>,

    /// All props for examination and rendering.
    pub props: Vec<PropView>,

    /// All items for examination and rendering.
    pub items: Vec<ItemView>,

    /// Aggregate world statistics.
    pub world: WorldSummary,

    /// Last synchronized GameState nonce for sync verification.
    pub last_sync_nonce: u64,
}

impl ViewModel {
    /// Create ViewModel from initial GameState.
    ///
    /// This is called once at startup. Subsequent updates use incremental
    /// methods to avoid full regeneration.
    ///
    /// # Invariants
    ///
    /// - `actors[0].id == EntityId::PLAYER`
    /// - `player` field equals `actors[0]`
    pub fn from_initial_state<M: MapOracle + ?Sized>(state: &GameState, map_oracle: &M) -> Self {
        let actors = collect_actors(state);
        let player = actors
            .first()
            .expect("Player must exist in actors list")
            .clone();

        let view_model = Self {
            turn: TurnView::from_state(state),
            map: MapView::from_state(map_oracle, state),
            player,
            actors,
            props: collect_props(state),
            items: collect_items(state),
            world: WorldSummary::from_state(state),
            last_sync_nonce: state.turn.nonce,
        };

        #[cfg(debug_assertions)]
        view_model.validate_invariants();

        view_model
    }

    /// Full rebuild from GameState (fallback for when incremental update is not feasible).
    pub fn rebuild_from_state<M: MapOracle + ?Sized>(&mut self, state: &GameState, map_oracle: &M) {
        self.turn = TurnView::from_state(state);
        self.map = MapView::from_state(map_oracle, state);

        self.actors = collect_actors(state);
        self.player = self
            .actors
            .first()
            .expect("Player must exist in actors list")
            .clone();

        self.props = collect_props(state);
        self.items = collect_items(state);
        self.world = WorldSummary::from_state(state);
        self.last_sync_nonce = state.turn.nonce;

        #[cfg(debug_assertions)]
        self.validate_invariants();
    }

    /// Get iterator over NPCs only (excludes player).
    ///
    /// This is a convenience method for UI code that needs to iterate over
    /// NPCs without the player. Uses skip(1) since player is always at index 0.
    pub fn npcs(&self) -> impl Iterator<Item = &ActorView> {
        self.actors.iter().skip(1)
    }

    /// Check if ViewModel is synchronized with given GameState.
    pub fn is_synced(&self, state: &GameState) -> bool {
        self.last_sync_nonce == state.turn.nonce
    }

    /// Validate ViewModel invariants (debug builds only).
    ///
    /// Ensures critical invariants are maintained:
    /// - `actors` list is not empty
    /// - `actors[0]` is always the player
    /// - `player` field matches `actors[0]`
    ///
    /// # Panics
    ///
    /// Panics if any invariant is violated (debug builds only).
    #[cfg(debug_assertions)]
    pub(crate) fn validate_invariants(&self) {
        debug_assert!(
            !self.actors.is_empty(),
            "ViewModel invariant broken: actors list must not be empty"
        );
        debug_assert_eq!(
            self.actors[0].id,
            EntityId::PLAYER,
            "ViewModel invariant broken: actors[0] must be player (got {:?})",
            self.actors[0].id
        );
        debug_assert_eq!(
            self.player.id, self.actors[0].id,
            "ViewModel invariant broken: player cache must match actors[0]"
        );
    }
}
