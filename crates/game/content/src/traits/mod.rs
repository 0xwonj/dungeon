//! NPC trait system for behavioral parameterization.
//!
//! This module provides a layered trait system where each NPC's behavioral
//! parameters are computed from multiple layers (Species, Archetype, Faction,
//! Temperament) combined with weighted averages.
//!
//! # Design Principles
//!
//! - **Integer-only**: All calculations use u8 values (0..255) for ZK-friendliness
//! - **Delta-based**: Unused traits default to 0, only specify what differs
//! - **Layered composition**: 4 layers combine with per-trait weights (sum=16)
//! - **Deterministic**: Pure functions, no randomness, no floating point

// Re-export core trait types from game-core
pub use game_core::{Faction, Species, TraitKind, TraitProfile};

// Note: TraitKind, TraitProfile, Species, and Faction now live in game-core at crates/game/core/src/traits.rs
// for challenge verification support (needed in ActorState).
// The composition logic (TraitLayer, TraitWeights, compose()) stays here in game-content.

/// A single layer of trait values (Species, Archetype, Faction, or Temperament).
///
/// Each value is 0..15 (4 bits conceptually, stored as u8).
/// Delta-based: only non-zero values need to be specified.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct TraitLayer {
    /// Raw trait values (0..15). Values > 15 are clamped during construction.
    values: [u8; TraitKind::COUNT],
}

impl Default for TraitLayer {
    fn default() -> Self {
        Self {
            values: [0; TraitKind::COUNT],
        }
    }
}

impl TraitLayer {
    /// Creates a new layer with all values set to 0.
    pub const fn zero() -> Self {
        Self {
            values: [0; TraitKind::COUNT],
        }
    }

    /// Creates a layer from raw values, clamping to 0..15.
    pub fn from_raw(values: [u8; TraitKind::COUNT]) -> Self {
        let mut clamped = [0u8; TraitKind::COUNT];
        let mut i = 0;
        while i < TraitKind::COUNT {
            clamped[i] = if values[i] > 15 { 15 } else { values[i] };
            i += 1;
        }
        Self { values: clamped }
    }

    /// Gets the value for a specific trait (0..15).
    #[inline]
    pub fn get(&self, trait_kind: TraitKind) -> u8 {
        self.values[trait_kind.as_index()]
    }

    /// Sets the value for a specific trait, clamping to 0..15.
    #[inline]
    pub fn set(&mut self, trait_kind: TraitKind, value: u8) {
        self.values[trait_kind.as_index()] = if value > 15 { 15 } else { value };
    }

    /// Returns a builder for constructing layers fluently.
    pub fn builder() -> TraitLayerBuilder {
        TraitLayerBuilder::default()
    }
}

/// Builder for constructing trait layers fluently.
#[derive(Default)]
pub struct TraitLayerBuilder {
    values: [u8; TraitKind::COUNT],
}

impl TraitLayerBuilder {
    /// Sets a trait value (will be clamped to 0..15).
    pub fn set(mut self, trait_kind: TraitKind, value: u8) -> Self {
        self.values[trait_kind.as_index()] = if value > 15 { 15 } else { value };
        self
    }

    /// Builds the layer.
    pub fn build(self) -> TraitLayer {
        TraitLayer::from_raw(self.values)
    }
}

/// Per-trait weights for combining layers.
///
/// Each trait has 4 weights (one per layer: Species, Archetype, Faction, Temperament).
/// Weights must sum to 16 for each trait.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct TraitWeights {
    /// Weights for each trait. Each array element is [species, archetype, faction, temperament].
    weights: [[u8; 4]; TraitKind::COUNT],
}

impl TraitWeights {
    /// Creates weights from raw data.
    ///
    /// # Panics
    ///
    /// Panics if any trait's weights do not sum to exactly 16.
    pub fn from_raw(weights: [[u8; 4]; TraitKind::COUNT]) -> Self {
        // Validate that all weights sum to 16
        for (i, w) in weights.iter().enumerate() {
            let sum = w[0] as u32 + w[1] as u32 + w[2] as u32 + w[3] as u32;
            assert_eq!(sum, 16, "Trait {} weights must sum to 16, got {}", i, sum);
        }
        Self { weights }
    }

