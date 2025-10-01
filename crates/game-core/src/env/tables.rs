use crate::action::AttackStyle;

pub trait TablesOracle {
    fn movement_rules(&self) -> MovementRules;
    fn attack_profile(&self, style: AttackStyle) -> Option<AttackProfile>;
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
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
