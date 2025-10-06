use game_core::{ItemCategory, ItemDefinition, ItemHandle, ItemOracle};
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

    /// Create with basic test items
    pub fn test_items() -> Self {
        let mut oracle = Self::new();

        // Add a basic potion
        oracle.add_definition(ItemDefinition::new(
            ItemHandle(1),
            ItemCategory::Utility,
            None,
            None,
        ));

        oracle
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
}