    /// Gets the weights for a specific trait [species, archetype, faction, temperament].
    #[inline]
    pub fn get(&self, trait_kind: TraitKind) -> [u8; 4] {
        self.weights[trait_kind.as_index()]
    }

    /// Returns the default weight table from the design document.
    pub fn default_weights() -> Self {
        Self::from_raw([
            [6, 5, 2, 3],  // Bravery
            [3, 6, 5, 2],  // Discipline
            [4, 6, 3, 3],  // Aggression
            [4, 5, 3, 4],  // Caution
            [4, 4, 2, 6],  // Curiosity
            [3, 3, 7, 3],  // Loyalty
            [5, 3, 4, 4],  // Greed
            [3, 3, 2, 8],  // Impulsivity
            [3, 3, 4, 6],  // Empathy
            [8, 3, 2, 3],  // Perception
            [2, 8, 4, 2],  // TacticalSense
            [3, 5, 5, 3],  // Memory
            [7, 5, 2, 2],  // Mobility
            [6, 5, 3, 2],  // Stamina
            [2, 10, 2, 2], // PreferredRange
            [4, 3, 7, 2],  // Territoriality
            [2, 3, 9, 2],  // Obedience
            [2, 4, 8, 2],  // Honor
            [6, 2, 6, 2],  // Superstition
            [3, 5, 4, 4],  // Dominance
        ])
    }
}

impl Default for TraitWeights {
    fn default() -> Self {
        Self::default_weights()
    }
}

/// Composes a trait profile from 4 layers and weights.
///
/// This is a standalone function since TraitProfile is defined in game-core.
///
/// Formula: `value[trait] = Σ(layer[i][trait] * weight[trait][i])` for i in 0..4
/// Result range: 0..240 (since max is 15 * 16 = 240)
pub fn compose_trait_profile(
    species: &TraitLayer,
    archetype: &TraitLayer,
    faction: &TraitLayer,
    temperament: &TraitLayer,
    weights: &TraitWeights,
) -> TraitProfile {
    let mut values = [0u8; TraitKind::COUNT];

    for trait_kind in TraitKind::all() {
        let idx = trait_kind.as_index();
        let w = weights.get(trait_kind);

        // Compute weighted sum: w[0]*species + w[1]*archetype + w[2]*faction + w[3]*temperament
        let sum = (w[0] as u32 * species.get(trait_kind) as u32)
            + (w[1] as u32 * archetype.get(trait_kind) as u32)
            + (w[2] as u32 * faction.get(trait_kind) as u32)
            + (w[3] as u32 * temperament.get(trait_kind) as u32);

        // Result is guaranteed to be 0..240 (max: 16 * 15 = 240)
        values[idx] = sum as u8;
    }

    TraitProfile::from_raw(values)
}

// ============================================================================
// Helper Functions
// ============================================================================

/// Linear interpolation for u8 values.
///
/// # Arguments
///
/// * `min` - Minimum output
/// * `max` - Maximum output
/// * `value` - Input value (0..value_max)
/// * `value_max` - Maximum input value
///
/// # Returns
///
/// `min + (max - min) * value / value_max`
#[inline]
pub fn lerp_u8(min: u8, max: u8, value: u8, value_max: u8) -> u8 {
    if max >= min {
        let range = (max - min) as u32;
        let scaled = (range * value as u32) / value_max as u32;
        min + scaled as u8
    } else {
        let range = (min - max) as u32;
        let scaled = (range * value as u32) / value_max as u32;
        min - scaled as u8
    }
}

/// Linear interpolation for f32 values.
///
/// # Arguments
///
/// * `min` - Minimum output
/// * `max` - Maximum output
/// * `value` - Input value (0..value_max)
/// * `value_max` - Maximum input value
///
/// # Returns
///
/// `min + (max - min) * (value / value_max)`
#[inline]
pub fn lerp_f32(min: f32, max: f32, value: u8, value_max: u8) -> f32 {
    let t = value as f32 / value_max as f32;
    min + (max - min) * t
}

// ============================================================================
// Trait Registry and Preset System
// ============================================================================

