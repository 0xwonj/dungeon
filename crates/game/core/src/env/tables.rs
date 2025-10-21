use crate::action::AttackStyle;

/// Oracle providing game rules and balance tables.
///
/// This oracle defines core gameplay mechanics like movement constraints,
/// attack formulas, and status effect durations. It does NOT define entity data
/// (use NpcOracle, ItemOracle, etc. for that).
pub trait TablesOracle: Send + Sync {
    fn movement_rules(&self) -> MovementRules;
    fn attack_profile(&self, style: AttackStyle) -> Option<AttackProfile>;
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct MovementRules {
    pub max_step_distance: u8,
    pub base_action_cost: u8,
}

impl MovementRules {
    pub const fn new(max_step_distance: u8, base_action_cost: u8) -> Self {
        Self {
            max_step_distance,
            base_action_cost,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct AttackProfile {
    pub damage: u16,
    pub energy_cost: u8,
}

impl AttackProfile {
    pub const fn new(damage: u16, energy_cost: u8) -> Self {
        Self {
            damage,
            energy_cost,
        }
    }
}
