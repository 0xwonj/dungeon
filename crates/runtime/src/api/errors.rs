//! Unified error types surfaced by the runtime API.
//!
//! Wraps failures from worker coordination, repositories, and action providers
//! so clients can bubble them up with consistent context.
use std::fmt;

use thiserror::Error;
use tokio::sync::oneshot;

pub use crate::repository::RepositoryError;

pub type Result<T> = std::result::Result<T, RuntimeError>;

#[derive(Debug, Error)]
pub enum RuntimeError {
    #[error("no active entities available for turn scheduling")]
    NoActiveEntities,

    #[error("{kind} action provider not set")]
    ProviderNotSet { kind: ProviderKind },

    #[error("action provider channel closed")]
    ActionProviderChannelClosed,

    #[error("simulation worker command channel closed")]
    CommandChannelClosed,

    #[error("simulation worker reply channel closed")]
    ReplyChannelClosed(#[source] oneshot::error::RecvError),

    #[error("simulation worker join failed")]
    WorkerJoin(#[source] tokio::task::JoinError),

    #[error(transparent)]
    Repository(#[from] RepositoryError),

    #[error("runtime requires oracles to be configured before building")]
    MissingOracles,

    #[error("invalid configuration: {0}")]
    InvalidConfig(String),

    #[error("invalid entity ID: {0:?}")]
    InvalidEntityId(game_core::EntityId),

    #[error("missing action provider: {0}")]
    MissingProvider(ProviderKind),

    #[error("provider registry lock poisoned")]
    LockPoisoned,
}

/// Provider category distinguishing interactive inputs from automated AI.
///
/// This nested enum design provides clear separation between:
/// - Interactive sources (human players, network clients, replays)
/// - Automated AI decision makers (combat AI, passive behavior, etc.)
/// - Custom extensibility slots for user-defined providers
#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub enum ProviderKind {
    /// Interactive input sources (human players, network clients, etc.)
    Interactive(InteractiveKind),

    /// Automated AI decision makers
    Ai(AiKind),

    /// Custom provider types (extensibility slot)
    Custom(u32),
}

/// Interactive input provider types.
#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub enum InteractiveKind {
    /// Local CLI keyboard input
    CliInput,

    /// Network/remote player input
    NetworkInput,

    /// Replayed actions from file/log
    Replay,
}

/// AI provider types for automated decision making.
#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub enum AiKind {
    /// Simple wait-only AI (no-op, default for unmapped entities)
    Wait,

    /// Aggressive combat AI
    Aggressive,

    /// Defensive/passive AI
    Passive,

    /// Scripted behavior AI
    Scripted,

    /// Goal-based AI (recommended: Goal → Evaluate all candidates → Select best)
    GoalBased,

    /// Utility-based AI (legacy: Intent → Tactic → Action)
    Utility,
}

impl fmt::Display for ProviderKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Interactive(kind) => write!(f, "interactive/{}", kind),
            Self::Ai(kind) => write!(f, "ai/{}", kind),
            Self::Custom(id) => write!(f, "custom/{}", id),
        }
    }
}

impl fmt::Display for InteractiveKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let s = match self {
            Self::CliInput => "cli",
            Self::NetworkInput => "network",
            Self::Replay => "replay",
        };
        write!(f, "{}", s)
    }
}

impl fmt::Display for AiKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let s = match self {
            Self::Wait => "wait",
            Self::Aggressive => "aggressive",
            Self::Passive => "passive",
            Self::Scripted => "scripted",
            Self::GoalBased => "goal_based",
            Self::Utility => "utility",
        };
        write!(f, "{}", s)
    }
}
