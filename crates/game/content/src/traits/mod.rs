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

/// The 20 core behavioral traits.
///
/// Each trait ranges from 0 (minimum) to 15 (maximum) at the layer level,
/// and composites to 0..240 at the final profile level.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[repr(u8)]
pub enum TraitKind {
    // ========================================================================
    // Psychology/Temperament
    // ========================================================================
    /// Risk tolerance and fear resistance.
    ///
    /// - High: Fights longer, flees at lower HP
    /// - Low: Flees early, avoids danger
    Bravery = 0,

    /// Plan adherence, panic resistance, focus.
    ///
    /// - High: Sticks to tactics, resists distraction
    /// - Low: Easily disrupted, changes plans
    Discipline = 1,

    /// Initiative and pursuit willingness.
    ///
    /// - High: Initiates combat, chases aggressively
    /// - Low: Passive, defensive
    Aggression = 2,

    /// Caution and risk avoidance.
    ///
    /// - High: Checks corners, avoids traps, prepares
    /// - Low: Rushes in, ignores preparation
    Caution = 3,

    /// Exploration and investigation drive.
    ///
    /// - High: Investigates sounds, searches thoroughly
    /// - Low: Ignores distractions
    Curiosity = 4,

    /// Ally protection and call response.
    ///
    /// - High: Protects allies, responds to calls
    /// - Low: Self-preservation over allies
    Loyalty = 5,

    /// Loot obsession and resource hoarding.
    ///
    /// - High: Prioritizes looting, guards treasure
    /// - Low: Ignores loot
    Greed = 6,

    /// Spontaneity and explosive behavior.
    ///
    /// - High: Acts on impulse, sudden actions
    /// - Low: Deliberate, controlled
    Impulsivity = 7,

    /// Non-lethal and negotiation preference.
    ///
    /// - High: Prefers capture, negotiates, shows mercy
    /// - Low: Lethal force, no negotiation
    Empathy = 8,

    // ========================================================================
    // Cognition/Tactics
    // ========================================================================
    /// Vision, hearing, stealth detection baseline.
    ///
    /// - High: Wide vision, detects stealth easily
    /// - Low: Narrow perception, easily flanked
    Perception = 9,

    /// Positioning, cooldown timing, synergy awareness.
    ///
    /// - High: Optimal positioning, uses abilities well
    /// - Low: Poor positioning, wastes cooldowns
    TacticalSense = 10,

    /// Alertness retention and suspicion accumulation.
    ///
    /// - High: Remembers events, stays alert longer
    /// - Low: Forgets quickly, drops guard
    Memory = 11,

    // ========================================================================
    // Physical/Movement/Range
    // ========================================================================
    /// Acceleration, dodge, terrain adaptation.
    ///
    /// - High: Fast movement, good dodging
    /// - Low: Slow, clumsy
    Mobility = 12,

    /// Long pursuit, patrol, alertness duration.
    ///
    /// - High: Chases long distances, patrols far
    /// - Low: Gives up quickly, short patrols
    Stamina = 13,

    /// Combat range preference (0=melee, 15=long range/kiting).
    ///
    /// - High: Prefers ranged combat, kites
    /// - Low: Prefers melee engagement
    PreferredRange = 14,

    // ========================================================================
    // Social/Norms/Territory
    // ========================================================================
    /// Refusal to leave territory, area rage.
    ///
    /// - High: Never leaves home area, enraged if invaded
    /// - Low: Roams freely
    Territoriality = 15,

    /// Doctrine/command/formation adherence.
    ///
    /// - High: Follows orders strictly, maintains formation
    /// - Low: Acts independently
    Obedience = 16,

    /// Frontal combat preference, cowardly tactic aversion.
    ///
    /// - High: Prefers fair fights, dislikes ambushes
    /// - Low: Uses any tactic to win
    Honor = 17,

    /// Magic/light/taboo biases.
    ///
    /// - High: Affected by curses, fears holy symbols
    /// - Low: Rational, ignores superstitions
    Superstition = 18,

    /// Intimidation and command success tendency.
    ///
    /// - High: Successfully intimidates, commands respect
    /// - Low: Fails to intimidate
    Dominance = 19,
}

impl TraitKind {
    /// Total number of traits.
    pub const COUNT: usize = 20;

