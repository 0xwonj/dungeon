//! Actor stat aggregation - combines core stats with current resources.

use super::core::{CoreEffective, CoreStats};
use super::resources::{ResourceCurrent, ResourceMaximums, ResourceMeters};
use super::speed::SpeedStats;

/// Complete actor statistics combining base stats and current resources.
///
/// This is the primary struct stored in ActorState.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ActorStats {
    pub core: CoreStats,
    pub resources: ResourceCurrent,
}

impl ActorStats {
    /// Create new actor stats
    pub const fn new(core: CoreStats, resources: ResourceCurrent) -> Self {
        Self { core, resources }
    }

    /// Create actor stats at full resources
    pub fn at_full(core: CoreStats) -> Self {
        let max = ResourceMaximums::compute(&CoreEffective::from_base(&core));
        Self {
            core,
            resources: ResourceCurrent::at_max(&max),
        }
    }

    /// Get resource meters (current + maximum)
    pub fn resource_meters(&self) -> ResourceMeters {
        let max = ResourceMaximums::compute(&CoreEffective::from_base(&self.core));
        self.resources.to_meters(&max)
    }

    /// Check if actor is alive (HP > 0)
    pub fn is_alive(&self) -> bool {
        self.resources.hp > 0
    }

    /// Get physical action speed (for timeline calculations)
    pub fn speed_physical(&self) -> i32 {
        let core_effective = CoreEffective::from_base(&self.core);
        SpeedStats::compute(&core_effective).physical
    }

    /// Get cognitive action speed
    pub fn speed_cognitive(&self) -> i32 {
        let core_effective = CoreEffective::from_base(&self.core);
        SpeedStats::compute(&core_effective).cognitive
    }

    /// Get ritual action speed
    pub fn speed_ritual(&self) -> i32 {
        let core_effective = CoreEffective::from_base(&self.core);
        SpeedStats::compute(&core_effective).ritual
    }
}

impl Default for ActorStats {
    fn default() -> Self {
        Self::at_full(CoreStats::default())
    }
}
