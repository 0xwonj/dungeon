mod bitmask;
mod changes;
mod collection;

use std::collections::BTreeSet;

use crate::action::Action;
use crate::state::{EntitiesState, GameState, Tick, WorldState};

pub use bitmask::{ActorFields, ItemFields, PropFields, TurnFields};
pub use changes::{ActorChanges, ItemChanges, OccupancyChanges, PropChanges, TurnChanges};
pub use collection::CollectionChanges;

use changes::{ActorChanges as AC, ItemChanges as IC, PropChanges as PC};
use collection::diff_collection;

/// Minimal description of an executed action's impact on the deterministic state.
///
/// The delta system uses **bitmask-based change tracking** to capture metadata about
/// state transitions without storing actual values. This design supports:
///
/// - **ZK proof generation**: Efficiently encode only state changes in the proof circuit
/// - **Bandwidth optimization**: Transmit minimal diffs over network
/// - **Memory efficiency**: ~30 bytes per action vs. ~20KB with full state clone
/// - **Audit trails**: Capture precise state transitions for replay and debugging
///
/// # Design Philosophy
///
/// **Deltas store metadata, not values**:
/// - Changed values exist in before/after `GameState`
/// - Bitmasks indicate *which* fields changed
/// - ZK layer queries actual values during witness generation
///
/// # Current Implementation: Post-hoc Diffing
///
/// Phase 1 uses **post-hoc state comparison** for simplicity and minimal invasiveness:
/// ```rust,ignore
/// let before = state.clone();
/// let after = engine.execute(action, state)?;
/// let delta = StateDelta::from_states(action, &before, &after);
/// ```
///
/// **Trade-offs:**
/// - ✅ Non-invasive: Game logic remains unchanged
/// - ✅ Simple: Single comparison pass after execution
/// - ✅ Maintainable: Delta generation isolated from game code
/// - ⚠️ Requires `clone()`: ~1-2μs overhead per action
/// - ⚠️ O(n) comparison: Scales with entity count
///
/// # Future Optimization: Inline Change Tracking
///
/// **Planned for Phase 4+**: Record changes during state mutations for zero-overhead deltas:
/// ```rust,ignore
/// impl GameState {
///     pub fn move_actor(&mut self, id: EntityId, pos: Position) {
///         self.entities.actor_mut(id).position = pos;
///         self.delta_tracker.mark(id, ActorFields::POSITION); // O(1) bit set
///     }
/// }
/// ```
///
/// **Benefits:**
/// - Eliminates `clone()` requirement
/// - O(1) per change instead of O(n) comparison
/// - Suitable for large state (10K+ entities)
///
/// **Challenges:**
/// - Invasive: Requires modifying all mutators
/// - Coupling: State becomes aware of delta tracking
/// - Complexity: Must ensure tracking consistency
///
/// **When to implement:** If profiling shows `clone()` or `from_states()` as bottleneck.
///
/// # Architecture
///
/// ```text
/// GameEngine::execute() → StateDelta (bitmasks only)
///                              ↓
///                    Runtime broadcasts to:
///                    - Clients (UI updates)
///                    - ProverWorker (ZK generation)
///                              ↓
///              ProverWorker queries before/after states
///                    using delta as a guide
/// ```
///
/// See: `docs/state-delta-architecture.md` for detailed design rationale.
#[derive(Clone, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct StateDelta {
    /// The action that caused this state transition.
    pub action: Action,

    /// Game clock tick when the action executed.
    pub clock: Tick,

    /// Changes to turn scheduling state.
    pub turn: TurnChanges,

    /// Changes to all game entities (player, NPCs, props, items).
    pub entities: EntitiesChanges,

    /// Changes to world state (occupancy grid).
    pub world: WorldChanges,
}

impl StateDelta {
    /// Creates a delta by comparing two game states.
    ///
    /// This is the primary entry point for delta creation. It performs field-by-field
    /// comparison and generates bitmasks indicating which fields changed.
    ///
    /// # Algorithm
    ///
    /// 1. Compare turn state (clock, current actor, active set)
    /// 2. Compare entities (player, NPCs, props, items) using collection diff
    /// 3. Compare world occupancy grid
    ///
    /// # Complexity
    ///
    /// - Time: O(n) where n = number of entities
    /// - Space: O(k) where k = number of changed entities (typically k << n)
    ///
    /// # Examples
    ///
    /// ```rust,ignore
    /// let before = state.clone();
    /// let after = engine.execute(action, state)?;
    /// let delta = StateDelta::from_states(action, &before, &after);
    /// ```
    pub fn from_states(action: Action, before: &GameState, after: &GameState) -> Self {
        Self {
            action,
            clock: after.turn.clock,
            turn: TurnChanges::from_states(&before.turn, &after.turn),
            entities: EntitiesChanges::from_states(&before.entities, &after.entities),
            world: WorldChanges::from_states(&before.world, &after.world),
        }
    }

