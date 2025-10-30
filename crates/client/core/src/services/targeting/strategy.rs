//! Targeting strategy trait for auto-targeting in Normal mode.

use crate::view_model::ViewModel;
use game_core::Position;

/// Strategy for selecting which entity to highlight in Normal mode.
///
/// Implementations must be deterministic and thread-safe.
pub trait TargetingStrategy: Send + Sync {
    /// Selects the best target position from the current view state.
    ///
    /// Returns `None` if no valid targets exist.
    fn select_target(&self, view_model: &ViewModel) -> Option<Position>;

    /// Returns the strategy name for debugging and logging.
    fn name(&self) -> &'static str;

    /// Returns an optional description of the strategy's behavior.
    fn description(&self) -> &'static str {
        "No description available"
    }
}
