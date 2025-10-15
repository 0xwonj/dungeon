//! NPC template definitions and oracle interface.

use crate::state::InventoryState;
use crate::stats::{CoreStats, ResourceCurrent};

/// NPC template defining base attributes and inventory.
#[derive(Clone, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct NpcTemplate {
    pub core_stats: CoreStats,
    pub resources: ResourceCurrent,
    pub inventory: InventoryState,
}

impl NpcTemplate {
    /// Create a builder for constructing NPC templates
    pub fn builder() -> NpcTemplateBuilder {
        NpcTemplateBuilder::default()
    }

    /// Create a test NPC with default stats
    pub fn test_npc() -> Self {
        NpcTemplate::builder().build()
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
        let core_stats = self.stats.unwrap_or_default();

        // Compute default resource maximums if not explicitly set
        let resources =
            if let (Some(hp), Some(mp), Some(luc)) = (self.health, self.mana, self.lucidity) {
                ResourceCurrent::new(hp, mp, luc)
            } else {
                // Use reasonable defaults based on core stats
                // HP ≈ CON × 10, MP ≈ WIL × 5, Lucidity ≈ 50
                let hp = (core_stats.con as u32) * 10;
                let mp = (core_stats.wil as u32) * 5;
                let luc = 50;
                ResourceCurrent::new(hp, mp, luc)
            };

        NpcTemplate {
            core_stats,
            resources,
            inventory: self.inventory.unwrap_or_default(),
        }
    }
}

/// Oracle providing NPC template data for entity creation.
pub trait NpcOracle {
    /// Returns the template for a given NPC type ID.
    fn template(&self, template_id: u16) -> Option<NpcTemplate>;
}
