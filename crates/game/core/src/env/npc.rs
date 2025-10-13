use crate::state::{ActorStats, InventoryState, ResourceMeter};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct NpcTemplate {
    pub stats: ActorStats,
    pub inventory: InventoryState,
}

impl NpcTemplate {
    pub fn builder() -> NpcTemplateBuilder {
        NpcTemplateBuilder::default()
    }

    pub fn test_npc() -> Self {
        NpcTemplate {
            stats: ActorStats {
                health: ResourceMeter::new(30, 30),
                energy: ResourceMeter::new(20, 20),
                speed: 100,
            },
            inventory: InventoryState::default(),
        }
    }
}

/// Builder for constructing `NpcTemplate` step-by-step.
#[derive(Default)]
pub struct NpcTemplateBuilder {
    max_health: Option<u32>,
    max_energy: Option<u32>,
    speed: Option<u16>,
    inventory: Option<InventoryState>,
}

impl NpcTemplateBuilder {
    pub fn health(mut self, value: u32) -> Self {
        self.max_health = Some(value);
        self
    }

    pub fn energy(mut self, value: u32) -> Self {
        self.max_energy = Some(value);
        self
    }

    pub fn speed(mut self, value: u16) -> Self {
        self.speed = Some(value);
        self
    }

    pub fn inventory(mut self, inv: InventoryState) -> Self {
        self.inventory = Some(inv);
        self
    }

    pub fn build(self) -> NpcTemplate {
        let max_health = self.max_health.unwrap_or(10);
        let max_energy = self.max_energy.unwrap_or(5);
        let speed = self.speed.unwrap_or(100);

        NpcTemplate {
            stats: ActorStats::new(
                ResourceMeter::new(max_health, max_health),
                ResourceMeter::new(max_energy, max_energy),
                speed,
            ),
            inventory: self.inventory.unwrap_or_default(),
        }
    }
}

/// Oracle providing NPC template data for entity creation and reference.
///
/// This oracle defines the base stats and inventory for different NPC types.
/// Unlike MapOracle which describes WHERE entities are placed,
/// NpcOracle describes WHAT each NPC type's characteristics are.
pub trait NpcOracle {
    /// Returns the template for a given NPC type.
    /// Template IDs are game-specific and should be documented per game.
    ///
    /// Returns None if the template ID is unknown.
    fn template(&self, template_id: u16) -> Option<NpcTemplate>;
}
