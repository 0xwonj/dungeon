//! Bonus application system following the layered stack architecture.
//!
//! This module implements the bonus calculation stack:
//! Flat → %Inc → More → Less → Clamp → Conditions
//!
//! All stat layers (Core, Derived, Speed, Modifiers, Resources) use this
//! same calculation order for consistency and determinism.

/// A single bonus that can be applied to a stat value.
///
/// Bonuses are categorized by their application type:
/// - **Flat**: Additive bonuses applied first (e.g., +5 STR from equipment)
/// - **Increased**: Percentage increases, summed then multiplied (e.g., +20% STR)
/// - **More**: Sequential multipliers applied individually (e.g., ×1.5)
/// - **Less**: Sequential reductions applied individually (e.g., ×0.9)
#[derive(Clone, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum Bonus {
    Flat(i32),

    Increased(i32),

    More(i32),

    Less(i32),
}

impl Bonus {
    /// Create a flat bonus
    pub fn flat(value: i32) -> Self {
        Bonus::Flat(value)
    }

    /// Create a percentage increase bonus (20 = +20%)
    pub fn increased(percent: i32) -> Self {
        Bonus::Increased(percent)
    }

    /// Create a "more" multiplier (50 = ×1.5)
    pub fn more(percent: i32) -> Self {
        Bonus::More(percent)
    }

    /// Create a "less" multiplier (10 = ×0.9)
    pub fn less(percent: i32) -> Self {
        Bonus::Less(percent)
    }
}

/// A collection of bonuses that will be applied in the correct order.
///
/// The stack guarantees the following application order:
/// 1. Flat bonuses (summed)
/// 2. Increased bonuses (summed, then multiplied)
/// 3. More multipliers (applied sequentially)
/// 4. Less multipliers (applied sequentially)
/// 5. Clamp to bounds
/// 6. Conditions (applied by caller after this stack)
#[derive(Clone, Debug, Default, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct BonusStack {
    bonuses: Vec<Bonus>,
}

impl BonusStack {
    /// Create a new empty bonus stack
    pub fn new() -> Self {
        Self {
            bonuses: Vec::new(),
        }
    }

    /// Add a bonus to the stack
    pub fn add(&mut self, bonus: Bonus) {
        self.bonuses.push(bonus);
    }

    /// Add multiple bonuses at once
    pub fn extend(&mut self, bonuses: impl IntoIterator<Item = Bonus>) {
        self.bonuses.extend(bonuses);
    }

    /// Apply all bonuses to a base value with clamping
    ///
    /// # Arguments
    /// * `base` - The base value to apply bonuses to
    /// * `min` - Minimum allowed value (clamp lower bound)
    /// * `max` - Maximum allowed value (clamp upper bound)
    ///
    /// # Returns
    /// The final value after applying all bonuses and clamping
    ///
    /// # Formula
    /// ```text
    /// result = clamp((base + flat_sum) × (1 + inc_sum/100) × more_product × less_product, min, max)
    /// ```
    pub fn apply(&self, base: i32, min: i32, max: i32) -> i32 {
        // Step 1: Sum all flat bonuses
        let flat_sum: i32 = self
            .bonuses
            .iter()
            .filter_map(|b| match b {
                Bonus::Flat(v) => Some(*v),
                _ => None,
            })
            .sum();

        // Step 2: Sum all %Inc bonuses
        let inc_sum: i32 = self
            .bonuses
            .iter()
            .filter_map(|b| match b {
                Bonus::Increased(p) => Some(*p),
                _ => None,
            })
            .sum();

        // Apply base + flat, then %Inc
        let after_inc = if inc_sum == 0 {
            base + flat_sum
        } else {
            let multiplier = 100 + inc_sum;
            ((base + flat_sum) * multiplier) / 100
        };

        // Step 3: Apply More multipliers sequentially
        let after_more = self
            .bonuses
            .iter()
            .filter_map(|b| match b {
                Bonus::More(p) => Some(*p),
                _ => None,
            })
            .fold(after_inc, |acc, more_percent| {
                let multiplier = 100 + more_percent;
                (acc * multiplier) / 100
            });

        // Step 4: Apply Less multipliers sequentially
        let after_less = self
            .bonuses
            .iter()
            .filter_map(|b| match b {
                Bonus::Less(p) => Some(*p),
                _ => None,
            })
            .fold(after_more, |acc, less_percent| {
                let multiplier = 100 - less_percent;
                (acc * multiplier) / 100
            });

        // Step 5: Clamp to bounds
        after_less.clamp(min, max)
    }

    /// Apply bonuses without clamping
    ///
    /// Useful when you want to apply clamping separately or
    /// when there are no natural bounds.
    pub fn apply_unclamped(&self, base: i32) -> i32 {
        self.apply(base, i32::MIN, i32::MAX)
    }

    /// Check if the stack is empty
    pub fn is_empty(&self) -> bool {
        self.bonuses.is_empty()
    }

