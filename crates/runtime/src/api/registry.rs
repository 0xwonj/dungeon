//! Provider registry for managing action providers.
//!
//! The [`ProviderRegistry`] implements the Registry pattern to manage
//! action providers, supporting flexible entity-to-provider mappings.
//!
//! # Design
//!
//! - **Provider instances**: Stored by `ProviderKind`, shared across entities
//! - **Entity mappings**: Each entity can be bound to a specific `ProviderKind`
//! - **Fallback chain**: Entity mapping → Default provider
//! - **Runtime changes**: Entities can switch providers dynamically

use std::collections::HashMap;
use std::sync::Arc;

use game_core::EntityId;

use super::{ActionProvider, AiKind, ProviderKind, Result, RuntimeError};

/// Registry for managing action providers with entity-specific bindings.
///
/// # Architecture
///
/// ```text
/// ProviderRegistry
/// ├── providers: HashMap<ProviderKind, Provider>  (provider instances)
/// ├── entity_mappings: HashMap<EntityId, ProviderKind>  (entity bindings)
/// └── default_kind: ProviderKind  (fallback)
/// ```
pub struct ProviderRegistry {
    /// Provider instances by kind (shared across entities)
    /// Uses Arc instead of Box to allow cloning providers for use outside locks
    providers: HashMap<ProviderKind, Arc<dyn ActionProvider>>,

    /// Entity-to-provider mappings (sparse - only non-default entities)
    entity_mappings: HashMap<EntityId, ProviderKind>,

    /// Default provider kind for unmapped entities
    default_kind: ProviderKind,
}

impl ProviderRegistry {
    /// Create a new provider registry with default AI (Wait) fallback.
    pub fn new() -> Self {
        Self {
            providers: HashMap::new(),
            entity_mappings: HashMap::new(),
            default_kind: ProviderKind::Ai(AiKind::Wait),
        }
    }

    /// Register a provider for a specific kind.
    ///
    /// If a provider already exists for this kind, it will be replaced.
    pub fn register(&mut self, kind: ProviderKind, provider: impl ActionProvider + 'static) {
        self.providers.insert(kind, Arc::new(provider));
    }

    /// Register a boxed provider for a specific kind.
    ///
    /// This is useful when you already have a `Box<dyn ActionProvider>`.
    pub fn register_boxed(&mut self, kind: ProviderKind, provider: Box<dyn ActionProvider>) {
        self.providers.insert(kind, Arc::from(provider));
    }

    /// Bind an entity to a specific provider kind.
    ///
    /// The entity will use this provider until unbound or rebound to a different kind.
    ///
    /// # Example
    ///
    /// ```ignore
    /// registry.bind_entity(EntityId::PLAYER, ProviderKind::Interactive(InteractiveKind::CliInput));
    /// registry.bind_entity(EntityId(1), ProviderKind::Ai(AiKind::Aggressive));
    /// ```
    pub fn bind_entity(&mut self, entity: EntityId, kind: ProviderKind) {
        self.entity_mappings.insert(entity, kind);
    }

    /// Unbind an entity, reverting it to the default provider.
    ///
    /// Returns the previous provider kind if it was bound.
    pub fn unbind_entity(&mut self, entity: EntityId) -> Option<ProviderKind> {
        self.entity_mappings.remove(&entity)
    }

    /// Set the default provider kind for unmapped entities.
    ///
    /// This is used as a fallback when an entity has no explicit binding.
    pub fn set_default(&mut self, kind: ProviderKind) {
        self.default_kind = kind;
    }

    /// Get the default provider kind.
    pub fn default_kind(&self) -> ProviderKind {
        self.default_kind
    }

    /// Get the provider kind for an entity.
    ///
    /// Returns the explicitly bound kind, or the default if not bound.
    pub fn get_entity_kind(&self, entity: EntityId) -> ProviderKind {
        self.entity_mappings
            .get(&entity)
            .copied()
            .unwrap_or(self.default_kind)
    }

    /// Get a provider for a specific entity.
    ///
    /// # Resolution Order
    ///
    /// 1. Check if entity has explicit binding → use that provider kind
    /// 2. Otherwise → use default provider kind
    /// 3. Lookup provider instance by kind
    ///
    /// # Errors
    ///
    /// Returns `RuntimeError::ProviderNotSet` if the resolved provider kind
    /// has no registered provider instance.
    ///
    /// # Returns
    ///
    /// Returns an Arc clone of the provider, allowing it to be used outside
    /// of the registry's lock scope.
    pub fn get_for_entity(&self, entity: EntityId) -> Result<Arc<dyn ActionProvider>> {
        let kind = self.get_entity_kind(entity);
        self.get(kind)
    }

    /// Get a provider for a specific kind.
    ///
    /// # Errors
    ///
    /// Returns `RuntimeError::ProviderNotSet` if no provider is registered for this kind.
    ///
    /// # Returns
    ///
    /// Returns an Arc clone of the provider, allowing it to be used outside
    /// of the registry's lock scope. This is cheap (just incrementing a reference count).
    pub fn get(&self, kind: ProviderKind) -> Result<Arc<dyn ActionProvider>> {
        self.providers
            .get(&kind)
            .cloned()
            .ok_or_else(|| RuntimeError::ProviderNotSet { kind })
    }

    /// Check if a provider is registered for a specific kind.
    pub fn has(&self, kind: ProviderKind) -> bool {
        self.providers.contains_key(&kind)
    }

    /// Check if an entity has an explicit binding.
    pub fn is_entity_bound(&self, entity: EntityId) -> bool {
        self.entity_mappings.contains_key(&entity)
    }

    /// Get the number of registered providers.
    pub fn provider_count(&self) -> usize {
        self.providers.len()
    }

    /// Get the number of entity bindings.
    pub fn binding_count(&self) -> usize {
        self.entity_mappings.len()
    }

    /// Remove a provider for a specific kind.
    ///
    /// Returns the removed provider, or None if it wasn't registered.
    ///
    /// # Warning
    ///
    /// If entities are bound to this kind or it's the default, they will fail
    /// to resolve providers after removal. Ensure you rebind entities or change
    /// the default before removing.
    pub fn unregister(&mut self, kind: ProviderKind) -> Option<Arc<dyn ActionProvider>> {
        self.providers.remove(&kind)
    }

    /// Clear all entity bindings (they will use default provider).
    pub fn clear_bindings(&mut self) {
        self.entity_mappings.clear();
    }
}

impl Default for ProviderRegistry {
    fn default() -> Self {
        Self::new()
    }
}
