//! Runtime orchestration for the deterministic game simulation.
//!
//! This crate wires together the action provider abstraction, oracle access,
//! repositories, and worker tasks into a cohesive runtime API. Consumers embed
//! [`Runtime`] to drive turns, subscribe to [`GameEvent`] notifications, and
//! interact with the world through [`RuntimeHandle`].
//!
//! Modules are organized by responsibility:
//! - [`runtime`] hosts the orchestrator and builder
//! - [`api`] exposes the types downstream clients interact with
//! - [`workers`] keeps background tasks internal to the crate
//! - [`oracle`] and [`repository`] provide data adapters reused by other crates
pub mod api;
pub mod oracle;
pub mod repository;
pub mod runtime;

mod workers;

pub use api::{
    ActionProvider, GameEvent, ProviderKind, Result, RuntimeError, RuntimeHandle,
    WaitActionProvider,
};
pub use oracle::{
    ConfigOracleImpl, ItemOracleImpl, MapOracleImpl, NpcOracleImpl, OracleManager,
    TablesOracleImpl,
};
pub use repository::{InMemoryStateRepo, RepositoryError, StateRepository};
pub use runtime::{Runtime, RuntimeBuilder, RuntimeConfig};
