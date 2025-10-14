//! StatsSnapshot - Complete stat snapshot at a point in time.
//!
//! The snapshot captures all derived values at a specific moment (typically action initiation),
//! ensuring deterministic resolution regardless of subsequent state changes.
//!
//! This is critical for:
//! - Deterministic gameplay
//! - ZK proof generation
//! - Replay consistency

use super::bonus::{ActorBonuses, StatLayer};
use super::core::{CoreEffective, CoreStats};
use super::derived::DerivedStats;
use super::modifiers::StatModifiers;
use super::resources::{ResourceBonuses, ResourceCurrent, ResourceMaximums};
use super::speed::SpeedStats;

/// Complete snapshot of all stats at a point in time.
///
/// This struct captures:
/// 1. CoreEffective - Base stats after bonuses
/// 2. DerivedStats - Combat stats (attack, evasion, etc.)
/// 3. SpeedStats - Action speed values
/// 4. StatModifiers - Roll modifiers
/// 5. ResourceMaximums - Maximum HP/MP/Lucidity
/// 6. ResourceCurrent - Current HP/MP/Lucidity
///
/// All values are computed and locked at snapshot creation.
/// The snapshot is immutable - create a new one if state changes.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct StatsSnapshot {
    /// Core stats after applying bonuses (Layer 1)
    pub core: CoreEffective,

    /// Derived combat stats (Layer 2)
    pub derived: DerivedStats,

    /// Speed values (Layer 3)
    pub speed: SpeedStats,

    /// Roll modifiers (Layer 4)
    pub modifiers: StatModifiers,

    /// Maximum resource values (Layer 5)
    pub resource_max: ResourceMaximums,

    /// Current resource values (stored state)
    pub resource_current: ResourceCurrent,
}

impl StatsSnapshot {
    /// Create a complete snapshot from all inputs.
    ///
    /// This is the primary constructor that computes all layers.
    ///
    /// # Arguments
    /// * `base_stats` - Base core stats (stored state)
    /// * `bonuses` - Aggregated bonuses for all stat layers
    /// * `resource_current` - Current HP/MP/Lucidity (stored state)
    pub fn create(
        base_stats: &CoreStats,
        bonuses: &ActorBonuses,
        resource_current: &ResourceCurrent,
    ) -> Self {
        // Layer 1: Compute CoreEffective
        let core = <CoreEffective as StatLayer>::compute(base_stats, &bonuses.core);

        // Layer 2: Compute Derived Stats
        let derived = <DerivedStats as StatLayer>::compute(&core, &bonuses.derived);

        // Layer 3: Compute Speed
        let speed = <SpeedStats as StatLayer>::compute(&core, &bonuses.speed);

        // Layer 4: Compute Modifiers
        let modifiers = <StatModifiers as StatLayer>::compute(&core, &bonuses.modifiers);

        // Layer 5: Compute Resource Maximums
        let resource_max = <ResourceMaximums as StatLayer>::compute(&core, &bonuses.resources);

        // Clamp current resources to not exceed maximums
        // This ensures invariant: current <= max, even if stats changed (e.g., unequipped +HP item)
        let clamped_current = ResourceCurrent {
            hp: resource_current.hp.min(resource_max.hp_max),
            mp: resource_current.mp.min(resource_max.mp_max),
            lucidity: resource_current.lucidity.min(resource_max.lucidity_max),
        };

        Self {
            core,
            derived,
            speed,
            modifiers,
            resource_max,
            resource_current: clamped_current,
        }
    }

    /// Create a snapshot with no bonuses (base stats only)
    pub fn from_base(base_stats: &CoreStats, resource_current: &ResourceCurrent) -> Self {
        Self::create(base_stats, &ActorBonuses::new(), resource_current)
    }

    /// Get HP (current, maximum)
    pub fn hp(&self) -> (u32, u32) {
        (self.resource_current.hp, self.resource_max.hp_max)
    }

    /// Get MP (current, maximum)
    pub fn mp(&self) -> (u32, u32) {
        (self.resource_current.mp, self.resource_max.mp_max)
    }

