//! Actor template definitions and oracle interface.
//!
//! This module provides `ActorTemplate` for defining all actors (including player)
//! in a data-driven way. Templates can be serialized from RON files and spawned
//! into `ActorState` instances.
//!
//! The `ActorOracle` trait allows runtime systems to provide actor templates
//! by definition ID (e.g., "player", "goblin_scout").

use arrayvec::ArrayVec;

use crate::config::GameConfig;
use crate::provider::ProviderKind;
use crate::state::{
    ActionAbility, ActorState, EntityId, Equipment, InventoryState, PassiveAbility, Position,
    StatusEffects,
};
use crate::stats::{CoreStats, ResourceCurrent, StatsSnapshot, compute_actor_bonuses};
use crate::traits::{Faction, Species, TraitProfile};

/// Actor template defining all ActorState fields except id/position/scheduling.
///
/// This type can be serialized directly from RON files and used to spawn
/// actors with proper initialization. Resources are derived from core_stats
/// at spawn time.
///
/// # Trait Resolution
///
/// - `species` and `faction` are always explicitly specified (enums)
/// - `archetype` and `temperament` are string references to trait layer catalogs
/// - `trait_profile`:
///   - `None`: ActorLoader resolves from species/faction/archetype/temperament
///   - `Some(...)`: Explicitly specified (overrides resolution)
/// - After loading, `trait_profile` is always `Some(...)`
#[derive(Clone, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct ActorTemplate {
    pub core_stats: CoreStats,
    pub equipment: Equipment,
    pub status_effects: StatusEffects,
    pub actions: ArrayVec<ActionAbility, { GameConfig::MAX_ACTIONS }>,
    pub passives: ArrayVec<PassiveAbility, { GameConfig::MAX_PASSIVES }>,
    pub inventory: InventoryState,
    pub provider_kind: ProviderKind,

    /// Species (biological/existential identity).
    pub species: Species,

    /// Faction (relationship/allegiance).
    pub faction: Faction,

    /// Archetype trait layer reference (e.g., "warrior", "scout", "mage").
    /// Resolved by ActorLoader from archetype trait catalog.
    pub archetype: String,

    /// Temperament trait layer reference (e.g., "aggressive", "cowardly", "cautious").
    /// Resolved by ActorLoader from temperament trait catalog.
    pub temperament: String,

    /// Behavioral trait profile for AI decision making.
    ///
    /// - `None`: Will be resolved by ActorLoader from species/faction/archetype/temperament
    /// - `Some(...)`: Explicitly specified (overrides automatic resolution)
    ///
    /// After loading via ActorLoader, this is always `Some(...)`.
    pub trait_profile: Option<TraitProfile>,
}

impl ActorTemplate {
    /// Create a new actor from this template with the given id and position.
    ///
    /// Resources are automatically derived from core_stats.
    /// Bonuses are computed from equipment, status effects, and passives.
    ///
    /// # Arguments
    ///
    /// * `id` - The entity ID for the new actor
    /// * `position` - The spawn position
    ///
    /// # Panics
    ///
    /// Panics if `trait_profile` is `None`. Templates must have resolved trait profiles
    /// before spawning (typically done by ActorLoader).
    pub fn to_actor(&self, id: EntityId, position: Position) -> ActorState {
        // Compute bonuses from equipment, status effects, actions, and passives
        let bonuses = compute_actor_bonuses();

        // Compute resource maximums from core stats + bonuses
        let snapshot = StatsSnapshot::create(
            &self.core_stats,
            &bonuses,
            &ResourceCurrent::new(0, 0, 0), // Dummy for computation
        );

        let resources = ResourceCurrent::at_max(&snapshot.resource_max);

        ActorState {
            id,
            position: Some(position),
            core_stats: self.core_stats.clone(),
            resources,
            equipment: self.equipment.clone(),
            status_effects: self.status_effects.clone(),
            actions: self.actions.clone(),
            passives: self.passives.clone(),
            bonuses,
            inventory: self.inventory.clone(),
            provider_kind: self.provider_kind,
            trait_profile: self.trait_profile.expect(
                "ActorTemplate.trait_profile must be Some(...) when spawning. \
                 This should have been resolved by ActorLoader.",
            ),
            species: self.species,
            faction: self.faction,
            ready_at: None,
        }
    }

    /// Create a builder for constructing actor templates
    pub fn builder() -> ActorTemplateBuilder {
        ActorTemplateBuilder::default()
    }

    /// Create a test actor template with default stats
    pub fn test_actor() -> Self {
        ActorTemplate::builder().build()
    }
}

