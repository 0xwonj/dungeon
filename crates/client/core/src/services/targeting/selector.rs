//! Target selector managing strategy selection and execution.
//!
//! Acts as a facade for the targeting subsystem, similar to runtime's
//! ProviderRegistry for ActionProviders.

use crate::services::targeting::{
    TargetingStrategy,
    strategies::{
        FastestStrategy, LowestHealthStrategy, NearestStrategy, NextToActStrategy,
        ThreatBasedStrategy,
    },
};
use crate::view_model::ViewModel;
use game_core::Position;

/// Target selector managing strategy selection and execution.
///
/// **Design Pattern:** Facade + Strategy
/// - Manages the currently active targeting strategy
/// - Provides convenient constructors for built-in strategies
/// - Supports runtime strategy switching
///
/// **Relationship to Runtime:**
/// Similar to `runtime::ProviderRegistry` which manages `ActionProvider` instances,
/// but for UI target selection instead of entity AI.
///
/// # Examples
///
/// ```ignore
/// // Default (ThreatBased)
/// let selector = TargetSelector::with_default();
///
/// // Named strategy
/// let selector = TargetSelector::with_strategy_name("nearest")?;
///
/// // Custom strategy
/// let strategy = Box::new(MyCustomStrategy);
/// let selector = TargetSelector::new(strategy);
///
/// // Change at runtime
/// selector.set_strategy(Box::new(FastestStrategy::default()));
/// ```
pub struct TargetSelector {
    strategy: Box<dyn TargetingStrategy>,
}

impl TargetSelector {
    /// Create a new selector with the given strategy.
    pub fn new(strategy: Box<dyn TargetingStrategy>) -> Self {
        Self { strategy }
    }

    /// Create with default strategy (ThreatBased).
    pub fn with_default() -> Self {
        Self::new(Box::new(ThreatBasedStrategy::default()))
    }

    /// Create with a specific named strategy.
    ///
    /// # Supported Names
    ///
    /// - `"threat"` or `"threat-based"` - Default threat-based targeting
    /// - `"nearest"` - Simple nearest-NPC targeting
    /// - `"lowest-health"` - Target lowest HP percentage
    /// - `"fastest"` - Target highest speed stat
    /// - `"next-to-act"` - Target enemy that will act soonest
    ///
    /// # Errors
    ///
    /// Returns `Err` if the strategy name is not recognized.
    pub fn with_strategy_name(name: &str) -> Result<Self, &'static str> {
        let strategy: Box<dyn TargetingStrategy> = match name {
            "threat" | "threat-based" => Box::new(ThreatBasedStrategy::default()),
            "nearest" => Box::new(NearestStrategy),
            "lowest-health" => Box::new(LowestHealthStrategy::default()),
            "fastest" => Box::new(FastestStrategy::default()),
            "next-to-act" => Box::new(NextToActStrategy::default()),
            _ => return Err("Unknown targeting strategy"),
        };
        Ok(Self::new(strategy))
    }

    /// Replace the current strategy.
    ///
    /// This allows runtime switching of targeting behavior, useful for:
    /// - Keybinds to cycle through strategies ('T' key)
    /// - UI settings to change targeting mode
    /// - Situational switching (exploration vs combat)
    pub fn set_strategy(&mut self, strategy: Box<dyn TargetingStrategy>) {
        self.strategy = strategy;
    }

    /// Select target using current strategy.
    ///
    /// Delegates to the active strategy's `select_target` implementation.
    pub fn select_target(&self, view_model: &ViewModel) -> Option<Position> {
        self.strategy.select_target(view_model)
    }

    /// Get current strategy name.
    ///
    /// Useful for UI display, debug logging, and message feedback.
    pub fn current_strategy_name(&self) -> &'static str {
        self.strategy.name()
    }

    /// Get current strategy description.
    ///
    /// Useful for tooltips and help text.
    pub fn current_strategy_description(&self) -> &'static str {
        self.strategy.description()
    }
}

impl Default for TargetSelector {
    fn default() -> Self {
        Self::with_default()
    }
}
