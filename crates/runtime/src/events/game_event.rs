//! Game events extracted from state deltas.
//!
//! Events represent high-level occurrences in the game (entity died, damage taken, etc.)
//! extracted from low-level state deltas. Event handlers react to these events to
//! generate system actions.

use game_core::{Action, EntityId, Position, Tick};

/// High-level game events extracted from StateDelta.
///
/// Events are computed from deltas after each action execution and are used
/// by event handlers to generate reactive system actions.
#[derive(Debug, Clone)]
pub enum GameEvent {
    /// An action was completed (always emitted for non-system actions).
    ActionCompleted {
        actor: EntityId,
        action: Action,
        cost: Tick,
    },

    /// An entity took damage.
    DamageTaken {
        entity: EntityId,
        amount: u32,
        hp_before: u32,
        hp_after: u32,
        source: Option<EntityId>,
    },

    /// An entity died (HP reached 0).
    EntityDied {
        entity: EntityId,
        position: Position,
        killer: Option<EntityId>,
    },

    /// An entity moved to a new position.
    EntityMoved {
        entity: EntityId,
        from: Position,
        to: Position,
    },

    /// An entity was removed from the active set.
    EntityRemovedFromActive { entity: EntityId },

    /// An entity's health crossed a threshold.
    HealthThresholdCrossed {
        entity: EntityId,
        threshold: HealthThreshold,
        hp_percent: u32,
    },

    /// An entity's ready_at timestamp was updated.
    ReadyAtUpdated {
        entity: EntityId,
        old_ready_at: Option<Tick>,
        new_ready_at: Option<Tick>,
    },
}

/// Health threshold levels for triggering effects.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HealthThreshold {
    /// 100% HP
    Full,
    /// 75-99% HP
    Healthy,
    /// 25-74% HP
    Wounded,
    /// 1-24% HP
    Critical,
    /// 0% HP (dead)
    Dead,
}

impl HealthThreshold {
    /// Calculate health threshold from current and max HP.
    pub fn from_hp(current: u32, max: u32) -> Self {
        if current == 0 {
            Self::Dead
        } else if max == 0 {
            Self::Full
        } else {
            let percent = (current * 100) / max;
            match percent {
                100 => Self::Full,
                75..=99 => Self::Healthy,
                25..=74 => Self::Wounded,
                _ => Self::Critical,
            }
        }
    }
}
