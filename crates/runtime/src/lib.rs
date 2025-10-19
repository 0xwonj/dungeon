//! Runtime orchestration for the deterministic game simulation.
//!
//! This crate wires together the action provider abstraction, oracle access,
//! repositories, and worker tasks into a cohesive runtime API. Consumers embed
//! [`Runtime`] to drive turns, subscribe to events, and interact with the world
//! through [`RuntimeHandle`].
//!
//! Modules are organized by responsibility:
//! - [`runtime`] hosts the orchestrator and builder
//! - [`api`] exposes the types downstream clients interact with
//! - [`events`] provides topic-based event bus for flexible event routing
//! - [`workers`] keeps background tasks internal to the crate
//! - [`hooks`] provides post-execution hook system for runtime orchestration
//! - [`oracle`] and [`repository`] provide data adapters reused by other crates
//! - [`types`] provides common type aliases for semantic clarity
pub mod api;
pub mod events;
pub mod hooks;
pub mod oracle;
pub mod repository;
pub mod runtime;
pub mod types;

mod utils;
mod workers;

pub use api::{
    ActionProvider, AiKind, InteractiveKind, ProviderKind, ProviderRegistry, Result, RuntimeError,
    RuntimeHandle, WaitActionProvider,
};
pub use events::{Event, EventBus, GameStateEvent, ProofEvent, Topic};
pub use hooks::{
    ActionCostHook, ActivationHook, HookContext, HookCriticality, HookRegistry, PostExecutionHook,
};
pub use oracle::{
    ConfigOracleImpl, ItemOracleImpl, MapOracleImpl, NpcOracleImpl, OracleManager, TablesOracleImpl,
};
pub use repository::{
    ActionLogEntry, ActionLogReader, ActionLogWriter, Checkpoint, CheckpointRepository,
    EventRepository, FileActionLog, FileCheckpointRepository, FileEventLog,
    FileProofIndexRepository, FileStateRepository, InMemoryActionLogReader, InMemoryStateRepo,
    MmapActionLogReader, ProofEntry, ProofIndex, ProofIndexRepository, RepositoryError,
    StateReference, StateRepository,
};
pub use runtime::{PersistenceSettings, ProvingSettings, Runtime, RuntimeBuilder, RuntimeConfig};
pub use types::{ByteOffset, DurationMs, Nonce, ProofSize, SessionId, StateHash, Timestamp};
pub use workers::{CheckpointStrategy, PersistenceConfig, ProofMetrics};
