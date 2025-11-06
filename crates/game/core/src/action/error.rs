//! Action execution errors.
//!
//! Errors related to action execution, validation, and system actions.

use crate::error::{ErrorContext, ErrorSeverity, GameError};
use crate::state::EntityId;

// ============================================================================
// Action Execution Errors
// ============================================================================

/// Errors that can occur during character action execution.
#[derive(Clone, Debug, PartialEq, Eq, thiserror::Error)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum ActionError {
    /// Actor not found in game state.
    #[error("Actor not found")]
    ActorNotFound,

    /// Actor is dead (HP = 0).
    #[error("Actor is dead")]
    ActorDead,

    /// Not actor's turn.
    #[error("Not actor's turn")]
    NotActorsTurn,

    /// Actor is not ready to act yet.
    #[error("Actor is not ready to act")]
    ActorNotReady,

    /// Target not found in game state.
    #[error("Target not found")]
    TargetNotFound,

    /// Action profile not found in oracle.
    #[error("Action profile not found")]
    ProfileNotFound,

    /// Invalid target for this action.
    #[error("Invalid target")]
    InvalidTarget,

    /// Out of range.
    #[error("Out of range")]
    OutOfRange,

    /// Position is out of map bounds.
    #[error("Position out of bounds")]
    OutOfBounds,

    /// Position is invalid (e.g., wall, impassable terrain).
    #[error("Invalid position")]
    InvalidPosition,

    /// Position is blocked by terrain.
    #[error("Position blocked by terrain")]
    Blocked,

    /// Position is occupied by another entity.
    #[error("Position occupied by entity")]
    Occupied,

    /// Map oracle not available.
    #[error("Map not available")]
    MapNotAvailable,

    /// Insufficient resources (lucidity, mana).
    #[error("Insufficient resources")]
    InsufficientResources,

    /// Action is on cooldown.
    #[error("Action is on cooldown")]
    OnCooldown,

    /// Action is not available (not in actor's ability list or disabled).
    #[error("Action not available")]
    ActionNotAvailable,

    /// Requirements not met.
    #[error("Requirements not met: {0}")]
    RequirementsNotMet(String),

    /// Effect application failed.
    #[error("Effect failed: {0}")]
    EffectFailed(String),

    /// Formula evaluation failed.
    #[error("Formula evaluation failed: {0}")]
    FormulaEvaluationFailed(String),

    /// Not yet implemented.
    #[error("Not implemented: {0}")]
    NotImplemented(String),
}

impl GameError for ActionError {
    fn severity(&self) -> ErrorSeverity {
        use ActionError::*;
        match self {
            ActorNotFound | TargetNotFound | ProfileNotFound => ErrorSeverity::Validation,
            ActorDead | NotActorsTurn | ActorNotReady => ErrorSeverity::Recoverable,
            InvalidTarget | OutOfRange | OutOfBounds => ErrorSeverity::Validation,
            InvalidPosition | Blocked | Occupied => ErrorSeverity::Recoverable,
            MapNotAvailable => ErrorSeverity::Fatal,
            InsufficientResources | OnCooldown | ActionNotAvailable => ErrorSeverity::Recoverable,
            RequirementsNotMet(_) => ErrorSeverity::Validation,
            EffectFailed(_) | FormulaEvaluationFailed(_) => ErrorSeverity::Internal,
            NotImplemented(_) => ErrorSeverity::Internal,
        }
    }

    fn error_code(&self) -> &'static str {
        use ActionError::*;
        match self {
            ActorNotFound => "ACTION_ACTOR_NOT_FOUND",
            ActorDead => "ACTION_ACTOR_DEAD",
            NotActorsTurn => "ACTION_NOT_ACTORS_TURN",
            ActorNotReady => "ACTION_ACTOR_NOT_READY",
            TargetNotFound => "ACTION_TARGET_NOT_FOUND",
            ProfileNotFound => "ACTION_PROFILE_NOT_FOUND",
            InvalidTarget => "ACTION_INVALID_TARGET",
            OutOfRange => "ACTION_OUT_OF_RANGE",
            OutOfBounds => "ACTION_OUT_OF_BOUNDS",
            InvalidPosition => "ACTION_INVALID_POSITION",
            Blocked => "ACTION_BLOCKED",
            Occupied => "ACTION_OCCUPIED",
            MapNotAvailable => "ACTION_MAP_NOT_AVAILABLE",
            InsufficientResources => "ACTION_INSUFFICIENT_RESOURCES",
            OnCooldown => "ACTION_ON_COOLDOWN",
            ActionNotAvailable => "ACTION_NOT_AVAILABLE",
            RequirementsNotMet(_) => "ACTION_REQUIREMENTS_NOT_MET",
            EffectFailed(_) => "ACTION_EFFECT_FAILED",
            FormulaEvaluationFailed(_) => "ACTION_FORMULA_EVALUATION_FAILED",
            NotImplemented(_) => "ACTION_NOT_IMPLEMENTED",
        }
    }
}