    /// Returns true if no state changes occurred (no-op action).
    pub fn is_empty(&self) -> bool {
        self.turn.is_empty() && self.entities.is_empty() && self.world.is_empty()
    }

    /// Creates a minimal empty delta.
    ///
    /// This is used in zkvm mode where delta computation is completely skipped to reduce overhead.
    /// The delta contains placeholder values and no change tracking - it's only used to satisfy
    /// the return type while avoiding all computation costs. The placeholder values (action, clock)
    /// have no meaning and should not be used.
    #[cfg(feature = "zkvm")]
    pub fn empty() -> Self {
        use crate::action::CharacterActionKind;
        use crate::state::EntityId;

        Self {
            action: Action::Character {
                actor: EntityId(0),
                kind: CharacterActionKind::Wait,
            },
            clock: 0,
            turn: TurnChanges::default(),
            entities: EntitiesChanges::empty(),
            world: WorldChanges::default(),
        }
    }
}

/// Changes to all game entities.
///
/// Tracks modifications to the four entity categories:
/// - Player (single special entity)
/// - NPCs (dynamic collection)
/// - Props (dynamic collection)
/// - Items (dynamic collection)
#[derive(Clone, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct EntitiesChanges {
    pub actors: CollectionChanges<ActorChanges>,
    pub props: CollectionChanges<PropChanges>,
    pub items: CollectionChanges<ItemChanges>,
}

impl EntitiesChanges {
    fn from_states(before: &EntitiesState, after: &EntitiesState) -> Self {
        let actors = diff_collection(
            &before.actors,
            &after.actors,
            |actor| actor.id,
            AC::from_states,
        );

        let props = diff_collection(&before.props, &after.props, |prop| prop.id, PC::from_states);

        let items = diff_collection(&before.items, &after.items, |item| item.id, IC::from_states);

        Self {
            actors,
            props,
            items,
        }
    }

    /// Returns true if no entity changes occurred.
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.actors.is_empty() && self.props.is_empty() && self.items.is_empty()
    }
}

#[cfg(feature = "zkvm")]
impl EntitiesChanges {
    fn empty() -> Self {
        Self {
            actors: CollectionChanges::empty(),
            props: CollectionChanges::empty(),
            items: CollectionChanges::empty(),
        }
    }
}

/// Changes to world state.
///
/// Currently tracks only occupancy grid changes. Future extensions may include:
/// - Terrain modifications
/// - Fog of war updates
/// - Region state changes
#[derive(Clone, Debug, PartialEq, Eq, Default)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct WorldChanges {
    /// Tile positions where occupancy changed.
    ///
    /// The actual occupant lists are stored in before/after `WorldState` and
    /// can be queried by position when needed (e.g., for ZK witness generation).
    pub occupancy: Vec<OccupancyChanges>,
}

impl WorldChanges {
    fn from_states(before: &WorldState, after: &WorldState) -> Self {
        let occupancy = diff_occupancy(before, after);
        Self { occupancy }
    }

    /// Returns true if no world changes occurred.
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.occupancy.is_empty()
    }
}

/// Compares occupancy grids and returns list of changed positions.
///
/// # Algorithm
///
/// 1. Collect all positions from both before and after occupancy maps
/// 2. For each position, compare occupant lists
/// 3. Return positions where lists differ
///
/// # Optimization Notes
///
/// We store only positions, not the actual occupant lists, because:
/// - Occupant lists can be large (multiple entities per tile)
/// - Lists are already in before/after WorldState
/// - ZK layer queries by position when building witnesses
fn diff_occupancy(before: &WorldState, after: &WorldState) -> Vec<OccupancyChanges> {
    let mut positions = BTreeSet::new();
    positions.extend(before.tile_map.occupancy().keys().copied());
    positions.extend(after.tile_map.occupancy().keys().copied());

    positions
        .into_iter()
        .filter_map(|position| {
            let before_occupants = before
                .tile_map
                .occupants(&position)
                .map(|slot| slot.iter().copied().collect::<Vec<_>>())
                .unwrap_or_default();

            let after_occupants = after
                .tile_map
                .occupants(&position)
                .map(|slot| slot.iter().copied().collect::<Vec<_>>())
                .unwrap_or_default();

            if before_occupants != after_occupants {
                Some(OccupancyChanges { position })
            } else {
                None
            }
        })
        .collect()
}
