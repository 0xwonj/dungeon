//! ActionSnapshot - Complete stat snapshot at action initiation.
//!
//! The snapshot captures all derived values at the moment an action begins,
//! ensuring deterministic resolution regardless of mid-action state changes.
//!
//! This is critical for:
//! - Deterministic gameplay
//! - ZK proof generation
//! - Replay consistency

use super::core::{CoreEffective, CoreStatBonuses, CoreStats};
use super::derived::{DerivedBonuses, DerivedStats};
use super::modifiers::{FinalModifiers, ModifierBonuses};
use super::resources::{ResourceCurrent, ResourceMaximums, ResourceMeters};
use super::speed::{SpeedConditions, SpeedStats};

/// Complete snapshot of all stats at action initiation.
///
/// This struct captures:
/// 1. CoreEffective - Base stats after bonuses
/// 2. DerivedStats - Combat stats (attack, evasion, etc.)
/// 3. SpeedStats - Action speed values
/// 4. FinalModifiers - Roll modifiers
/// 5. ResourceMaximums - Maximum HP/MP/Lucidity
/// 6. ResourceCurrent - Current HP/MP/Lucidity
///
/// All values are computed and locked at snapshot creation.
/// The snapshot is immutable - create a new one if state changes.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ActionSnapshot {
    /// Core stats after applying bonuses (Layer 1)
    pub core: CoreEffective,

    /// Derived combat stats (Layer 2)
    pub derived: DerivedStats,

    /// Speed values (Layer 3)
    pub speed: SpeedStats,

    /// Roll modifiers (Layer 4)
    pub modifiers: FinalModifiers,

    /// Maximum resource values (Layer 5)
    pub resource_max: ResourceMaximums,

    /// Current resource values (stored state)
    pub resource_current: ResourceCurrent,
}

impl ActionSnapshot {
    /// Create a complete snapshot from all inputs.
    ///
    /// This is the primary constructor that computes all layers.
    ///
    /// # Arguments
    /// * `base_stats` - Base core stats (stored state)
    /// * `core_bonuses` - Bonuses to apply to core stats
    /// * `derived_bonuses` - Bonuses to apply to derived stats
    /// * `modifier_bonuses` - Bonuses to apply to modifiers
    /// * `speed_conditions` - Conditions affecting speed
    /// * `resource_current` - Current HP/MP/Lucidity (stored state)
    pub fn create(
        base_stats: &CoreStats,
        core_bonuses: &CoreStatBonuses,
        derived_bonuses: &DerivedBonuses,
        modifier_bonuses: &ModifierBonuses,
        speed_conditions: &SpeedConditions,
        resource_current: &ResourceCurrent,
    ) -> Self {
        // Layer 1: Compute CoreEffective
        let core = CoreEffective::compute(base_stats, core_bonuses);

        // Layer 2: Compute Derived Stats
        let derived = DerivedStats::compute(&core, derived_bonuses);

        // Layer 3: Compute Speed
        let speed_base = SpeedStats::compute(&core);
        let speed = speed_base.apply_conditions(speed_conditions);

        // Layer 4: Compute Modifiers
        let modifiers = FinalModifiers::compute(&core, modifier_bonuses);

        // Layer 5: Compute Resource Maximums
        let resource_max = ResourceMaximums::compute(&core);

        Self {
            core,
            derived,
            speed,
            modifiers,
            resource_max,
            resource_current: resource_current.clone(),
        }
    }

    /// Create a snapshot with no bonuses (base stats only)
    pub fn from_base(base_stats: &CoreStats, resource_current: &ResourceCurrent) -> Self {
        Self::create(
            base_stats,
            &CoreStatBonuses::new(),
            &DerivedBonuses::new(),
            &ModifierBonuses::new(),
            &SpeedConditions::new(),
            resource_current,
        )
    }

    /// Get complete resource meters (current + maximum)
    pub fn resource_meters(&self) -> ResourceMeters {
        ResourceMeters::new(&self.resource_current, &self.resource_max)
    }

    /// Check if actor is alive (HP > 0)
    pub fn is_alive(&self) -> bool {
        self.resource_current.hp > 0
    }

    /// Check if actor has enough MP for an action
    pub fn has_mp(&self, cost: u32) -> bool {
        self.resource_current.mp >= cost
    }

    /// Get Lucidity percentage (for scaling)
    pub fn lucidity_percent(&self) -> u32 {
        if self.resource_max.lucidity_max == 0 {
            return 100;
        }
        (self.resource_current.lucidity * 100) / self.resource_max.lucidity_max
    }
}

