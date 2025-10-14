//! NPC template definitions and oracle interface.

use crate::state::InventoryState;
use crate::stats::{ActorStats, CoreStats, ResourceCurrent};

/// NPC template defining base attributes and inventory.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct NpcTemplate {
    pub stats: ActorStats,
    pub inventory: InventoryState,
}

impl NpcTemplate {
    /// Create a builder for constructing NPC templates
    pub fn builder() -> NpcTemplateBuilder {
        NpcTemplateBuilder::default()
    }

    /// Create a test NPC with default stats
    pub fn test_npc() -> Self {
        Self {
            stats: ActorStats::default(),
            inventory: InventoryState::default(),
        }
    }
}

/// Builder for constructing NPC templates.
#[derive(Default)]
pub struct NpcTemplateBuilder {
    stats: Option<CoreStats>,
    health: Option<u32>,
    mana: Option<u32>,
    lucidity: Option<u32>,
    inventory: Option<InventoryState>,
}

impl NpcTemplateBuilder {
    /// Set base stats
    pub fn stats(mut self, stats: CoreStats) -> Self {
        self.stats = Some(stats);
        self
    }

    /// Set current HP
    pub fn health(mut self, value: u32) -> Self {
        self.health = Some(value);
        self
    }

    /// Set current MP
    pub fn mana(mut self, value: u32) -> Self {
        self.mana = Some(value);
        self
    }

    /// Set current Lucidity
    pub fn lucidity(mut self, value: u32) -> Self {
        self.lucidity = Some(value);
        self
    }

    /// Set inventory
    pub fn inventory(mut self, inv: InventoryState) -> Self {
        self.inventory = Some(inv);
        self
    }

    /// Build the NPC template
    pub fn build(self) -> NpcTemplate {
        let core = self.stats.unwrap_or_default();
        let actor_stats =
            if let (Some(hp), Some(mp), Some(luc)) = (self.health, self.mana, self.lucidity) {
                ActorStats::new(core, ResourceCurrent::new(hp, mp, luc))
            } else {
                ActorStats::at_full(core)
            };

        NpcTemplate {
            stats: actor_stats,
            inventory: self.inventory.unwrap_or_default(),
        }
    }
}

/// Oracle providing NPC template data for entity creation.
pub trait NpcOracle {
    /// Returns the template for a given NPC type ID.
    fn template(&self, template_id: u16) -> Option<NpcTemplate>;
}
