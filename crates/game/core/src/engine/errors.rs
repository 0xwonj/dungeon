//! Error types for action execution pipeline.

use crate::action::{
    ActionCostAction, ActionError, ActionTransition, ActivationAction, PrepareTurnAction,
    RemoveFromActiveAction,
};
use crate::error::{ErrorContext, ErrorSeverity, GameError};

/// Identifies which stage of the transition pipeline produced an error.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum TransitionPhase {
    PreValidate,
    Apply,
    PostValidate,
}

impl TransitionPhase {
    pub fn as_str(&self) -> &'static str {
        match self {
            TransitionPhase::PreValidate => "pre_validate",
            TransitionPhase::Apply => "apply",
            TransitionPhase::PostValidate => "post_validate",
        }
    }
}

/// Associates a transition phase with the underlying error.
#[derive(Clone, Debug)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct TransitionPhaseError<E> {
    pub phase: TransitionPhase,
    pub error: E,
}

impl<E> TransitionPhaseError<E> {
    /// Creates a new transition phase error.
    pub const fn new(phase: TransitionPhase, error: E) -> Self {
        Self { phase, error }
    }

    /// Returns the phase where this error occurred.
    pub const fn phase(&self) -> TransitionPhase {
        self.phase
    }

    /// Returns a reference to the underlying error.
    pub const fn inner(&self) -> &E {
        &self.error
    }

    /// Consumes self and returns the underlying error.
    pub fn into_inner(self) -> E {
        self.error
    }

    /// Consumes self and returns both phase and error.
    pub fn into_parts(self) -> (TransitionPhase, E) {
        (self.phase, self.error)
    }
}

impl<E: core::fmt::Display> core::fmt::Display for TransitionPhaseError<E> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "{} failed: {}", self.phase.as_str(), self.error)
    }
}

// std::error::Error is only available with std feature
#[cfg(feature = "std")]
impl<E: core::fmt::Display + core::fmt::Debug> std::error::Error for TransitionPhaseError<E> {}

// GameError delegation: when the inner error implements GameError,
// TransitionPhaseError automatically implements it by delegating to the inner error.
impl<E> GameError for TransitionPhaseError<E>
where
    E: GameError,
{
    fn severity(&self) -> ErrorSeverity {
        self.error.severity()
    }

    fn context(&self) -> Option<&ErrorContext> {
        self.error.context()
    }

    fn error_code(&self) -> &'static str {
        // Preserve the inner error code (not "TRANSITION_PHASE")
        // This makes error metrics and logging more meaningful
        self.error.error_code()
    }
}

/// Errors surfaced while executing an action through the game engine.
#[derive(Clone, Debug, thiserror::Error)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum ExecuteError {
    #[error("character action failed: {0}")]
    Character(TransitionPhaseError<ActionError>),

    #[error("prepare turn action failed: {0}")]
    #[cfg_attr(feature = "serde", serde(skip))]
    PrepareTurn(TransitionPhaseError<<PrepareTurnAction as ActionTransition>::Error>),

    #[error("action cost action failed: {0}")]
    #[cfg_attr(feature = "serde", serde(skip))]
    ActionCost(TransitionPhaseError<<ActionCostAction as ActionTransition>::Error>),

    #[error("activation action failed: {0}")]
    #[cfg_attr(feature = "serde", serde(skip))]
    Activation(TransitionPhaseError<<ActivationAction as ActionTransition>::Error>),

    #[error("remove from active action failed: {0}")]
    #[cfg_attr(feature = "serde", serde(skip))]
    RemoveFromActive(TransitionPhaseError<<RemoveFromActiveAction as ActionTransition>::Error>),

    #[error("hook chain too deep: hook '{hook_name}' reached depth {depth}")]
    HookChainTooDeep {
        hook_name: String,
        depth: usize,
        #[cfg_attr(feature = "serde", serde(skip))]
        context: ErrorContext,
    },

    #[error("invalid actor: system action must be executed by SYSTEM (got {actor})")]
    SystemActionNotFromSystem {
        actor: crate::state::EntityId,
        #[cfg_attr(feature = "serde", serde(skip))]
        context: ErrorContext,
    },

    #[error(
        "invalid actor: action actor {actor} does not match current turn actor {current_actor}"
    )]
    ActorNotCurrent {
        actor: crate::state::EntityId,
        current_actor: crate::state::EntityId,
        #[cfg_attr(feature = "serde", serde(skip))]
        context: ErrorContext,
    },
}

impl ExecuteError {
    /// Creates a HookChainTooDeep error with context.
    pub fn hook_chain_too_deep(hook_name: String, depth: usize, nonce: u64) -> Self {
        Self::HookChainTooDeep {
            hook_name,
            depth,
            context: ErrorContext::new(nonce).with_message("hook chain exceeded maximum depth"),
        }
    }