// ============================================================================
// System Action Errors
// ============================================================================

/// Errors that can occur during turn preparation.
#[derive(Clone, Debug, PartialEq, Eq, thiserror::Error)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum TurnError {
    /// System actor validation failed.
    #[error("prepare turn action must be executed by SYSTEM actor")]
    NotSystemActor {
        #[cfg_attr(feature = "serde", serde(skip))]
        context: ErrorContext,
    },

    /// No active entities available for scheduling.
    #[error("no entities are currently active")]
    NoActiveEntities {
        #[cfg_attr(feature = "serde", serde(skip))]
        context: ErrorContext,
    },
}

impl TurnError {
    /// Creates a NotSystemActor error with context.
    pub fn not_system_actor(nonce: u64) -> Self {
        Self::NotSystemActor {
            context: ErrorContext::new(nonce)
                .with_message("system action executed by non-system actor"),
        }
    }

    /// Creates a NoActiveEntities error with context.
    pub fn no_active_entities(nonce: u64) -> Self {
        Self::NoActiveEntities {
            context: ErrorContext::new(nonce).with_message("turn scheduling failed"),
        }
    }
}

impl GameError for TurnError {
    fn severity(&self) -> ErrorSeverity {
        match self {
            Self::NotSystemActor { .. } => ErrorSeverity::Validation,
            Self::NoActiveEntities { .. } => ErrorSeverity::Fatal,
        }
    }

    fn context(&self) -> Option<&ErrorContext> {
        match self {
            Self::NotSystemActor { context } => Some(context),
            Self::NoActiveEntities { context } => Some(context),
        }
    }

    fn error_code(&self) -> &'static str {
        match self {
            Self::NotSystemActor { .. } => "TURN_NOT_SYSTEM_ACTOR",
            Self::NoActiveEntities { .. } => "TURN_NO_ACTIVE_ENTITIES",
        }
    }
}

/// Errors that can occur during entity activation.
#[derive(Clone, Debug, PartialEq, Eq, thiserror::Error)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum ActivationError {
    /// System actor validation failed.
    #[error("activation action must be executed by SYSTEM actor")]
    NotSystemActor {
        #[cfg_attr(feature = "serde", serde(skip))]
        context: ErrorContext,
    },

    /// Player not found in game state.
    #[error("player not found in game state")]
    PlayerNotFound {
        #[cfg_attr(feature = "serde", serde(skip))]
        context: ErrorContext,
    },
}

impl ActivationError {
    /// Creates a NotSystemActor error with context.
    pub fn not_system_actor(nonce: u64) -> Self {
        Self::NotSystemActor {
            context: ErrorContext::new(nonce)
                .with_message("system action executed by non-system actor"),
        }
    }

    /// Creates a PlayerNotFound error with context.
    pub fn player_not_found(nonce: u64) -> Self {
        Self::PlayerNotFound {
            context: ErrorContext::new(nonce).with_message("player entity not found"),
        }
    }
}

impl GameError for ActivationError {
    fn severity(&self) -> ErrorSeverity {
        match self {
            Self::NotSystemActor { .. } => ErrorSeverity::Validation,
            Self::PlayerNotFound { .. } => ErrorSeverity::Fatal,
        }
    }

    fn context(&self) -> Option<&ErrorContext> {
        match self {
            Self::NotSystemActor { context } => Some(context),
            Self::PlayerNotFound { context } => Some(context),
        }
    }

    fn error_code(&self) -> &'static str {
        match self {
            Self::NotSystemActor { .. } => "ACTIVATION_NOT_SYSTEM_ACTOR",
            Self::PlayerNotFound { .. } => "ACTIVATION_PLAYER_NOT_FOUND",
        }
    }
}