/// Builder for constructing snapshots with a fluent API.
///
/// This provides an ergonomic way to build snapshots step-by-step.
pub struct SnapshotBuilder {
    base_stats: CoreStats,
    core_bonuses: CoreStatBonuses,
    derived_bonuses: DerivedBonuses,
    modifier_bonuses: ModifierBonuses,
    speed_conditions: SpeedConditions,
    resource_current: ResourceCurrent,
}

impl SnapshotBuilder {
    /// Start building a snapshot from base stats
    pub fn from_base(base_stats: CoreStats) -> Self {
        let resource_max = ResourceMaximums::compute(&CoreEffective::from_base(&base_stats));
        let resource_current = ResourceCurrent::at_max(&resource_max);

        Self {
            base_stats,
            core_bonuses: CoreStatBonuses::new(),
            derived_bonuses: DerivedBonuses::new(),
            modifier_bonuses: ModifierBonuses::new(),
            speed_conditions: SpeedConditions::new(),
            resource_current,
        }
    }

    /// Set core stat bonuses
    pub fn with_core_bonuses(mut self, bonuses: CoreStatBonuses) -> Self {
        self.core_bonuses = bonuses;
        self
    }

    /// Set derived stat bonuses
    pub fn with_derived_bonuses(mut self, bonuses: DerivedBonuses) -> Self {
        self.derived_bonuses = bonuses;
        self
    }

    /// Set modifier bonuses
    pub fn with_modifier_bonuses(mut self, bonuses: ModifierBonuses) -> Self {
        self.modifier_bonuses = bonuses;
        self
    }

    /// Set speed conditions
    pub fn with_speed_conditions(mut self, conditions: SpeedConditions) -> Self {
        self.speed_conditions = conditions;
        self
    }

    /// Set current resources
    pub fn with_resources(mut self, current: ResourceCurrent) -> Self {
        self.resource_current = current;
        self
    }

    /// Build the snapshot
    pub fn build(self) -> ActionSnapshot {
        ActionSnapshot::create(
            &self.base_stats,
            &self.core_bonuses,
            &self.derived_bonuses,
            &self.modifier_bonuses,
            &self.speed_conditions,
            &self.resource_current,
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::stats::bonus::Bonus;

    #[test]
    fn integrated_warrior_snapshot() {
        let base = CoreStats::new(18, 16, 14, 10, 10, 10, 5);

        let mut derived_bonuses = DerivedBonuses::new();
        derived_bonuses.attack.add(Bonus::flat(20)); // Weapon damage
        derived_bonuses.ac.add(Bonus::flat(8)); // Armor

        let resource_max = ResourceMaximums::compute(&CoreEffective::from_base(&base));
        let resources = ResourceCurrent::at_max(&resource_max);

        let snapshot = ActionSnapshot::create(
            &base,
            &CoreStatBonuses::new(),
            &derived_bonuses,
            &ModifierBonuses::new(),
            &SpeedConditions::new(),
            &resources,
        );

        // Attack: 18 × 1.5 + 20 = 27 + 20 = 47
        assert_eq!(snapshot.derived.attack, 47);
        // AC: 10 + (14-10)/2 + 8 = 10 + 2 + 8 = 20
        assert_eq!(snapshot.derived.ac, 20);
        // HP: (16 × 10) + (5 × 16 / 2) = 160 + 40 = 200
        assert_eq!(snapshot.resource_max.hp_max, 200);
    }

    #[test]
    fn integrated_mage_snapshot() {
        let base = CoreStats::new(10, 10, 10, 18, 16, 14, 5);

        let mut speed_conditions = SpeedConditions::new();
        speed_conditions.cognitive.add(Bonus::more(25)); // Mental acceleration

        let resource_max = ResourceMaximums::compute(&CoreEffective::from_base(&base));
        let resources = ResourceCurrent::at_max(&resource_max);

        let snapshot = ActionSnapshot::create(
            &base,
            &CoreStatBonuses::new(),
            &DerivedBonuses::new(),
            &ModifierBonuses::new(),
            &speed_conditions,
            &resources,
        );

        // Cognitive speed: 116 × 1.25 = 145
        assert_eq!(snapshot.speed.cognitive, 145);
        // MP: (16 + 18) × 5 + (14 × 2) + (5 × √16) = 170 + 28 + 20 = 218
        assert_eq!(snapshot.resource_max.mp_max, 218);
    }
}
