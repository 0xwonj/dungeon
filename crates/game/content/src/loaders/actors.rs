//! Actor catalog loader.
//!
//! Loads actors (both players and NPCs) from RON files.
//! Trait profiles are resolved from species/faction/archetype/temperament fields.

use std::path::Path;

use game_core::ActorTemplate;

use crate::loaders::{LoadResult, read_file};
use crate::traits::TraitRegistry;

/// Loader for actor catalog from RON files.
pub struct ActorLoader;

impl ActorLoader {
    /// Load actor catalog from a RON file with trait registry.
    ///
    /// RON format: Vec<(String, ActorTemplate)>
    ///
    /// # Trait Profile Resolution
    ///
    /// Each ActorTemplate contains:
    /// - `species`: Species enum (e.g., Human, Goblin)
    /// - `faction`: Faction enum (e.g., Player, Hostile)
    /// - `archetype`: String reference to archetype trait layer
    /// - `temperament`: String reference to temperament trait layer
    /// - `trait_profile`: Optional explicit trait profile
    ///
    /// If `trait_profile` is `None`, the loader resolves it from the four components.
    /// After loading, all templates have `trait_profile` set to `Some(...)`.
    ///
    /// # Arguments
    ///
    /// * `path` - Path to the RON file
    /// * `trait_registry` - Registry containing trait layer presets
    ///
    /// # Returns
    ///
    /// Returns a Vec of (actor_id, ActorTemplate) with trait_profile resolved.
    pub fn load(
        path: &Path,
        trait_registry: &TraitRegistry,
    ) -> LoadResult<Vec<(String, ActorTemplate)>> {
        let content = read_file(path)?;

        // Deserialize as Vec<(String, ActorTemplate)>
        let raw_data: Vec<(String, ActorTemplate)> = ron::from_str(&content)
            .map_err(|e| anyhow::anyhow!("Failed to parse actor catalog RON: {}", e))?;

        // Resolve trait profile if not explicitly specified
        let mut actors = Vec::new();
        for (actor_id, mut template) in raw_data {
            if template.trait_profile.is_none() {
                // Build trait profile from the four components
                let trait_profile = trait_registry
                    .resolve_from_components(
                        template.species,
                        template.faction,
                        &template.archetype,
                        &template.temperament,
                    )
                    .map_err(|e| {
                        anyhow::anyhow!(
                            "Failed to resolve trait profile for actor '{}': {}",
                            actor_id,
                            e
                        )
                    })?;

                template.trait_profile = Some(trait_profile);
            }

            actors.push((actor_id, template));
        }

        Ok(actors)
    }
}
