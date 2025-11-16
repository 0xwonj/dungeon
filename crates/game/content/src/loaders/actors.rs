//! Actor catalog loader.
//!
//! Loads actors (both players and NPCs) from RON files with trait profile specs.

use std::path::Path;

use game_core::ActorTemplate;

use crate::loaders::{LoadResult, read_file};
use crate::traits::{TraitProfile, TraitProfileSpec, TraitRegistry};

/// Loader for actor catalog from RON files.
pub struct ActorLoader;

impl ActorLoader {
    /// Load actor catalog from a RON file with trait registry.
    ///
    /// RON format: Vec<(String, ActorTemplate, TraitProfileSpec)>
    ///
    /// # Trait Profile Resolution
    ///
    /// - If `ActorTemplate.trait_profile` is `None`: Resolved from TraitProfileSpec
    /// - If `ActorTemplate.trait_profile` is `Some(...)`: Used directly (TraitProfileSpec ignored)
    ///
    /// # Arguments
    ///
    /// * `path` - Path to the RON file
    /// * `trait_registry` - Registry containing trait layer presets
    ///
    /// # Returns
    ///
    /// Returns a Vec of (actor_id, ActorTemplate, TraitProfile).
    pub fn load(
        path: &Path,
        trait_registry: &TraitRegistry,
    ) -> LoadResult<Vec<(String, ActorTemplate, TraitProfile)>> {
        let content = read_file(path)?;

        // Deserialize as Vec<(String, ActorTemplate, TraitProfileSpec)>
        let raw_data: Vec<(String, ActorTemplate, TraitProfileSpec)> = ron::from_str(&content)
            .map_err(|e| anyhow::anyhow!("Failed to parse actor catalog RON: {}", e))?;

        // Resolve TraitProfileSpec to TraitProfile, Species, and Faction
        let mut actors = Vec::new();
        for (actor_id, mut template, trait_spec) in raw_data {
            // Resolve trait profile
            let final_trait_profile = if let Some(explicit_profile) = template.trait_profile {
                // Case 1: Explicitly specified in template - use it directly
                explicit_profile
            } else {
                // Case 2: Not specified (None) - resolve from TraitProfileSpec
                trait_registry.resolve(&trait_spec).map_err(|e| {
                    anyhow::anyhow!(
                        "Failed to resolve trait profile for actor '{}': {}",
                        actor_id,
                        e
                    )
                })?
            };

            // Resolve species
            let final_species = if let Some(explicit_species) = template.species {
                // Case 1: Explicitly specified in template - use it directly
                explicit_species
            } else {
                // Case 2: Not specified (None) - resolve from TraitProfileSpec.species string
                trait_spec.species.parse().map_err(|_| {
                    anyhow::anyhow!(
                        "Failed to resolve species '{}' for actor '{}': unknown species",
                        trait_spec.species,
                        actor_id
                    )
                })?
            };

            // Resolve faction
            let final_faction = if let Some(explicit_faction) = template.faction {
                // Case 1: Explicitly specified in template - use it directly
                explicit_faction
            } else {
                // Case 2: Not specified (None) - resolve from TraitProfileSpec.faction string
                trait_spec.faction.parse().map_err(|_| {
                    anyhow::anyhow!(
                        "Failed to resolve faction '{}' for actor '{}': unknown faction",
                        trait_spec.faction,
                        actor_id
                    )
                })?
            };

            // Update template with resolved values
            template.trait_profile = Some(final_trait_profile);
            template.species = Some(final_species);
            template.faction = Some(final_faction);

            actors.push((actor_id, template, final_trait_profile));
        }

        Ok(actors)
    }
}
