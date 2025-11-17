//! Target selection service for auto-targeting and tactical UI.
//!
//! Provides pluggable targeting strategies for selecting which entity to
//! highlight in Normal mode.
//!
//! The `TargetSelector` facade manages strategy selection and execution,
//! with built-in strategies: ThreatBased, Nearest, LowestHealth, Fastest.

pub mod selector;
pub mod strategies;
pub mod strategy;
pub mod utils;

// Re-export core types
pub use selector::TargetSelector;
pub use strategy::TargetingStrategy;

// Re-export strategies for convenience
pub use strategies::{
    FastestStrategy, LowestHealthStrategy, NearestStrategy, NextToActStrategy, ThreatBasedStrategy,
};

// Re-export utilities
pub use utils::{health_percentage, manhattan_distance};

use crate::view_model::{ViewModel, entities::ActorView};
use game_core::{EntityId, Position};

/// Finds all NPCs at the specified position.
#[allow(dead_code)]
pub fn find_targets_at_position(
    view_model: &ViewModel,
    position: Position,
) -> Vec<(EntityId, &ActorView)> {
    view_model
        .npcs()
        .filter(|npc| npc.position == Some(position))
        .map(|npc| (npc.id, npc))
        .collect()
}
