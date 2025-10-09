//! Public runtime API surface.
//!
//! This module gathers the types exposed to consumers of the runtime crate so
//! other layers can stay focused on orchestration, workers, or infrastructure.

pub mod errors;
pub mod events;
pub mod handle;
pub mod providers;

pub use errors::{ProviderKind, Result, RuntimeError};
pub use events::GameEvent;
pub use handle::RuntimeHandle;
pub use providers::{ActionProvider, WaitActionProvider};
