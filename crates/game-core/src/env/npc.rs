use crate::state::{ActorStats, InventoryState, ResourceMeter};

/// Template describing the initial state of an NPC type.
/// Used by GameState initialization to create NPCs from map oracle specs.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct NpcTemplate {
    pub stats: ActorStats,
    pub inventory: InventoryState,
}

impl NpcTemplate {
    pub fn new(stats: ActorStats, inventory: InventoryState) -> Self {
        Self { stats, inventory }
    }

    /// Creates a simple template with given HP/energy and empty inventory.
    pub fn simple(max_health: u16, max_energy: u16) -> Self {
        Self {
            stats: ActorStats::new(
                ResourceMeter::new(max_health as u32, max_health as u32),
                ResourceMeter::new(max_energy as u32, max_energy as u32),
            ),
            inventory: InventoryState::default(),
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
