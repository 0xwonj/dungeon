//! Minimal [`game_core::ItemOracle`] backed by an in-memory map.
use game_core::{
    ArmorData, ArmorKind, ConsumableData, ConsumableEffect, ItemDefinition, ItemHandle, ItemKind,
    ItemOracle, WeaponData, WeaponKind,
};
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

        // Add a basic health potion (stackable, max 99)
        oracle.add_definition(ItemDefinition::new(
            ItemHandle(1),
            ItemKind::Consumable(ConsumableData {
                effect: ConsumableEffect::HealHealth(50),
                use_cost: 100, // Takes 100 ticks to drink
            }),
            99, // max_stack
        ));

        // Add a basic sword (not stackable)
        oracle.add_definition(ItemDefinition::new(
            ItemHandle(2),
            ItemKind::Weapon(WeaponData {
                kind: WeaponKind::Sword,
                damage: 10,
            }),
            1, // max_stack
        ));

        // Add basic armor (not stackable)
        oracle.add_definition(ItemDefinition::new(
            ItemHandle(3),
            ItemKind::Armor(ArmorData {
                kind: ArmorKind::Light,
                defense: 5,
            }),
            1, // max_stack
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
