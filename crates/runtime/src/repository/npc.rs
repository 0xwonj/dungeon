use game_core::EntityId;
use std::collections::HashMap;

use super::NpcRepository;
use crate::error::Result;

/// NPC archetype for AI behavior
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NpcArchetype {
    /// Moves towards player, attacks if adjacent
    Chaser,
}

/// In-memory implementation of NpcRepository
pub struct InMemoryNpcRepo {
    npcs: HashMap<EntityId, NpcArchetype>,
}

impl InMemoryNpcRepo {
    pub fn new() -> Self {
        Self {
            npcs: HashMap::new(),
        }
    }

    /// Register an NPC with an archetype
    pub fn add_npc(&mut self, entity: EntityId, archetype: NpcArchetype) {
        self.npcs.insert(entity, archetype);
    }

    /// Get archetype for an NPC
    pub fn get_archetype(&self, entity: EntityId) -> Option<NpcArchetype> {
        self.npcs.get(&entity).copied()
    }
}

impl Default for InMemoryNpcRepo {
    fn default() -> Self {
        Self::new()
    }
}

impl NpcRepository for InMemoryNpcRepo {
    fn list_npcs(&self) -> Result<Vec<EntityId>> {
        Ok(self.npcs.keys().copied().collect())
    }
}