    /// Get Lucidity (current, maximum)
    pub fn lucidity(&self) -> (u32, u32) {
        (
            self.resource_current.lucidity,
            self.resource_max.lucidity_max,
        )
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

/// Builder for constructing stats snapshots with a fluent API.
///
/// This provides an ergonomic way to build snapshots step-by-step.
pub struct StatsSnapshotBuilder {
    base_stats: CoreStats,
    resource_current: ResourceCurrent,
    actor_bonuses: ActorBonuses,
}

impl StatsSnapshotBuilder {
    /// Start building a snapshot from base stats
    pub fn from_base(base_stats: CoreStats) -> Self {
        let core = <CoreEffective as StatLayer>::from_base(&base_stats);
        let resource_max = <ResourceMaximums as StatLayer>::compute(&core, &ResourceBonuses::new());
        let resource_current = ResourceCurrent::at_max(&resource_max);
        let actor_bonuses = ActorBonuses::new();

        Self {
            base_stats,
            resource_current,
            actor_bonuses,
        }
    }

    /// Set actor bonuses (all layers at once)
    pub fn with_bonuses(mut self, bonuses: ActorBonuses) -> Self {
        self.actor_bonuses = bonuses;
        self
    }

    /// Set core stat bonuses
    pub fn with_core_bonuses(mut self, bonuses: super::core::CoreStatBonuses) -> Self {
        self.actor_bonuses.core = bonuses;
        self
    }

    /// Set derived stat bonuses
    pub fn with_derived_bonuses(mut self, bonuses: super::derived::DerivedBonuses) -> Self {
        self.actor_bonuses.derived = bonuses;
        self
    }

    /// Set modifier bonuses
    pub fn with_modifier_bonuses(mut self, bonuses: super::modifiers::ModifierBonuses) -> Self {
        self.actor_bonuses.modifiers = bonuses;
        self
    }

    /// Set speed bonuses
    pub fn with_speed_bonuses(mut self, bonuses: super::speed::SpeedBonuses) -> Self {
        self.actor_bonuses.speed = bonuses;
        self
    }

    /// Set resource bonuses
    pub fn with_resource_bonuses(mut self, bonuses: super::resources::ResourceBonuses) -> Self {
        self.actor_bonuses.resources = bonuses;
        self
    }

    /// Set current resources
    pub fn with_resources(mut self, current: ResourceCurrent) -> Self {
        self.resource_current = current;
        self
    }

    /// Build the snapshot
    pub fn build(self) -> StatsSnapshot {
        StatsSnapshot::create(
            &self.base_stats,
            &self.actor_bonuses,
            &self.resource_current,
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::stats::bonus::Bonus;
    use crate::stats::resources::ResourceBonuses;

    #[test]
    fn integrated_warrior_snapshot() {
        let base = CoreStats::new(18, 16, 14, 10, 10, 10, 5);

        let mut bonuses = ActorBonuses::new();
        bonuses.derived.attack.add(Bonus::flat(20)); // Weapon damage
        bonuses.derived.ac.add(Bonus::flat(8)); // Armor

        let core = CoreEffective::from_base(&base);
        let resource_max = <ResourceMaximums as StatLayer>::compute(&core, &ResourceBonuses::new());
        let resources = ResourceCurrent::at_max(&resource_max);

        let snapshot = StatsSnapshot::create(&base, &bonuses, &resources);

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

        let mut bonuses = ActorBonuses::new();
        bonuses.speed.cognitive.add(Bonus::more(25)); // Mental acceleration

        let core = CoreEffective::from_base(&base);
        let resource_max = <ResourceMaximums as StatLayer>::compute(&core, &ResourceBonuses::new());
        let resources = ResourceCurrent::at_max(&resource_max);

        let snapshot = StatsSnapshot::create(&base, &bonuses, &resources);

        // Cognitive speed: 116 × 1.25 = 145
        assert_eq!(snapshot.speed.cognitive, 145);
        // MP: (16 + 18) × 5 + (14 × 2) + (5 × √16) = 170 + 28 + 20 = 218
        assert_eq!(snapshot.resource_max.mp_max, 218);
    }

    #[test]
    fn resource_clamping_exceeds_maximum() {
        let base = CoreStats::new(10, 10, 10, 10, 10, 10, 1);

        // Calculate actual maximum: HP = 10 × 10 + 1 × 10 / 2 = 100 + 5 = 105
        let core = CoreEffective::from_base(&base);
        let max = <ResourceMaximums as StatLayer>::compute(&core, &ResourceBonuses::new());
        assert_eq!(max.hp_max, 105);

        // Try to create snapshot with current HP > max HP
        let invalid_resources = ResourceCurrent::new(999, 999, 999);

        let snapshot = StatsSnapshot::from_base(&base, &invalid_resources);

        // Current should be clamped to maximum
        assert_eq!(snapshot.resource_current.hp, 105);
        assert_eq!(snapshot.resource_max.hp_max, 105);
        assert!(snapshot.resource_current.hp <= snapshot.resource_max.hp_max);
    }

    #[test]
    fn resource_clamping_after_unequipping_hp_item() {
        let base = CoreStats::new(10, 10, 10, 10, 10, 10, 1);

        // Scenario: Equipped +100 HP item
        let mut bonuses_with_item = ActorBonuses::new();
        bonuses_with_item.resources.hp_max.add(Bonus::flat(100));

        let core = CoreEffective::from_base(&base);
        let max_with_item =
            <ResourceMaximums as StatLayer>::compute(&core, &bonuses_with_item.resources);
        // Base 105 + 100 bonus = 205
        assert_eq!(max_with_item.hp_max, 205);

        // Character at full HP with item equipped
        let current_with_item = ResourceCurrent::new(205, 0, 0);

        // Now unequip the item (no bonuses)
        let snapshot_after_unequip = StatsSnapshot::from_base(&base, &current_with_item);

        // Current HP should be clamped to new maximum (105, not 205)
        assert_eq!(snapshot_after_unequip.resource_max.hp_max, 105);
        assert_eq!(snapshot_after_unequip.resource_current.hp, 105);
        assert!(
            snapshot_after_unequip.resource_current.hp
                <= snapshot_after_unequip.resource_max.hp_max
        );
    }
}
