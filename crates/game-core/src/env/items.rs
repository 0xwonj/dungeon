use crate::state::ItemHandle;

pub trait ItemOracle {
    fn definition(&self, handle: ItemHandle) -> Option<ItemDefinition>;
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ItemDefinition {
    pub handle: ItemHandle,
    pub category: ItemCategory,
    pub cooldown_turns: Option<u8>,
    pub charges: Option<u8>,
}

impl ItemDefinition {
    pub fn new(
        handle: ItemHandle,
        category: ItemCategory,
        cooldown_turns: Option<u8>,
        charges: Option<u8>,
    ) -> Self {
        Self {
            handle,
            category,
            cooldown_turns,
            charges,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ItemCategory {
    Consumable,
    Key,
    Equipment,
    Utility,
    Custom(u16),
}
