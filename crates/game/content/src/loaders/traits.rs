// ! Trait layer preset loader.
//!
//! Loads trait layer presets from RON files into a TraitRegistry.

use std::collections::HashMap;
use std::path::Path;

use crate::loaders::{LoadResult, read_file};
use crate::traits::{TraitKind, TraitLayer, TraitRegistry, build_layer_from_pairs};

/// Loads a single trait layer preset file.
///
/// File format: HashMap<String, Vec<(TraitKind, u8)>>
///
/// Example:
/// ```ron
/// {
///     "goblin": [(Bravery, 4), (Mobility, 11)],
///     "orc": [(Bravery, 12), (Aggression, 10)],
/// }
/// ```
fn load_layer_presets(path: &Path) -> LoadResult<HashMap<String, TraitLayer>> {
    let content = read_file(path)?;

    // Deserialize as HashMap<String, Vec<(TraitKind, u8)>>
    let raw_data: HashMap<String, Vec<(TraitKind, u8)>> = ron::from_str(&content)
        .map_err(|e| anyhow::anyhow!("Failed to parse trait layer RON at {:?}: {}", path, e))?;

    // Convert Vec<(TraitKind, u8)> → TraitLayer
    let mut result = HashMap::new();
    for (name, pairs) in raw_data {
        let layer = build_layer_from_pairs(&pairs);
        result.insert(name, layer);
    }

    Ok(result)
}

/// Loads all trait layer presets from the traits directory.
///
/// Expected directory structure:
/// ```text
/// traits/
///   ├── species.ron
///   ├── archetypes.ron
///   ├── factions.ron
///   └── temperaments.ron
/// ```
pub fn load_trait_registry(traits_dir: &Path) -> LoadResult<TraitRegistry> {
    let mut registry = TraitRegistry::new();

    // Load each layer type
    let species_path = traits_dir.join("species.ron");
    let archetypes_path = traits_dir.join("archetypes.ron");
    let factions_path = traits_dir.join("factions.ron");
    let temperaments_path = traits_dir.join("temperaments.ron");

    // Load species
    let species = load_layer_presets(&species_path)?;
    for (name, layer) in species {
        registry.add_species(name, layer);
    }

    // Load archetypes
    let archetypes = load_layer_presets(&archetypes_path)?;
    for (name, layer) in archetypes {
        registry.add_archetype(name, layer);
    }

    // Load factions
    let factions = load_layer_presets(&factions_path)?;
    for (name, layer) in factions {
        registry.add_faction(name, layer);
    }

    // Load temperaments
    let temperaments = load_layer_presets(&temperaments_path)?;
    for (name, layer) in temperaments {
        registry.add_temperament(name, layer);
    }

    Ok(registry)
}
