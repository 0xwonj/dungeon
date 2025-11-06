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
//! - [`providers`] contains concrete action provider implementations
//! - [`events`] provides topic-based event bus for flexible event routing
//! - [`workers`] keeps background tasks internal to the crate
//! - [`handlers`] provides event-based reactive action generation
//! - [`oracle`] and [`repository`] provide data adapters reused by other crates
//! - [`scenario`] provides entity placement and game initialization
//! - [`types`] provides common type aliases for semantic clarity
pub mod api;
pub mod events;
pub mod handlers;
pub mod oracle;
pub mod providers;
pub mod repository;
pub mod runtime;
pub mod scenario;
pub mod types;

mod utils;
mod workers;

pub use api::{
    ActionProvider, AiKind, InteractiveKind, ProviderKind, ProviderRegistry, Result, RuntimeError,
    RuntimeHandle,
};
pub use events::{
    Event, EventBus, GameEvent, GameStateEvent, HealthThreshold, ProofEvent, Topic, extract_events,
};
pub use handlers::{
    ActionCostHandler, ActivationHandler, DeathHandler, EventContext, HandlerCriticality,
};
pub use oracle::{
    ActorOracleImpl, AiConfig, ConfigOracleImpl, ItemOracleImpl, MapOracleImpl, OracleManager,
    TablesOracleImpl,
};
pub use providers::ai::{AiContext, GoalBasedAiProvider};
pub use providers::{SystemActionHandler, SystemActionProvider};
pub use repository::{
    ActionLogEntry, ActionLogReader, ActionLogWriter, Checkpoint, CheckpointRepository,
    EventRepository, FileActionLog, FileCheckpointRepository, FileEventLog,
    FileProofIndexRepository, FileStateRepository, InMemoryActionLogReader, InMemoryStateRepo,
    MmapActionLogReader, ProofEntry, ProofIndex, ProofIndexRepository, RepositoryError,
    StateReference, StateRepository,
};
pub use runtime::{PersistenceSettings, ProvingSettings, Runtime, RuntimeBuilder, RuntimeConfig};
pub use scenario::{EntityKind, EntityPlacement, Scenario};
pub use types::{ByteOffset, DurationMs, Nonce, ProofSize, SessionId, StateHash, Timestamp};
pub use workers::{CheckpointStrategy, PersistenceConfig, ProofMetrics};