/// Builder for constructing actor templates.
#[derive(Default)]
pub struct ActorTemplateBuilder {
    stats: Option<CoreStats>,
    equipment: Option<Equipment>,
    status_effects: Option<StatusEffects>,
    inventory: Option<InventoryState>,
    actions: Option<ArrayVec<ActionAbility, { GameConfig::MAX_ACTIONS }>>,
    passives: Option<ArrayVec<PassiveAbility, { GameConfig::MAX_PASSIVES }>>,
    provider_kind: Option<ProviderKind>,
    species: Option<Species>,
    faction: Option<Faction>,
    archetype: Option<String>,
    temperament: Option<String>,
    trait_profile: Option<TraitProfile>,
}

impl ActorTemplateBuilder {
    /// Set base stats
    pub fn stats(mut self, stats: CoreStats) -> Self {
        self.stats = Some(stats);
        self
    }

    /// Set equipment
    pub fn equipment(mut self, equipment: Equipment) -> Self {
        self.equipment = Some(equipment);
        self
    }

    /// Set status effects
    pub fn status_effects(mut self, status_effects: StatusEffects) -> Self {
        self.status_effects = Some(status_effects);
        self
    }

    /// Set inventory
    pub fn inventory(mut self, inv: InventoryState) -> Self {
        self.inventory = Some(inv);
        self
    }

    /// Set action abilities
    pub fn actions(
        mut self,
        actions: ArrayVec<ActionAbility, { GameConfig::MAX_ACTIONS }>,
    ) -> Self {
        self.actions = Some(actions);
        self
    }

    /// Set passive abilities
    pub fn passives(
        mut self,
        passives: ArrayVec<PassiveAbility, { GameConfig::MAX_PASSIVES }>,
    ) -> Self {
        self.passives = Some(passives);
        self
    }

    /// Set provider kind
    pub fn provider_kind(mut self, provider_kind: ProviderKind) -> Self {
        self.provider_kind = Some(provider_kind);
        self
    }

    /// Set species
    pub fn species(mut self, species: Species) -> Self {
        self.species = Some(species);
        self
    }

    /// Set faction
    pub fn faction(mut self, faction: Faction) -> Self {
        self.faction = Some(faction);
        self
    }

    /// Set archetype
    pub fn archetype(mut self, archetype: impl Into<String>) -> Self {
        self.archetype = Some(archetype.into());
        self
    }

    /// Set temperament
    pub fn temperament(mut self, temperament: impl Into<String>) -> Self {
        self.temperament = Some(temperament.into());
        self
    }

    /// Set trait profile
    pub fn trait_profile(mut self, trait_profile: TraitProfile) -> Self {
        self.trait_profile = Some(trait_profile);
        self
    }

    /// Build the actor template
    pub fn build(self) -> ActorTemplate {
        use crate::provider::{AiKind, ProviderKind};

        ActorTemplate {
            core_stats: self.stats.unwrap_or_default(),
            equipment: self.equipment.unwrap_or_else(Equipment::empty),
            status_effects: self.status_effects.unwrap_or_else(StatusEffects::empty),
            actions: self.actions.unwrap_or_default(),
            passives: self.passives.unwrap_or_default(),
            inventory: self.inventory.unwrap_or_default(),
            provider_kind: self.provider_kind.unwrap_or(ProviderKind::Ai(AiKind::Wait)),
            species: self.species.unwrap_or_default(),
            faction: self.faction.unwrap_or_default(),
            archetype: self.archetype.unwrap_or_else(|| "none".to_string()),
            temperament: self.temperament.unwrap_or_else(|| "neutral".to_string()),
            trait_profile: self.trait_profile,
        }
    }
}

/// Oracle providing actor template data for entity creation.
///
/// This trait provides access to actor templates (both player and NPCs)
/// by definition ID. Runtime systems implement this to provide static
/// actor data from configuration files.
pub trait ActorOracle: Send + Sync {
    /// Returns the actor template for a given definition ID.
    ///
    /// # Arguments
    ///
    /// * `def_id` - Definition identifier (e.g., "player", "goblin_scout", "orc_warrior")
    ///
    /// # Returns
    ///
    /// The actor template if found, None otherwise.
    fn template(&self, def_id: &str) -> Option<ActorTemplate>;

    /// Returns all available actor definition IDs.
    ///
    /// This is used for creating snapshots that need to capture all actors.
    /// For snapshot-backed oracles, this may return an empty vec if not needed.
    #[cfg(feature = "std")]
    fn all_ids(&self) -> Vec<String> {
        Vec::new()
    }
}
