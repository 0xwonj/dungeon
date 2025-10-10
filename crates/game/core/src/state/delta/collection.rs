use std::collections::HashMap;
use std::hash::Hash;

/// Generic collection delta capturing additions, removals, and updates.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CollectionDelta<Id, Added, Patch> {
    pub added: Vec<Added>,
    pub removed: Vec<Id>,
    pub updated: Vec<Patch>,
}

impl<Id, Added, Patch> CollectionDelta<Id, Added, Patch> {
    pub(super) fn new() -> Self {
        Self {
            added: Vec::new(),
            removed: Vec::new(),
            updated: Vec::new(),
        }
    }
}

impl<Id, Added, Patch> Default for CollectionDelta<Id, Added, Patch> {
    fn default() -> Self {
        Self::new()
    }
}

pub(super) fn diff_collection<Id, T, Patch, FId, FPatch>(
    before: &[T],
    after: &[T],
    id_fn: FId,
    mut patch_fn: FPatch,
) -> CollectionDelta<Id, T, Patch>
where
    Id: Eq + Hash + Copy,
    T: Clone,
    FId: Fn(&T) -> Id,
    FPatch: FnMut(&T, &T) -> Option<Patch>,
{
    let mut before_map: HashMap<Id, &T> = before.iter().map(|item| (id_fn(item), item)).collect();
    let mut delta = CollectionDelta::new();

    for entry in after {
        let id = id_fn(entry);
        match before_map.remove(&id) {
            Some(prev) => {
                if let Some(patch) = patch_fn(prev, entry) {
                    delta.updated.push(patch);
                }
            }
            None => delta.added.push(entry.clone()),
        }
    }

    delta.removed.extend(before_map.into_keys());
    delta
}
