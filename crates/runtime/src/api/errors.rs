//! Unified error types surfaced by the runtime API.
//!
//! Wraps failures from worker coordination, repositories, and action providers
//! so clients can bubble them up with consistent context.
use thiserror::Error;
use tokio::sync::oneshot;

pub use crate::repository::RepositoryError;
pub use game_core::{AiKind, InteractiveKind, ProviderKind};

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

    #[error("persistence is not enabled")]
    PersistenceNotEnabled,

    #[error("persistence error: {0}")]
    PersistenceError(String),

    #[error("blockchain integration is not enabled")]
    BlockchainNotEnabled,
}
