//! Actor catalog loader.
//!
//! Loads actors (both players and NPCs) from RON files with trait profile specs.

use std::path::Path;

use game_core::ActorTemplate;
use serde::Deserialize;

use crate::loaders::{LoadResult, read_file};
use crate::traits::{TraitProfile, TraitProfileSpec, TraitRegistry};

/// Provider kind specification for deserialization.
///
/// This mirrors runtime::ProviderKind but exists in game-content to avoid circular dependency.
/// Converted to runtime::ProviderKind in ContentOracleFactory.
#[derive(Debug, Clone, Deserialize)]
pub enum ProviderKindSpec {
    Interactive(InteractiveKindSpec),
    Ai(AiKindSpec),
    Custom(u32),
}

#[derive(Debug, Clone, Deserialize)]
pub enum InteractiveKindSpec {
    CliInput,
    NetworkInput,
    Replay,
}

#[derive(Debug, Clone, Deserialize)]
pub enum AiKindSpec {
    Wait,
    Aggressive,
    Passive,
    Scripted,
    Utility,
}

/// Loader for actor catalog from RON files.
pub struct ActorLoader;

impl ActorLoader {
    /// Load actor catalog from a RON file with trait registry.
    ///
    /// RON format: Vec<(String, ActorTemplate, ProviderKindSpec, TraitProfileSpec)>
    ///
    /// # Arguments
    ///
    /// * `path` - Path to the RON file
    /// * `trait_registry` - Registry containing trait layer presets
    ///
    /// # Returns
    ///
    /// Returns a Vec of (actor_id, ActorTemplate, ProviderKindSpec, TraitProfile).
    pub fn load(
        path: &Path,
        trait_registry: &TraitRegistry,
    ) -> LoadResult<Vec<(String, ActorTemplate, ProviderKindSpec, TraitProfile)>> {
        let content = read_file(path)?;

        // Deserialize as Vec<(String, ActorTemplate, ProviderKindSpec, TraitProfileSpec)>
        let raw_data: Vec<(String, ActorTemplate, ProviderKindSpec, TraitProfileSpec)> =
            ron::from_str(&content)
                .map_err(|e| anyhow::anyhow!("Failed to parse actor catalog RON: {}", e))?;

        // Resolve TraitProfileSpec to TraitProfile
        let mut actors = Vec::new();
        for (actor_id, template, provider, trait_spec) in raw_data {
            let trait_profile = trait_registry.resolve(&trait_spec).map_err(|e| {
                anyhow::anyhow!(
                    "Failed to resolve trait profile for actor '{}': {}",
                    actor_id,
                    e
                )
            })?;

            actors.push((actor_id, template, provider, trait_profile));
        }

        Ok(actors)
    }
}
