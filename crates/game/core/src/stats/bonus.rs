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
///
/// # Design Pattern: Value Object
/// Bonuses are immutable and composable. Multiple bonuses are combined
/// via `BonusStack` which applies them in the correct order.
#[derive(Clone, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum Bonus {
    /// Flat additive bonus (applied first)
    Flat(i32),

    /// Percentage increase (summed with other %Inc, then multiplied)
    /// Stored as integer percentage (e.g., 20 = +20%)
    Increased(i32),

    /// Multiplicative "more" modifier (applied sequentially)
    /// Stored as percentage (e.g., 50 = ×1.5, -20 = ×0.8)
    More(i32),

    /// Multiplicative "less" modifier (applied sequentially)
    /// Stored as percentage (e.g., 10 = ×0.9, -10 = ×1.1)
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
///
/// # Example
/// ```
/// # use game_core::stats::bonus::{Bonus, BonusStack};
/// let mut stack = BonusStack::new();
/// stack.add(Bonus::flat(5));           // +5
/// stack.add(Bonus::increased(20));     // +20%
/// stack.add(Bonus::increased(15));     // +15% (summed)
/// stack.add(Bonus::more(50));          // ×1.5
/// stack.add(Bonus::less(10));          // ×0.9
///
/// let result = stack.apply(10, 5, 100);
/// // = clamp((10 + 5) × 1.35 × 1.5 × 0.9, 5, 100)
/// // = clamp(27.3375, 5, 100)
/// // = 27
/// assert_eq!(result, 27);
/// ```
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

/// Builder for constructing bonus stacks fluently
///
/// # Example
/// ```
/// # use game_core::stats::bonus::BonusStack;
/// let result = BonusStack::new()
///     .flat(5)
///     .increased(20)
///     .more(50)
///     .apply(10, 0, 100);
/// ```
impl BonusStack {
    /// Add a flat bonus (builder pattern)
    pub fn flat(mut self, value: i32) -> Self {
        self.add(Bonus::flat(value));
        self
    }

    /// Add a percentage increase (builder pattern)
    pub fn increased(mut self, percent: i32) -> Self {
        self.add(Bonus::increased(percent));
        self
    }

    /// Add a "more" multiplier (builder pattern)
    pub fn more(mut self, percent: i32) -> Self {
        self.add(Bonus::more(percent));
        self
    }

    /// Add a "less" multiplier (builder pattern)
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
///
/// # Examples
///
/// ## Basic Usage
///
/// ```
/// # use game_core::stats::*;
/// // Compute core stats with no bonuses
/// let base = CoreStats::new(18, 16, 14, 10, 10, 10, 5);
/// let core = CoreEffective::from_base(&base);
/// assert_eq!(core.str, 18);
/// assert_eq!(core.level, 5);
/// ```
///
/// ## With Bonuses
///
/// ```
/// # use game_core::stats::*;
/// # use game_core::stats::bonus::*;
/// let base = CoreStats::new(18, 16, 14, 10, 10, 10, 5);
///
/// let mut bonuses = CoreStatBonuses::new();
/// bonuses.add_str(Bonus::flat(2));  // +2 STR from equipment
///
/// let core = CoreEffective::compute(&base, &bonuses);
/// assert_eq!(core.str, 20);  // 18 + 2
/// ```
///
/// ## Generic Functions
///
/// ```
/// # use game_core::stats::*;
/// # use game_core::stats::bonus::*;
/// // Write functions that work with any layer
/// fn compute_with_empty<L: StatLayer>(base: &L::Base) -> L::Final {
///     L::compute(base, &L::empty_bonuses())
/// }
///
/// let base = CoreStats::default();
/// let core = CoreEffective::from_base(&base);
/// let derived = compute_with_empty::<DerivedStats>(&core);
/// assert_eq!(derived.attack, 15);  // STR 10 × 1.5
/// ```
///
/// ## Layer Chaining
///
/// ```
/// # use game_core::stats::*;
/// // Chain multiple layers together
/// let base = CoreStats::new(18, 16, 14, 10, 10, 10, 5);
///
/// // Layer 1: CoreStats -> CoreEffective
/// let core = CoreEffective::from_base(&base);
///
/// // Layer 2: CoreEffective -> DerivedStats
/// let derived = DerivedStats::from_base(&core);
/// assert_eq!(derived.attack, 27);  // 18 × 1.5
///
/// // Layer 3: CoreEffective -> SpeedStats
/// let speed = SpeedStats::from_base(&core);
/// assert_eq!(speed.physical, 114);  // 100 + (14×0.8) + (18×0.2)
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

