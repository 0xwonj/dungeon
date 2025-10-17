//! Provider registry for managing action providers.
//!
//! The [`ProviderRegistry`] implements the Registry pattern to manage
//! action providers for different entity types, eliminating duplicate
//! fields and providing a unified interface.

use std::collections::HashMap;

use game_core::EntityId;

use super::{ActionProvider, ProviderKind, Result, RuntimeError};

/// Registry for managing action providers.
pub struct ProviderRegistry {
    providers: HashMap<ProviderKind, Box<dyn ActionProvider>>,
}

impl ProviderRegistry {
    /// Create a new empty provider registry.
    pub fn new() -> Self {
        Self {
            providers: HashMap::new(),
        }
    }

    /// Register a provider for a specific kind.
    ///
    /// If a provider already exists for this kind, it will be replaced.
    pub fn register(&mut self, kind: ProviderKind, provider: impl ActionProvider + 'static) {
        self.providers.insert(kind, Box::new(provider));
    }

    /// Get a provider for a specific kind.
    ///
    /// # Errors
    ///
    /// Returns `RuntimeError::ProviderNotSet` if no provider is registered for this kind.
    pub fn get(&self, kind: ProviderKind) -> Result<&dyn ActionProvider> {
        self.providers
            .get(&kind)
            .map(|boxed| &**boxed)
            .ok_or_else(|| RuntimeError::ProviderNotSet { kind })
    }

    /// Get a provider for a specific entity.
    ///
    /// Maps entity ID to provider kind and retrieves the corresponding provider.
    pub fn get_for_entity(&self, entity: EntityId) -> Result<&dyn ActionProvider> {
        let kind = if entity == EntityId::PLAYER {
            ProviderKind::Player
        } else {
            ProviderKind::Npc
        };
        self.get(kind)
    }

    /// Check if a provider is registered for a specific kind.
    pub fn has(&self, kind: ProviderKind) -> bool {
        self.providers.contains_key(&kind)
    }

    /// Check if providers are registered for both player and NPC.
    pub fn is_complete(&self) -> bool {
        self.has(ProviderKind::Player) && self.has(ProviderKind::Npc)
    }

    /// Remove a provider for a specific kind.
    ///
    /// Returns the removed provider, or None if it wasn't registered.
    pub fn unregister(&mut self, kind: ProviderKind) -> Option<Box<dyn ActionProvider>> {
        self.providers.remove(&kind)
    }
}

impl Default for ProviderRegistry {
    fn default() -> Self {
        Self::new()
    }
}
