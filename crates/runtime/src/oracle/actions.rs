//! Action profile oracle implementation.
//!
//! Provides action profiles loaded from RON data files.

use game_content::ActionProfileRegistry;
use game_core::{ActionKind, ActionOracle, ActionProfile};
use std::sync::Arc;

/// Action profile oracle implementation.
///
/// This implementation loads action profiles from RON data files
/// and provides them to the game engine.
#[derive(Debug, Clone)]
pub struct ActionOracleImpl {
    action_profiles: Arc<ActionProfileRegistry>,
}

impl Default for ActionOracleImpl {
    fn default() -> Self {
        Self::new()
    }
}

impl ActionOracleImpl {
    /// Create a new action oracle by loading profiles from data files.
    pub fn new() -> Self {
        let action_profiles =
            ActionProfileRegistry::load().expect("Failed to load action profiles from data files");

        Self {
            action_profiles: Arc::new(action_profiles),
        }
    }

    /// Create with default profiles for testing.
    pub fn test_actions() -> Self {
        Self::new()
    }
}

impl ActionOracle for ActionOracleImpl {
    fn action_profile(&self, kind: ActionKind) -> ActionProfile {
        self.action_profiles.get(kind).clone()
    }
}
