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

    #[error("failed to initialize game state from oracles")]
    InitialState(#[source] game_core::state::InitializationError),

    #[error("action actor {provided:?} does not match current entity {expected:?}")]
    InvalidActionActor {
        expected: game_core::EntityId,
        provided: game_core::EntityId,
    },
}

#[derive(Debug, Copy, Clone)]
pub enum ProviderKind {
    Player,
    Npc,
}

impl fmt::Display for ProviderKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let label = match self {
            ProviderKind::Player => "player",
            ProviderKind::Npc => "npc",
        };
        write!(f, "{}", label)
    }
}
