use std::collections::HashMap;

use crate::state::EntityId;

/// Generic collection delta tracking additions, removals, and updates.
///
/// This structure captures minimal metadata about collection changes:
/// - `added`: IDs of newly created entities
/// - `removed`: IDs of deleted entities
/// - `updated`: Metadata about modified entities (containing field bitmasks)
///
/// # Design Rationale
///
/// **Why store IDs instead of full entities for added/removed?**
/// - For `removed`: Entity no longer exists in `after` state, storing ID is sufficient
/// - For `added`: Full entity data available in `after` state, can be retrieved by ID
/// - Reduces delta size significantly (8 bytes per ID vs. 100+ bytes per entity)
///
/// **When is this different from storing full entities?**
/// - ZK witness generation needs full entity data: query from `after` state by ID
/// - Network transmission of new entities: query from `after` state by ID
/// - Bandwidth optimization: Can send just IDs, client queries from state snapshot
///
/// # Type Parameters
///
/// - `TChanges`: The change metadata type (e.g., `ActorChanges` with field bitmask)
#[derive(Clone, Debug, PartialEq, Eq, Default)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct CollectionChanges<TChanges> {
    pub added: Vec<EntityId>,
    pub removed: Vec<EntityId>,
    pub updated: Vec<TChanges>,
}

impl<TChanges> CollectionChanges<TChanges> {
    /// Creates an empty collection delta.
    #[inline]
    pub fn new() -> Self {
        Self {
            added: Vec::new(),
            removed: Vec::new(),
            updated: Vec::new(),
        }
    }

    /// Creates an empty collection delta (alias for zkvm compatibility).
    #[cfg(feature = "zkvm")]
    #[inline]
    pub(crate) fn empty() -> Self {
        Self::new()
    }

    /// Returns true if the collection is completely unchanged.
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.added.is_empty() && self.removed.is_empty() && self.updated.is_empty()
    }
}

/// Compares two entity collections and generates a delta.
///
/// This is the core diffing algorithm for entity collections. It uses a hash map
/// for efficient lookup and handles three cases:
/// 1. **Entity in after but not before** → Added
/// 2. **Entity in before but not after** → Removed
/// 3. **Entity in both** → Check fields, add to updated if changed
///
/// # Algorithm Complexity
///
/// - Time: O(n + m) where n = before.len(), m = after.len()
/// - Space: O(n) for the before_map hash table
///
/// # Parameters
///
/// - `before`, `after`: Slices of entities to compare
/// - `id_fn`: Function extracting entity ID (for matching entities across states)
/// - `changes_fn`: Function comparing two entities and generating change metadata
pub(super) fn diff_collection<T, TChanges, FId, FChanges>(
    before: &[T],
    after: &[T],
    id_fn: FId,
    mut changes_fn: FChanges,
) -> CollectionChanges<TChanges>
where
    T: Clone,
    FId: Fn(&T) -> EntityId,
    FChanges: FnMut(&T, &T) -> Option<TChanges>,
{
    // Build lookup map for before state (O(n))
    let mut before_map: HashMap<EntityId, &T> =
        before.iter().map(|item| (id_fn(item), item)).collect();

    let mut delta = CollectionChanges::new();

    // Process after state (O(m))
    for after_entity in after {
        let id = id_fn(after_entity);

        match before_map.remove(&id) {
            Some(before_entity) => {
                // Entity exists in both states - check if fields changed
                if let Some(changes) = changes_fn(before_entity, after_entity) {
                    delta.updated.push(changes);
                }
            }
            None => {
                // Entity only in after state - newly added
                delta.added.push(id);
            }
        }
    }

    // Remaining entities in before_map were removed (O(k) where k = removed count)
    delta.removed.extend(before_map.into_keys());

    delta
}