    /// Creates a SystemActionNotFromSystem error with context.
    pub fn system_action_not_from_system(actor: crate::state::EntityId, nonce: u64) -> Self {
        Self::SystemActionNotFromSystem {
            actor,
            context: ErrorContext::new(nonce)
                .with_actor(actor)
                .with_message("system action must be from SYSTEM actor"),
        }
    }

    /// Creates an ActorNotCurrent error with context.
    pub fn actor_not_current(
        actor: crate::state::EntityId,
        current_actor: crate::state::EntityId,
        nonce: u64,
    ) -> Self {
        Self::ActorNotCurrent {
            actor,
            current_actor,
            context: ErrorContext::new(nonce)
                .with_actor(actor)
                .with_message("action actor mismatch"),
        }
    }
}

impl ExecuteError {
    /// Returns the phase where this error occurred (if from action transition).
    ///
    /// Returns `None` for system-level errors that don't go through the transition pipeline.
    pub fn phase(&self) -> Option<TransitionPhase> {
        match self {
            Self::Character(e) => Some(e.phase),
            Self::PrepareTurn(e) => Some(e.phase),
            Self::ActionCost(e) => Some(e.phase),
            Self::Activation(e) => Some(e.phase),
            Self::RemoveFromActive(e) => Some(e.phase),
            Self::HookChainTooDeep { .. }
            | Self::SystemActionNotFromSystem { .. }
            | Self::ActorNotCurrent { .. } => None,
        }
    }

    /// Returns true if this error occurred during pre-validation.
    ///
    /// Pre-validation errors typically indicate invalid user input or
    /// precondition failures (e.g., destination blocked, target out of range).
    /// These are expected errors that should be communicated to the player.
    pub fn is_validation_failure(&self) -> bool {
        self.phase() == Some(TransitionPhase::PreValidate)
    }

    /// Returns true if this error occurred during the apply phase.
    ///
    /// Apply phase errors may indicate state mutation failures or
    /// runtime issues during action execution.
    pub fn is_apply_failure(&self) -> bool {
        self.phase() == Some(TransitionPhase::Apply)
    }

    /// Returns true if this error occurred during post-validation.
    ///
    /// Post-validation errors typically indicate internal state consistency
    /// violations and should be treated as bugs requiring investigation.
    pub fn is_postcondition_failure(&self) -> bool {
        self.phase() == Some(TransitionPhase::PostValidate)
    }

    /// Returns true if this is a system-level error (not from action transition).
    ///
    /// System errors include hook failures, actor validation failures, etc.
    pub fn is_system_error(&self) -> bool {
        matches!(
            self,
            Self::HookChainTooDeep { .. }
                | Self::SystemActionNotFromSystem { .. }
                | Self::ActorNotCurrent { .. }
        )
    }

    /// Returns a string representation of the error message.
    ///
    /// This is a convenience method for logging and display purposes.
    pub fn message(&self) -> String {
        format!("{}", self)
    }
}

impl GameError for ExecuteError {
    fn severity(&self) -> ErrorSeverity {
        match self {
            Self::Character(_) => ErrorSeverity::Validation,
            Self::PrepareTurn(e) => e.severity(),
            Self::ActionCost(e) => e.severity(),
            Self::Activation(e) => e.severity(),
            Self::RemoveFromActive(e) => e.severity(),
            Self::HookChainTooDeep { .. } => ErrorSeverity::Fatal,
            Self::SystemActionNotFromSystem { .. } => ErrorSeverity::Validation,
            Self::ActorNotCurrent { .. } => ErrorSeverity::Validation,
        }
    }

    fn context(&self) -> Option<&ErrorContext> {
        match self {
            Self::Character(_) => None,
            Self::PrepareTurn(e) => e.context(),
            Self::ActionCost(e) => e.context(),
            Self::Activation(e) => e.context(),
            Self::RemoveFromActive(e) => e.context(),
            Self::HookChainTooDeep { context, .. } => Some(context),
            Self::SystemActionNotFromSystem { context, .. } => Some(context),
            Self::ActorNotCurrent { context, .. } => Some(context),
        }
    }

    fn error_code(&self) -> &'static str {
        match self {
            Self::Character(_) => "EXECUTE_CHARACTER_ACTION",
            Self::PrepareTurn(e) => e.error_code(),
            Self::ActionCost(e) => e.error_code(),
            Self::Activation(e) => e.error_code(),
            Self::RemoveFromActive(e) => e.error_code(),
            Self::HookChainTooDeep { .. } => "EXECUTE_HOOK_CHAIN_TOO_DEEP",
            Self::SystemActionNotFromSystem { .. } => "EXECUTE_SYSTEM_ACTION_INVALID",
            Self::ActorNotCurrent { .. } => "EXECUTE_ACTOR_NOT_CURRENT",
        }
    }
}