    /// Get the stat bounds for this layer (if applicable)
    ///
    /// Returns `None` for layers that don't use the standard bonus system
    /// (e.g., ResourceMaximums which uses formula-based computation).
    ///
    /// # Example
    ///
    /// ```
    /// # use game_core::stats::*;
    /// # use game_core::stats::bonus::*;
    /// let bounds = CoreEffective::bounds();
    /// assert_eq!(bounds.unwrap().min, 1);
    /// assert_eq!(bounds.unwrap().max, 99);
    /// ```
    fn bounds() -> Option<StatBounds> {
        None
    }
}

/// Bounds configuration for a specific stat calculation.
///
/// Each layer defines appropriate min/max values based on game balance.
/// This centralizes all clamping bounds in one place, making it easy to
/// understand and adjust the ranges for different stat types.
///
/// # Design Rationale
///
/// Different stat layers require different ranges:
/// - **Core stats**: [1, 99] prevents degenerate cases (0 stats) and extreme values
/// - **Derived stats**: [0, 9999] allows flexibility for combat calculations
/// - **Speed**: [50, 200] ensures actions aren't too slow (>200% cost) or too fast (<50% cost)
/// - **Modifiers**: [-20, 50] keeps d20 rolls meaningful (DC 1-70 effective range)
///
/// # Usage
/// ```
/// # use game_core::stats::bonus::{BonusStack, StatBounds};
/// let stack = BonusStack::new();
/// let bounds = StatBounds::CORE_STATS;
/// let result = stack.apply(10, bounds.min, bounds.max);
/// ```
#[derive(Clone, Copy, Debug)]
pub struct StatBounds {
    pub min: i32,
    pub max: i32,
}

impl StatBounds {
    /// Core stats bounds [1, 99]
    ///
    /// Prevents stats from reaching 0 (broken game state) or exceeding 99 (balance issues).
    pub const CORE_STATS: Self = Self { min: 1, max: 99 };

    /// Derived stats bounds [0, 9999]
    ///
    /// Allows wide range for combat values while preventing overflow in calculations.
    pub const DERIVED_STATS: Self = Self { min: 0, max: 9999 };

    /// Speed stats bounds [50, 200]
    ///
    /// Clamps speed to 2x slower (50 speed = 200% action cost) to 2x faster (200 speed = 50% cost).
    pub const SPEED_STATS: Self = Self { min: 50, max: 200 };

    /// Modifier bounds [-20, 50]
    ///
    /// Extreme debuff (-20) still allows success on d20+1 vs DC 1, extreme buff (+50) caps at DC 70.
    pub const MODIFIERS: Self = Self { min: -20, max: 50 };

    /// Resource maximum bounds [1, 99999]
    ///
    /// Prevents resources from being reduced to 0 (dead/unusable) while allowing high-level scaling.
    /// Upper bound prevents u32 overflow in resource calculations.
    pub const RESOURCE_MAXIMUMS: Self = Self { min: 1, max: 99999 };

    /// No bounds (unclamped)
    ///
    /// Use sparingly - only when mathematical properties require unbounded values.
    pub const UNCLAMPED: Self = Self {
        min: i32::MIN,
        max: i32::MAX,
    };
}

/// Aggregated bonuses for all stat layers.
///
/// This struct caches bonuses computed from equipment, buffs, and environmental
/// effects. By storing bonuses in the actor's state, ZK proofs only need to
/// verify bonus correctness during state transitions (equipment changes) rather
/// than on every action execution.
///
/// # ZK Efficiency Pattern
///
/// ```text
/// Expensive (rare):  Equip Item  → Recompute Bonuses → Prove Correctness
/// Cheap (frequent):  Attack      → Use Cached Bonuses → Skip Recomputation
/// ```
///
/// This amortizes the cost of `compute_actor_bonuses()` across many actions.
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
///
/// This is the ONLY function that computes bonuses. It must be:
/// - **Pure**: Same inputs → same outputs (no I/O, no randomness)
/// - **Deterministic**: Reproducible in ZK circuits
/// - **Complete**: All bonus sources accounted for
///
/// # Bonus Sources
///
/// 1. **Equipment**: Items in inventory (weapons, armor, accessories)
/// 2. **Buffs/Debuffs**: Active status effects
/// 3. **Environment**: Terrain, weather, area effects
///
/// # ZK Context
///
/// This function must be implementable in both:
/// - **Runtime**: Off-chain game server (this code)
/// - **Circuit**: ZK proof verifier (on-chain)
///
/// Any change here requires updating the ZK circuit implementation.
///
/// # Example
///
/// ```ignore
/// let bonuses = compute_actor_bonuses(/* ... */);
/// // bonuses.derived.attack now includes weapon damage
/// // bonuses.resources.hp_max now includes armor bonuses
/// ```
pub fn compute_actor_bonuses(// Future: Add inventory, effects, position parameters
    // For now: Return empty bonuses (equipment system not yet implemented)
) -> ActorBonuses {
    // TODO: Implement when equipment system is ready
    // This is a placeholder to enable the refactoring
    ActorBonuses::new()
}
