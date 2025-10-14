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
#[derive(Clone, Debug, PartialEq)]
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
#[derive(Clone, Debug, Default)]
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
