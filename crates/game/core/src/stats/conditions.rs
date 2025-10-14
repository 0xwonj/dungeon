//! Conditions - Status effects that modify stats.
//!
//! Conditions represent reverse dependencies without violating layer hierarchy.
//! They are applied as final multipliers after all other calculations.
//!
//! Examples: Haste, Slow, Poisoned, Stunned, Low HP penalty, etc.

use super::speed::SpeedStats;

/// Trait for conditions that modify game state.
///
/// Conditions can affect:
/// - Speed values (Haste, Slow, Stunned)
/// - Action costs (Fatigue, Overload)
/// - Roll outcomes (Lucidity scaling, Blessed, Cursed)
///
/// # Design Pattern: Strategy
/// Each condition implements specific modification logic while
/// following the same interface for consistency.
///
/// # Implementation Rules
/// 1. Conditions MUST NOT read lower-layer computed values
/// 2. Conditions MAY read stored state (current HP, equipment load, etc.)
/// 3. Conditions are applied as final multipliers after all other bonuses
/// 4. Multiple conditions can stack (applied sequentially)
pub trait Condition: Send + Sync {
    /// Get the condition's name (for debugging/UI)
    fn name(&self) -> &str;

    /// Apply condition to speed stats
    ///
    /// This modifies speed values in-place. Return true if any modification was made.
    fn apply_to_speed(&self, _speed: &mut SpeedStats) -> bool {
        false // Default: no effect
    }

    /// Apply condition to action cost
    ///
    /// Returns a cost multiplier (100 = no change, 150 = +50% cost, 50 = -50% cost)
    fn apply_to_cost(&self) -> i32 {
        100 // Default: no change
    }

    /// Apply condition to roll outcome
    ///
    /// This can scale roll results for global effects like Lucidity.
    /// Returns a multiplier percentage (100 = no change)
    fn apply_to_roll(&self) -> i32 {
        100 // Default: no change
    }
}

/// Common condition implementations
pub mod common {
    use super::*;

    /// Haste: Increases speed
    #[derive(Clone, Debug)]
    pub struct Haste {
        pub speed_bonus_percent: i32,
    }

    impl Haste {
        pub fn new(speed_bonus_percent: i32) -> Self {
            Self {
                speed_bonus_percent,
            }
        }
    }

    impl Condition for Haste {
        fn name(&self) -> &str {
            "Haste"
        }

        fn apply_to_speed(&self, speed: &mut SpeedStats) -> bool {
            let multiplier = 100 + self.speed_bonus_percent;
            speed.physical = (speed.physical * multiplier) / 100;
            speed.cognitive = (speed.cognitive * multiplier) / 100;
            speed.ritual = (speed.ritual * multiplier) / 100;
            true
        }
    }

    /// Slow: Decreases speed
    #[derive(Clone, Debug)]
    pub struct Slow {
        pub speed_penalty_percent: i32,
    }

    impl Slow {
        pub fn new(speed_penalty_percent: i32) -> Self {
            Self {
                speed_penalty_percent,
            }
        }
    }

    impl Condition for Slow {
        fn name(&self) -> &str {
            "Slow"
        }

        fn apply_to_speed(&self, speed: &mut SpeedStats) -> bool {
            let multiplier = 100 - self.speed_penalty_percent;
            speed.physical = (speed.physical * multiplier) / 100;
            speed.cognitive = (speed.cognitive * multiplier) / 100;
            speed.ritual = (speed.ritual * multiplier) / 100;
            true
        }
    }

    /// Stunned: Cannot act (massive speed penalty)
    #[derive(Clone, Debug)]
    pub struct Stunned;

    impl Condition for Stunned {
        fn name(&self) -> &str {
            "Stunned"
        }

        fn apply_to_speed(&self, speed: &mut SpeedStats) -> bool {
            // Reduce speed to minimum (50)
            speed.physical = 50;
            speed.cognitive = 50;
            speed.ritual = 50;
            true
        }

        fn apply_to_cost(&self) -> i32 {
            200 // Double action cost
        }
    }

    /// Overload: Carrying too much (increased costs)
    #[derive(Clone, Debug)]
    pub struct Overload {
        pub cost_increase_percent: i32,
    }

    impl Overload {
        pub fn new(cost_increase_percent: i32) -> Self {
            Self {
                cost_increase_percent,
            }
        }
    }

    impl Condition for Overload {
        fn name(&self) -> &str {
            "Overload"
        }

        fn apply_to_cost(&self) -> i32 {
            100 + self.cost_increase_percent
        }
    }

    /// Low Lucidity: Reduced effectiveness
    #[derive(Clone, Debug)]
    pub struct LowLucidity {
        pub effectiveness_percent: i32, // 0-100
    }

    impl LowLucidity {
        pub fn new(effectiveness_percent: i32) -> Self {
            Self {
                effectiveness_percent: effectiveness_percent.clamp(0, 100),
            }
        }
    }

    impl Condition for LowLucidity {
        fn name(&self) -> &str {
            "Low Lucidity"
        }

        fn apply_to_roll(&self) -> i32 {
            self.effectiveness_percent
        }

        fn apply_to_speed(&self, speed: &mut SpeedStats) -> bool {
            // Lucidity affects cognitive and ritual speed
            speed.cognitive = (speed.cognitive * self.effectiveness_percent) / 100;
            speed.ritual = (speed.ritual * self.effectiveness_percent) / 100;
            true
        }
    }
}

/// A collection of active conditions.
///
/// Conditions are applied in the order they are added.
#[derive(Default)]
pub struct ConditionSet {
    conditions: Vec<Box<dyn Condition>>,
}

impl ConditionSet {
    /// Create a new empty condition set
    pub fn new() -> Self {
        Self {
            conditions: Vec::new(),
        }
    }

    /// Add a condition
    pub fn add(&mut self, condition: Box<dyn Condition>) {
        self.conditions.push(condition);
    }

    /// Apply all conditions to speed stats
    pub fn apply_all_to_speed(&self, speed: &mut SpeedStats) {
        for condition in &self.conditions {
            condition.apply_to_speed(speed);
        }
    }

    /// Get combined cost multiplier from all conditions
    pub fn get_combined_cost_multiplier(&self) -> i32 {
        let mut multiplier = 100;
        for condition in &self.conditions {
            let condition_mult = condition.apply_to_cost();
            multiplier = (multiplier * condition_mult) / 100;
        }
        multiplier
    }

    /// Get combined roll multiplier from all conditions
    pub fn get_combined_roll_multiplier(&self) -> i32 {
        let mut multiplier = 100;
        for condition in &self.conditions {
            let condition_mult = condition.apply_to_roll();
            multiplier = (multiplier * condition_mult) / 100;
        }
        multiplier
    }

    /// Check if the set is empty
    pub fn is_empty(&self) -> bool {
        self.conditions.is_empty()
    }

    /// Get the number of active conditions
    pub fn len(&self) -> usize {
        self.conditions.len()
    }
}
