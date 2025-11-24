//! Minimal [`game_core::ItemOracle`] backed by an in-memory map.
use game_core::{ItemDefinition, ItemHandle, ItemOracle};
use std::collections::HashMap;

/// ItemOracle implementation with static item definitions
pub struct ItemOracleImpl {
    definitions: HashMap<ItemHandle, ItemDefinition>,
}

impl ItemOracleImpl {
    pub fn new() -> Self {
        Self {
            definitions: HashMap::new(),
        }
    }

    /// Add an item definition
    pub fn add_definition(&mut self, def: ItemDefinition) {
        self.definitions.insert(def.handle, def);
    }
}

impl Default for ItemOracleImpl {
    fn default() -> Self {
        Self::new()
    }
}

impl ItemOracle for ItemOracleImpl {
    fn definition(&self, handle: ItemHandle) -> Option<ItemDefinition> {
        self.definitions.get(&handle).cloned()
    }

    fn all_definitions(&self) -> Vec<ItemDefinition> {
        self.definitions.values().cloned().collect()
    }
}
