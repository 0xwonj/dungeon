// Action profile loader
//!
//! Loads action profiles from RON data files.

use game_core::ActionKind;
use game_core::ActionProfile;
use std::collections::HashMap;

/// Registry for action profiles.
///
/// Loads ActionProfile data from RON files and provides lookup by ActionKind.
#[derive(Debug, Clone)]
pub struct ActionProfileRegistry {
    profiles: HashMap<ActionKind, ActionProfile>,
}

impl ActionProfileRegistry {
    /// Loads all action profiles from embedded RON data files.
    pub fn load() -> Result<Self, String> {
        let mut profiles = HashMap::new();

        // Load basic actions (Wait, Interact, UseItem, etc.)
        let basic_ron = include_str!("../../data/actions/basic.ron");
        let basic_profiles: Vec<ActionProfile> =
            ron::from_str(basic_ron).map_err(|e| format!("Failed to parse basic.ron: {}", e))?;
        for profile in basic_profiles {
            profiles.insert(profile.kind, profile);
        }

        // Load movement actions (Move, Dash, etc.)
        let movement_ron = include_str!("../../data/actions/movement.ron");
        let movement_profiles: Vec<ActionProfile> = ron::from_str(movement_ron)
            .map_err(|e| format!("Failed to parse movement.ron: {}", e))?;
        for profile in movement_profiles {
            profiles.insert(profile.kind, profile);
        }

        // Load attack actions (MeleeAttack, PowerAttack, RangedAttack, etc.)
        let attack_ron = include_str!("../../data/actions/attack.ron");
        let attack_profiles: Vec<ActionProfile> =
            ron::from_str(attack_ron).map_err(|e| format!("Failed to parse attack.ron: {}", e))?;
        for profile in attack_profiles {
            profiles.insert(profile.kind, profile);
        }

        // Load item actions (PickupItem, UseItem, etc.)
        let items_ron = include_str!("../../data/actions/items.ron");
        let items_profiles: Vec<ActionProfile> =
            ron::from_str(items_ron).map_err(|e| format!("Failed to parse items.ron: {}", e))?;
        for profile in items_profiles {
            profiles.insert(profile.kind, profile);
        }

        Ok(Self { profiles })
    }

    /// Gets an action profile by kind.
    ///
    /// # Panics
    ///
    /// Panics if the action profile is not registered.
    pub fn get(&self, kind: ActionKind) -> &ActionProfile {
        self.profiles
            .get(&kind)
            .unwrap_or_else(|| panic!("ActionProfile not found for {:?}", kind))
    }

    /// Returns an iterator over all registered action kinds.
    pub fn kinds(&self) -> impl Iterator<Item = ActionKind> + '_ {
        self.profiles.keys().copied()
    }

    /// Returns the number of registered action profiles.
    pub fn len(&self) -> usize {
        self.profiles.len()
    }

    /// Returns true if no action profiles are registered.
    pub fn is_empty(&self) -> bool {
        self.profiles.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_load_action_profiles() {
        let registry = ActionProfileRegistry::load().expect("Failed to load action profiles");

        assert!(
            registry.len() >= 3,
            "Should have at least 3 action profiles"
        );

        // Verify Move
        let move_profile = registry.get(ActionKind::Move);
        assert_eq!(move_profile.kind, ActionKind::Move);
        assert_eq!(move_profile.base_cost, 100);

        // Verify MeleeAttack
        let melee_profile = registry.get(ActionKind::MeleeAttack);
        assert_eq!(melee_profile.kind, ActionKind::MeleeAttack);
        assert!(melee_profile.tags.contains(&game_core::ActionTag::Attack));

        // Verify Wait
        let wait_profile = registry.get(ActionKind::Wait);
        assert_eq!(wait_profile.kind, ActionKind::Wait);
    }
}
