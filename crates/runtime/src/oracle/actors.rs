//! Actor oracle implementing [`game_core::ActorOracle`].

use std::collections::HashMap;

use game_core::{ActorOracle, ActorTemplate};

/// Oracle providing actor templates.
pub struct ActorOracleImpl {
    templates: HashMap<String, ActorTemplate>,
}

impl ActorOracleImpl {
    /// Create an empty oracle.
    pub fn new() -> Self {
        Self {
            templates: HashMap::new(),
        }
    }

    /// Add an actor template to the oracle.
    ///
    /// # Arguments
    ///
    /// * `id` - Actor definition ID (e.g., "goblin_scout", "player")
    /// * `template` - Actor template with resolved trait_profile
    pub fn add(&mut self, id: impl Into<String>, template: ActorTemplate) {
        let id = id.into();
        self.templates.insert(id, template);
    }

    /// Get actor template by ID.
    ///
    /// This is used by game-core via ActorOracle trait.
    pub fn template(&self, id: &str) -> Option<&ActorTemplate> {
        self.templates.get(id)
    }

    /// Check if an actor exists.
    pub fn contains(&self, id: &str) -> bool {
        self.templates.contains_key(id)
    }

    /// Get all actor IDs.
    pub fn actor_ids(&self) -> impl Iterator<Item = &String> {
        self.templates.keys()
    }

    /// Get number of actors in catalog.
    pub fn len(&self) -> usize {
        self.templates.len()
    }

    /// Check if catalog is empty.
    pub fn is_empty(&self) -> bool {
        self.templates.is_empty()
    }
}

impl Default for ActorOracleImpl {
    fn default() -> Self {
        Self::new()
    }
}

impl ActorOracle for ActorOracleImpl {
    fn template(&self, def_id: &str) -> Option<ActorTemplate> {
        self.templates.get(def_id).cloned()
    }

    fn all_ids(&self) -> Vec<String> {
        self.templates.keys().cloned().collect()
    }
}