    /// Returns all trait kinds in order.
    pub const fn all() -> [TraitKind; Self::COUNT] {
        [
            TraitKind::Bravery,
            TraitKind::Discipline,
            TraitKind::Aggression,
            TraitKind::Caution,
            TraitKind::Curiosity,
            TraitKind::Loyalty,
            TraitKind::Greed,
            TraitKind::Impulsivity,
            TraitKind::Empathy,
            TraitKind::Perception,
            TraitKind::TacticalSense,
            TraitKind::Memory,
            TraitKind::Mobility,
            TraitKind::Stamina,
            TraitKind::PreferredRange,
            TraitKind::Territoriality,
            TraitKind::Obedience,
            TraitKind::Honor,
            TraitKind::Superstition,
            TraitKind::Dominance,
        ]
    }

    /// Returns the trait as a u8 index.
    #[inline]
    pub const fn as_index(self) -> usize {
        self as usize
    }
}

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

/// Final composed trait profile for an NPC.
///
/// Each value is 0..240 (computed from 4 layers with weights summing to 16).
/// This is the profile used by behavior tree nodes to compute thresholds.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct TraitProfile {
    /// Composed trait values (0..240).
    values: [u8; TraitKind::COUNT],
}

impl Default for TraitProfile {
    fn default() -> Self {
        Self {
            values: [120; TraitKind::COUNT], // Middle value (neutral)
        }
    }
}

impl TraitProfile {
    /// Creates a profile with all traits set to a specific value.
    pub fn uniform(value: u8) -> Self {
        let clamped = if value > 240 { 240 } else { value };
        Self {
            values: [clamped; TraitKind::COUNT],
        }
    }

    /// Composes a trait profile from 4 layers and weights.
    ///
    /// Formula: `value[trait] = Σ(layer[i][trait] * weight[trait][i])` for i in 0..4
    /// Result range: 0..240 (since max is 15 * 16 = 240)
    pub fn compose(
        species: &TraitLayer,
        archetype: &TraitLayer,
        faction: &TraitLayer,
        temperament: &TraitLayer,
        weights: &TraitWeights,
    ) -> Self {
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

        Self { values }
    }

    /// Gets the composed value for a specific trait (0..240).
    #[inline]
    pub fn get(&self, trait_kind: TraitKind) -> u8 {
        self.values[trait_kind.as_index()]
    }

    /// Linear interpolation helper for threshold computation.
    ///
    /// Maps trait value (0..240) to range [min..max].
    ///
    /// # Arguments
    ///
    /// * `trait_kind` - The trait to use
    /// * `min` - Minimum output value (when trait = 0)
    /// * `max` - Maximum output value (when trait = 240)
    ///
    /// # Returns
    ///
    /// Interpolated value as u8.
    pub fn lerp_u8(&self, trait_kind: TraitKind, min: u8, max: u8) -> u8 {
        let value = self.get(trait_kind);
        lerp_u8(min, max, value, 240)
    }

    /// Linear interpolation for percentage thresholds.
    ///
    /// Maps trait value (0..240) to range [min..max] as f32.
    ///
    /// # Arguments
    ///
    /// * `trait_kind` - The trait to use
    /// * `min` - Minimum output value (when trait = 0)
    /// * `max` - Maximum output value (when trait = 240)
    ///
    /// # Returns
    ///
    /// Interpolated value as f32 (typically 0.0..1.0 for percentages).
    pub fn lerp_f32(&self, trait_kind: TraitKind, min: f32, max: f32) -> f32 {
        let value = self.get(trait_kind);
        lerp_f32(min, max, value, 240)
    }

    /// Inverted linear interpolation (high trait → low output).
    ///
    /// Maps trait value (0..240) to range [max..min] (reversed).
    /// Useful for traits like Bravery where high value = low flee threshold.
    ///
    /// # Arguments
    ///
    /// * `trait_kind` - The trait to use
    /// * `min` - Minimum output value (when trait = 240)
    /// * `max` - Maximum output value (when trait = 0)
    ///
    /// # Returns
    ///
    /// Interpolated value as f32.
    pub fn lerp_inverted(&self, trait_kind: TraitKind, min: f32, max: f32) -> f32 {
        let value = 240 - self.get(trait_kind); // Invert
        lerp_f32(min, max, value, 240)
    }
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
