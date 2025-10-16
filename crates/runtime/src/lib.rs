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
pub mod api;
pub mod events;
pub mod hooks;
pub mod oracle;
pub mod repository;
pub mod runtime;

mod workers;

pub use api::{
    ActionProvider, ProviderKind, Result, RuntimeError, RuntimeHandle, WaitActionProvider,
};
pub use events::{ActionFlags, Event, EventBus, GameStateEvent, ProofEvent, Topic, TurnEvent};
pub use hooks::{
    ActionCostHook, ActivationHook, HookContext, HookCriticality, HookRegistry, PostExecutionHook,
};
pub use oracle::{
    ConfigOracleImpl, ItemOracleImpl, MapOracleImpl, NpcOracleImpl, OracleManager, TablesOracleImpl,
};
pub use repository::{
    ActionLogEntry, ActionRepository, Checkpoint, CheckpointRepository, EventReference,
    EventRepository, FileActionLog, FileCheckpointRepository, FileEventLog, FileStateRepository,
    InMemoryStateRepo, ProofReference, RepositoryError, SnapshotService, StateReference,
    StateRepository,
};
pub use runtime::{Runtime, RuntimeBuilder, RuntimeConfig};
pub use workers::ProofMetrics;
