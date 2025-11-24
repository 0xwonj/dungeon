//! Action profile oracle.
//!
//! Provides access to action definitions and behavior specifications.

use crate::action::{ActionKind, ActionProfile};

/// Oracle providing action profiles.
///
/// Action profiles define the complete behavior, costs, targeting, and effects
/// for each action type. Profiles are loaded from RON data files.
pub trait ActionOracle: Send + Sync {
    /// Returns the action profile for a given action kind.
    ///
    /// Action profiles define behavior, costs, targeting, and effects for each action.
    fn action_profile(&self, kind: ActionKind) -> ActionProfile;
}