/// Errors that can occur when deactivating an entity.
#[derive(Clone, Debug, PartialEq, Eq, thiserror::Error)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum DeactivateError {
    /// System actor validation failed.
    #[error("deactivate action must be executed by SYSTEM actor")]
    NotSystemActor {
        #[cfg_attr(feature = "serde", serde(skip))]
        context: ErrorContext,
    },

    /// Entity not found in game state.
    #[error("entity {entity} not found in game state")]
    EntityNotFound {
        entity: EntityId,
        #[cfg_attr(feature = "serde", serde(skip))]
        context: ErrorContext,
    },
}

impl DeactivateError {
    /// Creates a NotSystemActor error with context.
    pub fn not_system_actor(nonce: u64) -> Self {
        Self::NotSystemActor {
            context: ErrorContext::new(nonce)
                .with_message("system action executed by non-system actor"),
        }
    }

    /// Creates an EntityNotFound error with context.
    pub fn entity_not_found(entity: EntityId, nonce: u64) -> Self {
        Self::EntityNotFound {
            entity,
            context: ErrorContext::new(nonce)
                .with_actor(entity)
                .with_message("entity not found"),
        }
    }
}

impl GameError for DeactivateError {
    fn severity(&self) -> ErrorSeverity {
        match self {
            Self::NotSystemActor { .. } => ErrorSeverity::Validation,
            Self::EntityNotFound { .. } => ErrorSeverity::Validation,
        }
    }

    fn context(&self) -> Option<&ErrorContext> {
        match self {
            Self::NotSystemActor { context } => Some(context),
            Self::EntityNotFound { context, .. } => Some(context),
        }
    }

    fn error_code(&self) -> &'static str {
        match self {
            Self::NotSystemActor { .. } => "DEACTIVATE_NOT_SYSTEM_ACTOR",
            Self::EntityNotFound { .. } => "DEACTIVATE_ENTITY_NOT_FOUND",
        }
    }
}

/// Errors that can occur when removing entity from world.
#[derive(Clone, Debug, PartialEq, Eq, thiserror::Error)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum RemoveFromWorldError {
    /// System actor validation failed.
    #[error("remove from world action must be executed by SYSTEM actor")]
    NotSystemActor {
        #[cfg_attr(feature = "serde", serde(skip))]
        context: ErrorContext,
    },

    /// Entity not found in game state.
    #[error("entity {entity} not found in game state")]
    EntityNotFound {
        entity: EntityId,
        #[cfg_attr(feature = "serde", serde(skip))]
        context: ErrorContext,
    },

    /// Entity still has a position after removal.
    #[error("entity {entity} still has position after removal from world")]
    StillHasPosition { entity: EntityId, nonce: u64 },

    /// Entity still in occupancy map after removal.
    #[error("entity {entity} still in occupancy map after removal from world")]
    StillInOccupancy { entity: EntityId, nonce: u64 },
}

impl RemoveFromWorldError {
    /// Creates a NotSystemActor error with context.
    pub fn not_system_actor(nonce: u64) -> Self {
        Self::NotSystemActor {
            context: ErrorContext::new(nonce)
                .with_message("system action executed by non-system actor"),
        }
    }

    /// Creates an EntityNotFound error with context.
    pub fn entity_not_found(entity: EntityId, nonce: u64) -> Self {
        Self::EntityNotFound {
            entity,
            context: ErrorContext::new(nonce)
                .with_actor(entity)
                .with_message("entity not found"),
        }
    }
}

impl GameError for RemoveFromWorldError {
    fn severity(&self) -> ErrorSeverity {
        match self {
            Self::NotSystemActor { .. } => ErrorSeverity::Validation,
            Self::EntityNotFound { .. } => ErrorSeverity::Validation,
            Self::StillHasPosition { .. } => ErrorSeverity::Internal,
            Self::StillInOccupancy { .. } => ErrorSeverity::Internal,
        }
    }

    fn context(&self) -> Option<&ErrorContext> {
        match self {
            Self::NotSystemActor { context } => Some(context),
            Self::EntityNotFound { context, .. } => Some(context),
            Self::StillHasPosition { .. } => None,
            Self::StillInOccupancy { .. } => None,
        }
    }

    fn error_code(&self) -> &'static str {
        match self {
            Self::NotSystemActor { .. } => "REMOVE_FROM_WORLD_NOT_SYSTEM_ACTOR",
            Self::EntityNotFound { .. } => "REMOVE_FROM_WORLD_ENTITY_NOT_FOUND",
            Self::StillHasPosition { .. } => "REMOVE_FROM_WORLD_STILL_HAS_POSITION",
            Self::StillInOccupancy { .. } => "REMOVE_FROM_WORLD_STILL_IN_OCCUPANCY",
        }
    }
}