/// Trait profile specification referencing named presets.
///
/// This is used in NPC definitions to reference trait layers by name
/// instead of repeating trait values.
#[derive(Clone, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct TraitProfileSpec {
    pub species: String,
    pub archetype: String,
    pub faction: String,
    pub temperament: String,
}

impl TraitProfileSpec {
    /// Creates a new trait profile specification.
    pub fn new(
        species: impl Into<String>,
        archetype: impl Into<String>,
        faction: impl Into<String>,
        temperament: impl Into<String>,
    ) -> Self {
        Self {
            species: species.into(),
            archetype: archetype.into(),
            faction: faction.into(),
            temperament: temperament.into(),
        }
    }
}

/// Registry of named trait layer presets.
///
/// Stores predefined trait layers by name for each of the 4 layer types
/// (species, archetype, faction, temperament).
pub struct TraitRegistry {
    species: std::collections::HashMap<String, TraitLayer>,
    archetypes: std::collections::HashMap<String, TraitLayer>,
    factions: std::collections::HashMap<String, TraitLayer>,
    temperaments: std::collections::HashMap<String, TraitLayer>,
    weights: TraitWeights,
}

impl TraitRegistry {
    /// Creates a new empty registry with default weights.
    pub fn new() -> Self {
        Self {
            species: std::collections::HashMap::new(),
            archetypes: std::collections::HashMap::new(),
            factions: std::collections::HashMap::new(),
            temperaments: std::collections::HashMap::new(),
            weights: TraitWeights::default(),
        }
    }

    /// Creates a new registry with custom weights.
    pub fn with_weights(weights: TraitWeights) -> Self {
        Self {
            species: std::collections::HashMap::new(),
            archetypes: std::collections::HashMap::new(),
            factions: std::collections::HashMap::new(),
            temperaments: std::collections::HashMap::new(),
            weights,
        }
    }

    /// Adds a species layer preset.
    pub fn add_species(&mut self, name: String, layer: TraitLayer) {
        self.species.insert(name, layer);
    }

    /// Adds an archetype layer preset.
    pub fn add_archetype(&mut self, name: String, layer: TraitLayer) {
        self.archetypes.insert(name, layer);
    }

    /// Adds a faction layer preset.
    pub fn add_faction(&mut self, name: String, layer: TraitLayer) {
        self.factions.insert(name, layer);
    }

    /// Adds a temperament layer preset.
    pub fn add_temperament(&mut self, name: String, layer: TraitLayer) {
        self.temperaments.insert(name, layer);
    }

    /// Resolves a trait profile specification into a composed TraitProfile.
    ///
    /// # Errors
    ///
    /// Returns an error if any of the referenced preset names are not found.
    pub fn resolve(&self, spec: &TraitProfileSpec) -> Result<TraitProfile, String> {
        let species = self
            .species
            .get(&spec.species)
            .ok_or_else(|| format!("Species preset '{}' not found", spec.species))?;

        let archetype = self
            .archetypes
            .get(&spec.archetype)
            .ok_or_else(|| format!("Archetype preset '{}' not found", spec.archetype))?;

        let faction = self
            .factions
            .get(&spec.faction)
            .ok_or_else(|| format!("Faction preset '{}' not found", spec.faction))?;

        let temperament = self
            .temperaments
            .get(&spec.temperament)
            .ok_or_else(|| format!("Temperament preset '{}' not found", spec.temperament))?;

        Ok(compose_trait_profile(
            species,
            archetype,
            faction,
            temperament,
            &self.weights,
        ))
    }
}

impl Default for TraitRegistry {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================================
// Helper Functions
// ============================================================================

/// Helper function to build a TraitLayer from sparse data Vec<(TraitKind, u8)>.
///
/// This is used during deserialization to convert sparse RON format into TraitLayer.
pub fn build_layer_from_pairs(pairs: &[(TraitKind, u8)]) -> TraitLayer {
    let mut builder = TraitLayer::builder();
    for (trait_kind, value) in pairs {
        if *value > 15 {
            panic!(
                "Trait {:?} has invalid value {} (must be 0..=15)",
                trait_kind, value
            );
        }
        builder = builder.set(*trait_kind, *value);
    }
    builder.build()
}