    /// Get the number of bonuses in the stack
    pub fn len(&self) -> usize {
        self.bonuses.len()
    }
}

/// Builder for constructing bonus stacks
impl BonusStack {
    /// Add a flat bonus
    pub fn flat(mut self, value: i32) -> Self {
        self.add(Bonus::flat(value));
        self
    }

    /// Add a percentage increase
    pub fn increased(mut self, percent: i32) -> Self {
        self.add(Bonus::increased(percent));
        self
    }

    /// Add a "more" multiplier
    pub fn more(mut self, percent: i32) -> Self {
        self.add(Bonus::more(percent));
        self
    }

    /// Add a "less" multiplier
    pub fn less(mut self, percent: i32) -> Self {
        self.add(Bonus::less(percent));
        self
    }
}

/// Trait for stat layers that follow the Base -> Bonuses -> Final pattern.
///
/// All stat layers in the system follow the same computational pattern:
/// 1. **Base**: Raw or computed base values
/// 2. **Bonuses**: Modifiers from equipment, buffs, debuffs, etc.
/// 3. **Final**: Base values with bonuses applied
///
/// This trait unifies this pattern across all layers, enabling:
/// - Consistent API across layers
/// - Generic functions that work with any layer
/// - Clear separation of concerns
///
/// # Type Parameters
///
/// - `Base`: The input type (e.g., `CoreStats`, `CoreEffective`)
/// - `Bonuses`: The bonus holder type (e.g., `CoreStatBonuses`)
/// - `Final`: The output type (e.g., `CoreEffective`, `DerivedStats`)
///
/// # Layer Architecture
///
/// ```text
/// Layer 1: CoreStats + CoreStatBonuses -> CoreEffective
/// Layer 2: CoreEffective + DerivedBonuses -> DerivedStats
/// Layer 3: CoreEffective + SpeedBonuses -> SpeedStats
/// Layer 4: CoreEffective + ModifierBonuses -> StatModifiers
/// Layer 5: CoreEffective + ResourceBonuses -> ResourceMaximums
/// ```
pub trait StatLayer {
    /// The base/input type for this layer
    type Base;

    /// The bonuses type for this layer
    type Bonuses;

    /// The final/output type for this layer
    type Final;

    /// Compute the final values from base and bonuses
    ///
    /// This is the core computation that applies the bonus stack to base values.
    fn compute(base: &Self::Base, bonuses: &Self::Bonuses) -> Self::Final;

    /// Create an empty bonus holder
    fn empty_bonuses() -> Self::Bonuses;

    /// Compute with no bonuses (convenience method)
    fn from_base(base: &Self::Base) -> Self::Final {
        Self::compute(base, &Self::empty_bonuses())
    }
}

/// Bounds configuration for a specific stat calculation.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct StatBounds {
    pub min: i32,
    pub max: i32,
}

impl StatBounds {
    /// Core stats bounds: [1, 99]
    pub const CORE: Self = Self { min: 1, max: 99 };

    /// Derived stats bounds: [0, 9999]
    pub const DERIVED: Self = Self { min: 0, max: 9999 };

    /// Speed stats bounds: [10, 1000]
    ///
    /// Clamps to 10x slower (10 speed = 1000% cost) to 10x faster (1000 speed = 10% cost).
    pub const SPEED: Self = Self { min: 10, max: 1000 };

    /// Modifier bounds: [-20, 50]
    ///
    /// Keeps d20 rolls meaningful (DC 1-70 effective range).
    pub const MODIFIER: Self = Self { min: -20, max: 50 };

    /// Resource maximum bounds: [1, 99999]
    ///
    /// Prevents resources from being reduced to 0 while allowing high-level scaling.
    pub const RESOURCE_MAX: Self = Self { min: 1, max: 99999 };
}

/// Aggregated bonuses for all stat layers.
///
/// This struct caches bonuses computed from equipment, buffs, and environmental
/// effects. By storing bonuses in the actor's state, ZK proofs only need to
/// verify bonus correctness during state transitions (equipment changes) rather
/// than on every action execution.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct ActorBonuses {
    pub core: super::core::CoreStatBonuses,
    pub derived: super::derived::DerivedBonuses,
    pub modifiers: super::modifiers::ModifierBonuses,
    pub speed: super::speed::SpeedBonuses,
    pub resources: super::resources::ResourceBonuses,
}

impl ActorBonuses {
    /// Create empty bonuses (no effects)
    pub fn new() -> Self {
        Self::default()
    }
}

/// Compute actor bonuses from game state (pure function).
pub fn compute_actor_bonuses(// Future: Add inventory, effects, position parameters
    // For now: Return empty bonuses (equipment system not yet implemented)
) -> ActorBonuses {
    // TODO: Implement when equipment system is ready
    // This is a placeholder to enable the refactoring
    ActorBonuses::new()
}
