use game_core::{AttackProfile, AttackStyle, MovementRules, TablesOracle};

/// TablesOracle implementation with static game rules
pub struct TablesOracleImpl {
    movement_rules: MovementRules,
}

impl TablesOracleImpl {
    pub fn new(movement_rules: MovementRules) -> Self {
        Self { movement_rules }
    }

    /// Create with default test rules
    pub fn test_tables() -> Self {
        Self::new(MovementRules::new(1, 1))
    }
}

impl Default for TablesOracleImpl {
    fn default() -> Self {
        Self::test_tables()
    }
}

impl TablesOracle for TablesOracleImpl {
    fn movement_rules(&self) -> MovementRules {
        self.movement_rules
    }

    fn attack_profile(&self, _style: AttackStyle) -> Option<AttackProfile> {
        // Basic melee attack
        Some(AttackProfile::new(5, 0))
    }
}
