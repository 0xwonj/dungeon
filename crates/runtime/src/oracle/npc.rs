//! NPC templates implementing [`game_core::NpcOracle`].
use game_core::{NpcOracle, NpcTemplate};
use std::collections::HashMap;

/// NpcOracle implementation with static NPC templates
pub struct NpcOracleImpl {
    templates: HashMap<u16, NpcTemplate>,
}

impl NpcOracleImpl {
    pub fn new() -> Self {
        Self {
            templates: HashMap::new(),
        }
    }

    /// Add an NPC template
    pub fn add_template(&mut self, template_id: u16, template: NpcTemplate) {
        self.templates.insert(template_id, template);
    }

    /// Create with basic test NPCs
    pub fn test_npcs() -> Self {
        let mut oracle = Self::new();

        // Template 0: Weak goblin
        oracle.add_template(0, NpcTemplate::simple(50, 30));

        // Template 1: Strong orc
        oracle.add_template(1, NpcTemplate::simple(100, 50));

        // Template 2: Boss
        oracle.add_template(2, NpcTemplate::simple(200, 100));

        oracle
    }
}

impl Default for NpcOracleImpl {
    fn default() -> Self {
        Self::new()
    }
}

impl NpcOracle for NpcOracleImpl {
    fn template(&self, template_id: u16) -> Option<NpcTemplate> {
        self.templates.get(&template_id).cloned()
    }
}
