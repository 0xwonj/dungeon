//! Default balance tables oracle implementation.
//!
//! Provides default game balance values that can be overridden
//! for different game modes, difficulty settings, or on-chain governance.

use game_core::{
    ActionCosts, CombatParams, DamageParams, HitChanceParams, SpeedParams, TablesOracle,
};

/// Default balance tables oracle implementation.
///
/// This implementation provides the baseline game balance values.
///
/// # Future Extensions
///
/// - Load from configuration files
/// - Support multiple difficulty modes
/// - Integrate with on-chain governance
/// - Version tracking for balance patches
#[derive(Debug, Clone)]
pub struct TablesOracleImpl {
    action_costs: ActionCosts,
    combat: CombatParams,
    speed: SpeedParams,
}

impl Default for TablesOracleImpl {
    fn default() -> Self {
        Self::new()
    }
}

impl TablesOracleImpl {
    /// Create a new tables oracle with default balance values.
    pub fn new() -> Self {
        Self {
            action_costs: Self::default_action_costs(),
            combat: Self::default_combat(),
            speed: Self::default_speed(),
        }
    }

    /// Create with default balance for testing.
    pub fn test_tables() -> Self {
        Self::new()
    }

    // ========================================================================
    // Default Value Constructors
    // ========================================================================

    /// Default action base costs (before speed scaling)
    fn default_action_costs() -> ActionCosts {
        ActionCosts {
            attack: 100,
            move_action: 100,
            wait: 100,
            interact: 100,
            activation: 0, // System action - no cost
        }
    }

    /// Default combat parameters
    fn default_combat() -> CombatParams {
        CombatParams {
            hit_chance: HitChanceParams {
                base: 80, // 80% base hit chance
                min: 5,   // 5% minimum (always a chance to hit)
                max: 95,  // 95% maximum (always a chance to miss)
            },
            damage: DamageParams {
                ac_divisor: 2,      // AC reduces damage by ac/2
                crit_multiplier: 2, // 2x damage on crit
                minimum: 1,         // At least 1 damage per hit
            },
        }
    }

    /// Default speed system parameters
    fn default_speed() -> SpeedParams {
        SpeedParams {
            cost_multiplier: 100, // (base_cost * 100) / speed
            min: 1,               // Minimum speed (very slow)
            max: 1000,            // Maximum speed (very fast)
        }
    }
}

impl TablesOracle for TablesOracleImpl {
    fn action_costs(&self) -> ActionCosts {
        self.action_costs
    }

    fn combat(&self) -> CombatParams {
        self.combat
    }

    fn speed(&self) -> SpeedParams {
        self.speed
    }
}
